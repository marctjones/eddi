//! Task 3: UDS and Child Process Management
//!
//! This demonstrates:
//! 1. Creating a Unix Domain Socket (UDS)
//! 2. Spawning gunicorn as a child process, bound to the UDS
//! 3. Connecting to the UDS from Rust
//! 4. Sending a hardcoded GET / request
//! 5. Reading and printing the response
//!
//! This verifies the UDS/process logic before combining with Arti in Task 4.

use anyhow::{Context, Result, bail};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;
use tracing::{info, error, debug};

/// Configuration for the child process and UDS
struct Config {
    socket_path: PathBuf,
    app_dir: PathBuf,
    gunicorn_workers: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            socket_path: PathBuf::from("/tmp/eddi-task3.sock"),
            app_dir: PathBuf::from("test-apps/flask-demo"),
            gunicorn_workers: 1,
        }
    }
}

/// Manages the gunicorn child process
struct GunicornProcess {
    child: Child,
    socket_path: PathBuf,
}

impl GunicornProcess {
    /// Spawn gunicorn bound to the specified UDS
    fn spawn(config: &Config) -> Result<Self> {
        // Remove existing socket file if it exists
        if config.socket_path.exists() {
            info!("Removing existing socket file: {:?}", config.socket_path);
            fs::remove_file(&config.socket_path)
                .context("Failed to remove existing socket file")?;
        }

        // Build the gunicorn command
        let bind_addr = format!("unix:{}", config.socket_path.display());

        info!("Spawning gunicorn...");
        info!("  Working directory: {:?}", config.app_dir);
        info!("  Bind address: {}", bind_addr);
        info!("  Workers: {}", config.gunicorn_workers);

        let child = Command::new("gunicorn")
            .current_dir(&config.app_dir)
            .arg("--workers")
            .arg(config.gunicorn_workers.to_string())
            .arg("--bind")
            .arg(&bind_addr)
            .arg("app:app")
            .spawn()
            .context("Failed to spawn gunicorn. Is it installed?")?;

        info!("Gunicorn spawned with PID: {}", child.id());

        Ok(Self {
            child,
            socket_path: config.socket_path.clone(),
        })
    }

    /// Get the process ID
    fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Wait for the socket file to be created
    fn wait_for_socket(&self, timeout_secs: u64) -> Result<()> {
        let start = std::time::Instant::now();

        info!("Waiting for socket file to be created...");

        while start.elapsed().as_secs() < timeout_secs {
            if self.socket_path.exists() {
                info!("Socket file created: {:?}", self.socket_path);
                // Give gunicorn a moment to start listening
                thread::sleep(Duration::from_millis(500));
                return Ok(());
            }
            thread::sleep(Duration::from_millis(100));
        }

        bail!("Timeout waiting for socket file: {:?}", self.socket_path)
    }
}

impl Drop for GunicornProcess {
    fn drop(&mut self) {
        info!("Shutting down gunicorn (PID: {})...", self.pid());

        // Try graceful shutdown first
        let _ = self.child.kill();
        let _ = self.child.wait();

        // Clean up socket file
        if self.socket_path.exists() {
            let _ = fs::remove_file(&self.socket_path);
        }

        info!("Gunicorn shut down successfully");
    }
}

/// Connect to the UDS and send an HTTP request
fn send_http_request(socket_path: &Path, method: &str, path: &str) -> Result<String> {
    info!("Connecting to Unix socket: {:?}", socket_path);

    let mut stream = UnixStream::connect(socket_path)
        .context("Failed to connect to Unix socket")?;

    info!("Connected! Sending {} request to {}", method, path);

    // Construct a simple HTTP/1.1 request
    let request = format!(
        "{} {} HTTP/1.1\r\n\
         Host: localhost\r\n\
         Connection: close\r\n\
         \r\n",
        method, path
    );

    stream.write_all(request.as_bytes())
        .context("Failed to write request to socket")?;

    stream.flush()
        .context("Failed to flush socket")?;

    debug!("Request sent, reading response...");

    // Read the response
    let mut response = String::new();
    stream.read_to_string(&mut response)
        .context("Failed to read response from socket")?;

    Ok(response)
}

/// Parse HTTP response and extract status and body
fn parse_http_response(response: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = response.splitn(2, "\r\n\r\n").collect();

    if parts.len() != 2 {
        bail!("Invalid HTTP response format");
    }

    let headers = parts[0];
    let body = parts[1];

    // Extract status line
    let status_line = headers
        .lines()
        .next()
        .context("No status line in response")?;

    Ok((status_line.to_string(), body.to_string()))
}

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    info!("=== Task 3: UDS and Child Process Management ===");

    let config = Config::default();

    // Verify the Flask app directory exists
    if !config.app_dir.exists() {
        error!("Flask app directory not found: {:?}", config.app_dir);
        bail!("Please ensure the Flask demo app exists at {:?}", config.app_dir);
    }

    // Spawn gunicorn
    let gunicorn = GunicornProcess::spawn(&config)?;

    // Wait for the socket to be ready
    gunicorn.wait_for_socket(10)?;

    info!("Gunicorn is ready!");
    info!("");

    // Send a test request to /
    info!("--- Test 1: GET / ---");
    let response = send_http_request(&config.socket_path, "GET", "/")?;
    let (status, body) = parse_http_response(&response)?;
    info!("Status: {}", status);
    info!("Body: {}", body.trim());
    info!("");

    // Send a test request to /status
    info!("--- Test 2: GET /status ---");
    let response = send_http_request(&config.socket_path, "GET", "/status")?;
    let (status, body) = parse_http_response(&response)?;
    info!("Status: {}", status);
    info!("Body: {}", body.trim());
    info!("");

    info!("=== Task 3 Complete ===");
    info!("Successfully demonstrated:");
    info!("  ✓ Creating a Unix Domain Socket");
    info!("  ✓ Spawning gunicorn as a child process");
    info!("  ✓ Binding gunicorn to the UDS");
    info!("  ✓ Connecting to the UDS from Rust");
    info!("  ✓ Sending HTTP requests over the UDS");
    info!("  ✓ Receiving HTTP responses");

    // gunicorn will be shut down when GunicornProcess is dropped
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_http_response() {
        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nHello, World!";
        let (status, body) = parse_http_response(response).unwrap();
        assert_eq!(status, "HTTP/1.1 200 OK");
        assert_eq!(body, "Hello, World!");
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.socket_path, PathBuf::from("/tmp/eddi-task3.sock"));
        assert_eq!(config.gunicorn_workers, 1);
    }
}
