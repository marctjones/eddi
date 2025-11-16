//! Test utilities and fixtures for eddi
//!
//! This module provides common test utilities, fixtures, and mock implementations
//! used across the test suite.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::fs;
use std::io::Write;
use tempfile::TempDir;

/// Create a temporary directory for testing
pub fn temp_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp directory")
}

/// Create a mock Flask application for testing
pub fn create_mock_flask_app(dir: &std::path::Path) -> PathBuf {
    let app_path = dir.join("app.py");
    let mut file = fs::File::create(&app_path).expect("Failed to create mock app");

    writeln!(file, "from flask import Flask").unwrap();
    writeln!(file, "app = Flask(__name__)").unwrap();
    writeln!(file, "").unwrap();
    writeln!(file, "@app.route('/')").unwrap();
    writeln!(file, "def index():").unwrap();
    writeln!(file, "    return 'Test OK'").unwrap();

    app_path
}

/// Check if a command is available in PATH
pub fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Check if Python is available
pub fn python_available() -> bool {
    command_exists("python3") || command_exists("python")
}

/// Check if gunicorn is available
pub fn gunicorn_available() -> bool {
    command_exists("gunicorn")
}

/// Get a unique temporary socket path
pub fn temp_socket_path() -> PathBuf {
    let temp_dir = std::env::temp_dir();
    let unique_name = format!("eddi-test-{}.sock", std::process::id());
    temp_dir.join(unique_name)
}

/// Clean up a socket file if it exists
pub fn cleanup_socket(path: &std::path::Path) {
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}

/// Wait for a condition with timeout
pub async fn wait_for<F>(mut condition: F, timeout_secs: u64) -> bool
where
    F: FnMut() -> bool,
{
    use tokio::time::{sleep, Duration};

    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < timeout_secs {
        if condition() {
            return true;
        }
        sleep(Duration::from_millis(100)).await;
    }
    false
}

/// Mock process configuration for testing
pub fn mock_process_config(socket_path: PathBuf) -> eddi::ProcessConfig {
    eddi::ProcessConfig {
        socket_path,
        app_dir: PathBuf::from("/tmp"),
        command: "echo".to_string(),
        args: vec!["test".to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temp_dir_creation() {
        let dir = temp_dir();
        assert!(dir.path().exists());
    }

    #[test]
    fn test_temp_socket_path_unique() {
        let path1 = temp_socket_path();
        let path2 = temp_socket_path();
        // Should contain process ID
        assert!(path1.to_string_lossy().contains(&std::process::id().to_string()));
        // Paths should be the same for same process
        assert_eq!(path1, path2);
    }

    #[test]
    fn test_command_exists() {
        // These should exist on most systems
        assert!(command_exists("echo") || command_exists("ls"));
    }

    #[test]
    fn test_cleanup_socket_nonexistent() {
        let path = PathBuf::from("/tmp/nonexistent-socket.sock");
        cleanup_socket(&path); // Should not panic
    }
}
