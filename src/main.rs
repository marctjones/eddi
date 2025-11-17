//! eddi - Task 4: Complete Arti-to-UDS Bridge
//!
//! This is the final implementation that combines:
//! - Task 2: Arti Tor hidden service
//! - Task 3: Unix Domain Socket and child process management
//!
//! The complete flow:
//! 1. Initialize Arti TorClient and bootstrap to Tor network
//! 2. Launch a Tor v3 onion service
//! 3. Spawn the web application (gunicorn) bound to a UDS
//! 4. Accept incoming connections from the Tor network
//! 5. Proxy requests to the UDS-bound application
//! 6. Proxy responses back through Tor
//!
//! This creates a fully isolated web application accessible ONLY via Tor.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn, error};
use tracing_subscriber;

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

/// Configuration for the eddi application
struct EddiConfig {
    /// Path to the Unix Domain Socket
    socket_path: PathBuf,

    /// Working directory for the web application
    app_dir: PathBuf,

    /// Application module (e.g., "app:app" for Flask)
    app_module: String,

    /// Number of worker processes
    workers: u8,

    /// Nickname for the onion service
    onion_service_nickname: String,

    /// Ports to expose on the onion service
    #[allow(dead_code)]
    onion_ports: Vec<u16>,
}

impl Default for EddiConfig {
    fn default() -> Self {
        Self {
            socket_path: PathBuf::from("/tmp/eddi.sock"),
            app_dir: PathBuf::from("test-apps/flask-demo"),
            app_module: "app:app".to_string(),
            workers: 2,
            onion_service_nickname: "eddi-demo".to_string(),
            onion_ports: vec![80],
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
    info!("Starting complete integration...");

    // Step 1: Initialize Arti Tor client
    info!("Step 1: Initializing Arti Tor client...");
    let tor_client = TorClient::create_bootstrapped(Default::default())
        .await
        .context("Failed to bootstrap Tor client")?;
    info!("✓ Tor client bootstrapped successfully");

    // Step 2: Launch onion service
    info!("Step 2: Launching onion service...");
    let svc_config = OnionServiceConfigBuilder::default()
        .nickname(
            config
                .onion_service_nickname
                .parse()
                .context("Invalid onion service nickname")?,
        )
        .build()
        .context("Failed to build onion service config")?;

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
    info!("========================================");
    info!("🧅 Onion Service Address:");
    info!("   {}", onion_address.display_unredacted());
    info!("========================================");
    info!("");

    // Step 3: Spawn the child process (gunicorn)
    info!("Step 3: Spawning child process...");
    let process_config = ProcessConfig::gunicorn(
        config.socket_path.clone(),
        config.app_dir.clone(),
        &config.app_module,
        config.workers,
    );

    let child_process = ChildProcessManager::spawn(&process_config)
        .context("Failed to spawn child process")?;

    info!("✓ Child process spawned (PID: {})", child_process.pid());

    // Wait for the child process to be ready
    child_process
        .wait_for_ready(10)
        .await
        .context("Child process failed to become ready")?;

    info!("✓ Child process is ready and accepting connections");

    // Step 4: Wait for onion service to be fully reachable
    info!("Step 4: Waiting for onion service to be fully reachable...");
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
    info!("");
    info!("Your web application is now accessible at:");
    info!("   http://{}", onion_address.display_unredacted());
    info!("");
    info!("The application is:");
    info!("  ✓ Accessible ONLY via Tor");
    info!("  ✓ No TCP ports exposed");
    info!("  ✓ Running on UDS: {:?}", config.socket_path);
    info!("  ✓ Process PID: {}", child_process.pid());
    info!("========================================");
    info!("");
    info!("Press Ctrl+C to shut down...");

    // Step 5: Handle incoming requests
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

    let config = EddiConfig::default();

    // Verify the app directory exists
    if !config.app_dir.exists() {
        error!("Application directory not found: {:?}", config.app_dir);
        error!("Please ensure the web application exists at the specified path.");
        anyhow::bail!("Application directory not found");
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
