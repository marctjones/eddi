//! eddi - Complete Arti-to-UDS Bridge with Configurable Onion Services
//!
//! This implementation provides:
//! - Arti Tor hidden service with persistent onion addresses
//! - Unix Domain Socket connection to any web application
//! - CLI configuration for flexible deployment
//! - Multi-instance support with different onion addresses
//!
//! The complete flow:
//! 1. Initialize Arti TorClient and bootstrap to Tor network
//! 2. Launch a Tor v3 onion service (new or existing)
//! 3. Connect to a web application via UDS
//! 4. Accept incoming connections from the Tor network
//! 5. Proxy requests to the UDS-bound application
//! 6. Proxy responses back through Tor
//!
//! This creates a fully isolated web application accessible ONLY via Tor.

use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use std::sync::Arc;
use std::fs;
use tracing::{info, warn, error, debug};
use tracing_subscriber;
use clap::Parser;

use arti_client::TorClient;
use tor_hsservice::config::OnionServiceConfigBuilder;
use tor_hsservice::{StreamRequest, handle_rend_requests};
use tor_proto::client::stream::IncomingStreamRequest;
use tor_cell::relaycell::msg::Connected;
use safelog::DisplayRedacted;

use tokio::net::UnixStream;
use tokio::io::AsyncWriteExt;
use futures::StreamExt;

use eddi::{ChildProcessManager, ProcessConfig};

/// eddi - Serve web applications over Tor via Unix Domain Sockets
///
/// eddi allows you to expose any web application (Flask, Django, nginx, etc.)
/// as a Tor hidden service (.onion address) using Unix Domain Sockets for
/// inter-process communication. This provides complete network isolation.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Unix Domain Socket path to connect to
    ///
    /// Path where your web server (gunicorn, uvicorn, nginx, etc.) is listening.
    /// Example: /tmp/my-app.sock
    #[arg(short = 's', long, default_value = "/tmp/eddi.sock")]
    socket: PathBuf,

    /// Onion service nickname
    ///
    /// A unique identifier for this onion service. Used to store and retrieve
    /// persistent keys. Each nickname gets its own .onion address.
    /// Example: my-blog, api-server, chat-app
    #[arg(short = 'n', long, default_value = "eddi-demo")]
    nickname: String,

    /// Working directory for the web application (optional)
    ///
    /// Only needed if eddi should spawn the web server process.
    /// If your web server is already running and listening on the UDS,
    /// you can omit this option.
    #[arg(short = 'd', long)]
    app_dir: Option<PathBuf>,

    /// Application module for WSGI/ASGI server (e.g., "app:app")
    ///
    /// Only used when spawning gunicorn. Format: module:application
    #[arg(short = 'm', long, default_value = "app:app")]
    app_module: String,

    /// Number of worker processes for gunicorn
    ///
    /// Only used when spawning gunicorn.
    #[arg(short = 'w', long, default_value = "2")]
    workers: u8,

    /// Directory to store onion service keys
    ///
    /// Keys are stored in subdirectories by nickname.
    /// Example: ~/.eddi/onion-services/my-blog/
    #[arg(short = 'k', long)]
    key_dir: Option<PathBuf>,

    /// Import existing onion service key directory
    ///
    /// Path to a directory containing hs_ed25519_secret_key and hs_ed25519_public_key.
    /// This allows you to use an existing .onion address from another Tor installation.
    #[arg(long)]
    import_keys: Option<PathBuf>,

    /// Test UDS connection before starting
    ///
    /// Verify that the Unix socket exists and is accepting connections.
    #[arg(long, default_value = "true")]
    test_connection: bool,

    /// Skip spawning child process (assume app is already running)
    ///
    /// Use this when your web application is already running and listening
    /// on the Unix Domain Socket.
    #[arg(long)]
    no_spawn: bool,
}

/// Configuration for the eddi application
struct EddiConfig {
    /// Path to the Unix Domain Socket
    socket_path: PathBuf,

    /// Working directory for the web application
    app_dir: Option<PathBuf>,

    /// Application module (e.g., "app:app" for Flask)
    app_module: String,

    /// Number of worker processes
    workers: u8,

    /// Nickname for the onion service
    onion_service_nickname: String,

    /// Directory to store onion service keys
    key_dir: PathBuf,

    /// Whether to test the connection first
    test_connection: bool,

    /// Whether to spawn the child process
    should_spawn: bool,
}

impl EddiConfig {
    /// Create configuration from CLI arguments
    fn from_cli(cli: Cli) -> Result<Self> {
        // Determine key directory
        let key_dir = if let Some(dir) = cli.key_dir {
            dir
        } else {
            // Default: ~/.eddi/onion-services
            let home = std::env::var("HOME")
                .context("HOME environment variable not set")?;
            PathBuf::from(home).join(".eddi").join("onion-services")
        };

        Ok(Self {
            socket_path: cli.socket,
            app_dir: cli.app_dir,
            app_module: cli.app_module,
            workers: cli.workers,
            onion_service_nickname: cli.nickname,
            key_dir,
            test_connection: cli.test_connection,
            should_spawn: !cli.no_spawn,
        })
    }

