//! Comprehensive unit tests for process management
//!
//! Tests the ChildProcessManager and ProcessConfig functionality

mod test_utils;

use eddi::{ChildProcessManager, ProcessConfig};
use std::path::PathBuf;
use test_utils::*;

#[test]
fn test_process_config_gunicorn_creation() {
    let socket_path = PathBuf::from("/var/run/app.sock");
    let app_dir = PathBuf::from("/opt/myapp");
    let config = ProcessConfig::gunicorn(
        socket_path.clone(),
        app_dir.clone(),
        "myapp:application",
        4,
    );

    assert_eq!(config.command, "gunicorn");
    assert_eq!(config.socket_path, socket_path);
    assert_eq!(config.app_dir, app_dir);
    assert!(config.args.contains(&"--workers".to_string()));
    assert!(config.args.contains(&"4".to_string()));
    assert!(config.args.contains(&"--bind".to_string()));
    assert!(config.args.iter().any(|arg| arg.starts_with("unix:")));
    assert!(config.args.contains(&"myapp:application".to_string()));
}

#[test]
fn test_process_config_gunicorn_socket_path_in_args() {
    let socket_path = PathBuf::from("/tmp/test.sock");
    let config = ProcessConfig::gunicorn(
        socket_path.clone(),
        PathBuf::from("/app"),
        "app:app",
        1,
    );

    let bind_arg = config.args.iter()
        .find(|arg| arg.starts_with("unix:"))
        .expect("Should have unix: bind argument");

    assert!(bind_arg.contains("test.sock"));
}

#[test]
fn test_process_config_workers_range() {
    // Test various worker counts
    for workers in [1, 2, 4, 8, 16] {
        let config = ProcessConfig::gunicorn(
            PathBuf::from("/tmp/test.sock"),
            PathBuf::from("/app"),
            "app:app",
            workers,
        );

        assert!(config.args.contains(&workers.to_string()));
    }
}

#[test]
fn test_process_config_custom() {
    let socket_path = PathBuf::from("/tmp/custom.sock");
    let config = ProcessConfig {
        socket_path: socket_path.clone(),
        app_dir: PathBuf::from("/custom/app"),
        command: "uvicorn".to_string(),
        args: vec![
            "main:app".to_string(),
            "--uds".to_string(),
            socket_path.to_string_lossy().to_string(),
        ],
    };

    assert_eq!(config.command, "uvicorn");
    assert_eq!(config.socket_path, socket_path);
    assert!(config.args.contains(&"main:app".to_string()));
    assert!(config.args.contains(&"--uds".to_string()));
}

#[test]
fn test_process_config_clone() {
    let config = ProcessConfig::gunicorn(
        PathBuf::from("/tmp/test.sock"),
        PathBuf::from("/app"),
        "app:app",
        2,
    );

    let cloned = config.clone();

    assert_eq!(config.socket_path, cloned.socket_path);
    assert_eq!(config.app_dir, cloned.app_dir);
    assert_eq!(config.command, cloned.command);
    assert_eq!(config.args, cloned.args);
}

#[test]
fn test_process_config_debug() {
    let config = ProcessConfig::gunicorn(
        PathBuf::from("/tmp/test.sock"),
        PathBuf::from("/app"),
        "app:app",
        2,
    );

    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("ProcessConfig"));
    assert!(debug_str.contains("socket_path"));
    assert!(debug_str.contains("gunicorn"));
}

// Integration-style tests (require actual commands)

#[test]
#[ignore] // Only run with: cargo test -- --ignored
fn test_spawn_echo_process() {
    let socket_path = temp_socket_path();
    cleanup_socket(&socket_path);

    let config = ProcessConfig {
        socket_path: socket_path.clone(),
        app_dir: PathBuf::from("/tmp"),
        command: "sleep".to_string(),
        args: vec!["0.1".to_string()],
    };

    let result = ChildProcessManager::spawn(&config);

    // Clean up
    cleanup_socket(&socket_path);

    assert!(result.is_ok(), "Should spawn simple process");
    let manager = result.unwrap();
    assert!(manager.pid() > 0);
}

#[test]
#[ignore]
fn test_spawn_nonexistent_command() {
    let socket_path = temp_socket_path();
    let config = ProcessConfig {
        socket_path: socket_path.clone(),
        app_dir: PathBuf::from("/tmp"),
        command: "this-command-does-not-exist-123456".to_string(),
        args: vec![],
    };

    let result = ChildProcessManager::spawn(&config);

    cleanup_socket(&socket_path);

    assert!(result.is_err(), "Should fail to spawn nonexistent command");
}

#[test]
#[ignore]
fn test_process_manager_pid() {
    let socket_path = temp_socket_path();
    cleanup_socket(&socket_path);

    let config = ProcessConfig {
        socket_path: socket_path.clone(),
        app_dir: PathBuf::from("/tmp"),
        command: "sleep".to_string(),
        args: vec!["1".to_string()],
    };

    let manager = ChildProcessManager::spawn(&config)
        .expect("Should spawn process");

    let pid = manager.pid();
    assert!(pid > 0);

    // Verify process exists
    let status = std::process::Command::new("ps")
        .arg("-p")
        .arg(pid.to_string())
        .output();

    cleanup_socket(&socket_path);

    assert!(status.is_ok());
}

#[test]
#[ignore]
fn test_socket_path_getter() {
    let socket_path = temp_socket_path();
    cleanup_socket(&socket_path);

    let config = ProcessConfig {
        socket_path: socket_path.clone(),
        app_dir: PathBuf::from("/tmp"),
        command: "sleep".to_string(),
        args: vec!["0.5".to_string()],
    };

    let manager = ChildProcessManager::spawn(&config)
        .expect("Should spawn process");

    assert_eq!(manager.socket_path(), socket_path.as_path());

    cleanup_socket(&socket_path);
}

#[test]
#[ignore]
fn test_cleanup_on_drop() {
    let socket_path = temp_socket_path();
    cleanup_socket(&socket_path);

    // Create a socket file manually
    std::fs::write(&socket_path, b"").expect("Create socket file");
    assert!(socket_path.exists());

    {
        let config = ProcessConfig {
            socket_path: socket_path.clone(),
            app_dir: PathBuf::from("/tmp"),
            command: "sleep".to_string(),
            args: vec!["0.1".to_string()],
        };

        let manager = ChildProcessManager::spawn(&config)
            .expect("Should spawn process");

        assert!(manager.pid() > 0);

        // Manager goes out of scope here
    }

    // Give it a moment
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Socket should be cleaned up
    assert!(!socket_path.exists(), "Socket should be cleaned up on drop");
}
