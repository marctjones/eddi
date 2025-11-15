//! Child process management for web applications
//!
//! This module provides utilities for spawning and managing web server
//! processes (like gunicorn, uvicorn, php-fpm) bound to Unix Domain Sockets.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;
use tracing::{info, debug};

/// Configuration for the child process
#[derive(Debug, Clone)]
pub struct ProcessConfig {
    /// Path to the Unix Domain Socket
    pub socket_path: PathBuf,

    /// Working directory for the application
    pub app_dir: PathBuf,

    /// Command to run (e.g., "gunicorn", "uvicorn")
    pub command: String,

    /// Arguments to pass to the command
    pub args: Vec<String>,
}

impl ProcessConfig {
    /// Create a new configuration for a gunicorn process
    pub fn gunicorn(socket_path: PathBuf, app_dir: PathBuf, app_module: &str, workers: u8) -> Self {
        let bind_addr = format!("unix:{}", socket_path.display());

        Self {
            socket_path,
            app_dir,
            command: "gunicorn".to_string(),
            args: vec![
                "--workers".to_string(),
                workers.to_string(),
                "--bind".to_string(),
                bind_addr,
                app_module.to_string(),
            ],
        }
    }
}

/// Manages a child process bound to a Unix Domain Socket
pub struct ChildProcessManager {
    child: Child,
    socket_path: PathBuf,
}

impl ChildProcessManager {
    /// Spawn a child process with the given configuration
    pub fn spawn(config: &ProcessConfig) -> Result<Self> {
        // Remove existing socket file if it exists
        if config.socket_path.exists() {
            info!("Removing existing socket file: {:?}", config.socket_path);
            fs::remove_file(&config.socket_path)
                .context("Failed to remove existing socket file")?;
        }

        info!("Spawning child process...");
        info!("  Command: {}", config.command);
        info!("  Working directory: {:?}", config.app_dir);
        info!("  Args: {:?}", config.args);

        let child = Command::new(&config.command)
            .current_dir(&config.app_dir)
            .args(&config.args)
            .spawn()
            .with_context(|| format!("Failed to spawn {}. Is it installed?", config.command))?;

        info!("Child process spawned with PID: {}", child.id());

        Ok(Self {
            child,
            socket_path: config.socket_path.clone(),
        })
    }

    /// Get the process ID
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Get the socket path
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Wait for the socket file to be created
    pub fn wait_for_socket(&self, timeout_secs: u64) -> Result<()> {
        let start = std::time::Instant::now();

        info!("Waiting for socket file to be created...");

        while start.elapsed().as_secs() < timeout_secs {
            if self.socket_path.exists() {
                info!("Socket file created: {:?}", self.socket_path);
                // Give the process a moment to start listening
                thread::sleep(Duration::from_millis(500));
                return Ok(());
            }
            thread::sleep(Duration::from_millis(100));
        }

        anyhow::bail!("Timeout waiting for socket file: {:?}", self.socket_path)
    }

    /// Try to connect to the socket to verify it's ready
    pub async fn wait_for_ready(&self, timeout_secs: u64) -> Result<()> {
        use tokio::net::UnixStream;
        use tokio::time::{timeout, Duration};

        self.wait_for_socket(timeout_secs)?;

        info!("Attempting to connect to socket...");

        // Try to actually connect
        let connect_timeout = Duration::from_secs(timeout_secs);
        timeout(connect_timeout, async {
            let mut attempts = 0;
            loop {
                match UnixStream::connect(&self.socket_path).await {
                    Ok(_) => {
                        info!("Successfully connected to socket - process is ready");
                        return Ok(());
                    }
                    Err(e) => {
                        attempts += 1;
                        if attempts > 10 {
                            anyhow::bail!("Failed to connect after {} attempts: {}", attempts, e);
                        }
                        debug!("Connect attempt {} failed, retrying...", attempts);
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        })
        .await
        .context("Timeout waiting for process to accept connections")?
    }
}

impl Drop for ChildProcessManager {
    fn drop(&mut self) {
        info!("Shutting down child process (PID: {})...", self.pid());

        // Try graceful shutdown first
        let _ = self.child.kill();
        let _ = self.child.wait();

        // Clean up socket file
        if self.socket_path.exists() {
            let _ = fs::remove_file(&self.socket_path);
        }

        info!("Child process shut down successfully");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gunicorn_config() {
        let config = ProcessConfig::gunicorn(
            PathBuf::from("/tmp/test.sock"),
            PathBuf::from("/app"),
            "app:app",
            2,
        );

        assert_eq!(config.command, "gunicorn");
        assert_eq!(config.socket_path, PathBuf::from("/tmp/test.sock"));
        assert!(config.args.contains(&"--workers".to_string()));
        assert!(config.args.contains(&"2".to_string()));
    }
}