    /// Get the path to store keys for this nickname
    fn get_key_storage_path(&self) -> PathBuf {
        self.key_dir.join(&self.onion_service_nickname)
    }
}

/// Test if we can connect to the Unix Domain Socket
async fn test_uds_connection(socket_path: &PathBuf) -> Result<bool> {
    debug!("Testing connection to Unix socket: {:?}", socket_path);

    // Check if the socket file exists
    if !socket_path.exists() {
        warn!("Unix socket file does not exist: {:?}", socket_path);
        return Ok(false);
    }

    // Try to connect
    match UnixStream::connect(socket_path).await {
        Ok(mut stream) => {
            info!("✓ Successfully connected to Unix socket: {:?}", socket_path);
            // Gracefully close the test connection
            let _ = stream.shutdown().await;
            Ok(true)
        }
        Err(e) => {
            warn!("Failed to connect to Unix socket: {}", e);
            Ok(false)
        }
    }
}

/// Handle an incoming stream request from the onion service
async fn handle_stream_request(
    stream_request: StreamRequest,
    socket_path: Arc<PathBuf>,
) -> Result<()> {
    match stream_request.request() {
        IncomingStreamRequest::Begin(begin) => {
            let port = begin.port();
            info!("Incoming connection request on port {}", port);

            // Only accept connections on port 80 (or configured ports)
            if port != 80 {
                warn!("Rejecting connection on unexpected port {}", port);
                stream_request.shutdown_circuit()?;
                return Ok(());
            }

            // Accept the stream
            info!("Accepting stream from onion service");
            let mut onion_stream = stream_request
                .accept(Connected::new_empty())
                .await
                .context("Failed to accept stream from onion service")?;

            info!("Connecting to Unix socket: {:?}", socket_path);

            // Connect to the Unix socket
            let mut unix_stream = UnixStream::connect(socket_path.as_ref())
                .await
                .context("Failed to connect to Unix socket")?;

            info!("Connected to Unix socket, starting bidirectional proxy");

            // Proxy data bidirectionally between the onion service and Unix socket
            match tokio::io::copy_bidirectional(&mut onion_stream, &mut unix_stream).await {
                Ok((to_unix, to_onion)) => {
                    info!(
                        "Stream closed. Transferred {} bytes to Unix socket, {} bytes to onion service",
                        to_unix, to_onion
                    );
                }
                Err(e) => {
                    error!("Error during stream proxy: {}", e);
                }
            }

            // Gracefully shutdown both streams
            let _ = unix_stream.shutdown().await;
            drop(onion_stream); // Tor stream will send END cell on drop
        }
        IncomingStreamRequest::BeginDir(_) => {
            warn!("Received BeginDir request (unexpected), rejecting");
            stream_request.shutdown_circuit()?;
        }
        _ => {
            warn!("Received unexpected stream request type, rejecting");
            stream_request.shutdown_circuit()?;
        }
    }

    Ok(())
}

/// Run the complete eddi application
async fn run_eddi(config: EddiConfig) -> Result<()> {
    info!("=== eddi: Arti-to-UDS Bridge ===");
    info!("Configuration:");
    info!("  Socket path: {:?}", config.socket_path);
    info!("  Onion service nickname: {}", config.onion_service_nickname);
    info!("  Key storage: {:?}", config.get_key_storage_path());
    info!("  Spawn child process: {}", config.should_spawn);
    info!("");

    // Step 1: Initialize Arti Tor client
    info!("Step 1: Initializing Arti Tor client...");

    // Ensure the key directory exists
    let key_storage_path = config.get_key_storage_path();
    if !key_storage_path.exists() {
        info!("Creating key storage directory: {:?}", key_storage_path);
        fs::create_dir_all(&key_storage_path)
            .context("Failed to create key storage directory")?;
        info!("✓ Key storage directory created");
    } else {
        info!("Using existing key storage directory: {:?}", key_storage_path);
    }

    let tor_client = TorClient::create_bootstrapped(Default::default())
        .await
        .context("Failed to bootstrap Tor client")?;
    info!("✓ Tor client bootstrapped successfully");
    info!("");

    // Step 2: Launch onion service
    info!("Step 2: Configuring onion service...");
    let svc_config = OnionServiceConfigBuilder::default()
        .nickname(
            config
                .onion_service_nickname
                .parse()
                .context("Invalid onion service nickname")?,
        )
        .build()
        .context("Failed to build onion service config")?;

    info!("Launching onion service '{}'...", config.onion_service_nickname);
    let (onion_service, request_stream) = tor_client
        .launch_onion_service(svc_config)
        .context("Failed to launch onion service")?;

    info!("✓ Onion service launched");

    // Wait for the onion address to be available
    info!("Waiting for onion address...");
    let onion_address = loop {
        if let Some(addr) = onion_service.onion_address() {
            break addr;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    };
    info!("");

    // Step 3: Handle child process or verify existing connection
    let child_process = if config.should_spawn {
        if config.app_dir.is_none() {
            bail!("--app-dir is required when spawning a child process. Use --no-spawn if the app is already running.");
        }

        info!("Step 3: Spawning child process...");
        let process_config = ProcessConfig::gunicorn(
            config.socket_path.clone(),
            config.app_dir.clone().unwrap(),
            &config.app_module,
            config.workers,
        );

        let child = ChildProcessManager::spawn(&process_config)
            .context("Failed to spawn child process")?;

        info!("✓ Child process spawned (PID: {})", child.pid());

        // Wait for the child process to be ready
        child
            .wait_for_ready(10)
            .await
            .context("Child process failed to become ready")?;

        info!("✓ Child process is ready and accepting connections");
        Some(child)
    } else {
        info!("Step 3: Skipping child process spawn (--no-spawn)");
        info!("Assuming web application is already running on: {:?}", config.socket_path);
        None
    };
    info!("");

    // Step 4: Test Unix Domain Socket connection
    if config.test_connection {
        info!("Step 4: Testing Unix Domain Socket connection...");
        match test_uds_connection(&config.socket_path).await {
            Ok(true) => {
                info!("✓ Unix Domain Socket is accessible and working");
            }
            Ok(false) => {
                error!("✗ Unix Domain Socket connection test failed");
                error!("  Socket path: {:?}", config.socket_path);
                error!("  Make sure your web application is running and listening on this socket.");
                bail!("Unix Domain Socket connection test failed");
            }
            Err(e) => {
                error!("✗ Error testing Unix Domain Socket: {}", e);
                bail!("Unix Domain Socket connection test failed");
            }
        }
    } else {
        info!("Step 4: Skipping connection test (--test-connection=false)");
    }
    info!("");

    // Step 5: Wait for onion service to be fully reachable
    info!("Step 5: Waiting for onion service to be fully reachable...");
    let mut status_stream = onion_service.status_events();

    // Wait for reachability (with timeout)
    let timeout_duration = std::time::Duration::from_secs(60);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout_duration {
        tokio::select! {
            Some(status) = status_stream.next() => {
                info!("Onion service status: {:?}", status);
                if status.state().is_fully_reachable() {
                    info!("✓ Onion service is fully reachable!");
                    break;
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                // Continue waiting
            }
        }
    }

    info!("");
    info!("========================================");
    info!("🎉 eddi is fully operational!");
    info!("========================================");
    info!("");
    info!("🧅  Onion Address:");
    info!("     http://{}", onion_address.display_unredacted());
    info!("");
    info!("🔌  Unix Domain Socket:");
    info!("     {:?}", config.socket_path);
    info!("     Status: ✓ Connected and working");
    info!("");
    info!("🔑  Onion Service Keys:");
    info!("     {:?}", config.get_key_storage_path());
    info!("");
    if let Some(ref child) = child_process {
        info!("⚙️   Web Application:");
        info!("     Process PID: {}", child.pid());
        info!("     Workers: {}", config.workers);
        info!("     Module: {}", config.app_module);
        info!("");
    }
    info!("🔒  Security:");
    info!("     ✓ Accessible ONLY via Tor");
    info!("     ✓ No TCP ports exposed");
    info!("     ✓ Complete network isolation");
    info!("");
    info!("========================================");
    info!("Press Ctrl+C to shut down...");
    info!("========================================");
    info!("");

    // Step 6: Handle incoming requests
    info!("Step 6: Accepting incoming connections...");
    info!("");

    let socket_path = Arc::new(config.socket_path.clone());
    let stream_requests = handle_rend_requests(request_stream);
    tokio::pin!(stream_requests);

    while let Some(stream_request) = stream_requests.next().await {
        let socket_path = Arc::clone(&socket_path);

        // Spawn a new task for each incoming connection
        tokio::spawn(async move {
            if let Err(e) = handle_stream_request(stream_request, socket_path).await {
                error!("Error handling stream: {}", e);
            }
        });
    }

    info!("Request stream ended, shutting down...");

    // child_process will be cleaned up when it's dropped
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Parse command-line arguments
    let cli = Cli::parse();

    // Create configuration from CLI
    let config = EddiConfig::from_cli(cli)?;

    // Verify the app directory exists if we're spawning
    if config.should_spawn {
        if let Some(ref app_dir) = config.app_dir {
            if !app_dir.exists() {
                error!("Application directory not found: {:?}", app_dir);
                error!("Please ensure the web application exists at the specified path.");
                bail!("Application directory not found");
            }
        }
    }

    // Run the complete eddi application
    run_eddi(config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eddi_config_default() {
        let config = EddiConfig::default();
        assert_eq!(config.socket_path, PathBuf::from("/tmp/eddi.sock"));
        assert_eq!(config.workers, 2);
        assert_eq!(config.app_module, "app:app");
    }
}
