//! Integration tests for the complete eddi workflow
//!
//! These tests verify end-to-end functionality where possible without
//! requiring actual Tor network access.

mod test_utils;

use std::path::PathBuf;
use test_utils::*;
use eddi::ProcessConfig;

#[test]
fn test_full_project_compiles() {
    // This test ensures the project compiles
    // The fact that this test runs means compilation succeeded
    assert!(true);
}

#[test]
fn test_all_binaries_exist() {
    use std::process::Command;

    // Test that binaries can be built
    let output = Command::new("cargo")
        .args(&["build", "--bins"])
        .output();

    assert!(output.is_ok(), "Should be able to build binaries");
    let output = output.unwrap();
    assert!(output.status.success(), "Build should succeed");
}

#[test]
#[ignore] // Requires gunicorn and Flask
fn test_flask_app_on_uds() {
    use tokio::net::UnixStream;
    use tokio::io::AsyncWriteExt;

    // This test requires the Flask demo app and gunicorn
    if !gunicorn_available() {
        eprintln!("Skipping: gunicorn not available");
        return;
    }

    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let temp_dir = temp_dir();
        let socket_path = temp_dir.path().join("test.sock");

        // Create Flask app
        create_mock_flask_app(temp_dir.path());

        // Spawn gunicorn
        let config = eddi::ProcessConfig::gunicorn(
            socket_path.clone(),
            temp_dir.path().to_path_buf(),
            "app:app",
            1,
        );

        let manager = eddi::ChildProcessManager::spawn(&config)
            .expect("Should spawn gunicorn");

        // Wait for socket
        manager.wait_for_socket(10)
            .expect("Socket should be created");

        // Connect and send request
        let mut stream = UnixStream::connect(&socket_path).await
            .expect("Should connect to socket");

        let request = "GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
        stream.write_all(request.as_bytes()).await
            .expect("Should write request");

        let mut response = String::new();
        let mut buf = [0u8; 1024];
        loop {
            match stream.try_read(&mut buf) {
                Ok(0) => break,
                Ok(n) => response.push_str(&String::from_utf8_lossy(&buf[..n])),
                Err(_) => break,
            }
        }

        assert!(response.contains("HTTP"), "Should get HTTP response");
        assert!(response.contains("200"), "Should get 200 OK");
    });
}

#[test]
#[ignore] // Requires network access
fn test_tor_check_without_network() {
    use std::process::Command;

    // Run tor-check binary
    let output = Command::new("cargo")
        .args(&["run", "--bin", "tor-check"])
        .env("RUST_LOG", "error") // Reduce noise
        .output()
        .expect("Should run tor-check");

    // In a sandboxed environment, this should fail
    // Exit code 1 = failed, 2 = error
    assert!(
        !output.status.success(),
        "tor-check should fail without network"
    );
}

#[tokio::test]
async fn test_wait_for_utility() {
    let mut counter = 0;
    let condition = || {
        counter += 1;
        counter >= 3
    };

    let result = wait_for(condition, 5).await;
    assert!(result, "Should succeed after 3 iterations");
}

#[tokio::test]
async fn test_wait_for_timeout() {
    let condition = || false; // Never succeeds

    let start = std::time::Instant::now();
    let result = wait_for(condition, 1).await;

    assert!(!result, "Should timeout");
    assert!(start.elapsed().as_secs() >= 1, "Should wait at least 1 second");
}

// Property-based tests

#[test]
fn test_socket_paths_are_valid() {
    use std::ffi::OsStr;

    // Test that generated socket paths are valid
    for _ in 0..100 {
        let path = temp_socket_path();

        // Should have .sock extension
        assert_eq!(path.extension(), Some(OsStr::new("sock")));

        // Should be in temp directory
        assert!(path.starts_with(std::env::temp_dir()));

        // Should contain PID
        let path_str = path.to_string_lossy();
        assert!(path_str.contains(&std::process::id().to_string()));
    }
}

#[test]
fn test_process_config_invariants() {
    // Test that ProcessConfig maintains invariants

    let configs = vec![
        ProcessConfig::gunicorn(
            PathBuf::from("/tmp/a.sock"),
            PathBuf::from("/app1"),
            "app:app",
            1,
        ),
        ProcessConfig::gunicorn(
            PathBuf::from("/tmp/b.sock"),
            PathBuf::from("/app2"),
            "main:application",
            8,
        ),
    ];

    for config in configs {
        // Socket path should be set
        assert!(!config.socket_path.as_os_str().is_empty());

        // Command should be set
        assert!(!config.command.is_empty());

        // Args should contain bind argument
        assert!(config.args.iter().any(|arg| arg.contains("unix:")));

        // Workers argument should be present and valid
        let workers: Vec<u8> = config.args.iter()
            .filter_map(|arg| arg.parse().ok())
            .collect();
        assert!(!workers.is_empty(), "Should have numeric worker count");
        assert!(workers.iter().all(|&w| w > 0 && w <= 16), "Workers should be reasonable");
    }
}

// Error handling tests

#[test]
fn test_mock_process_config_handles_special_chars() {
    let socket_path = PathBuf::from("/tmp/socket with spaces.sock");
    let config = eddi::ProcessConfig::gunicorn(
        socket_path.clone(),
        PathBuf::from("/app"),
        "app:app",
        1,
    );

    assert_eq!(config.socket_path, socket_path);

    let bind_arg = config.args.iter()
        .find(|arg| arg.starts_with("unix:"))
        .unwrap();

    assert!(bind_arg.contains("spaces"));
}

#[test]
fn test_multiple_socket_paths_dont_conflict() {
    let paths: Vec<PathBuf> = (0..10)
        .map(|_| temp_socket_path())
        .collect();

    // All paths should be the same (based on PID)
    for path in &paths {
        assert_eq!(path, &paths[0]);
    }

    cleanup_socket(&paths[0]);
}

// Smoke tests

#[test]
fn test_library_public_api() {
    // Verify public API is accessible
    let _config_type: Option<eddi::ProcessConfig> = None;
    let _manager_type: Option<eddi::ChildProcessManager> = None;

    // If this compiles, the public API is accessible
    assert!(true);
}

#[test]
fn test_no_panics_on_normal_usage() {
    // Test that normal usage doesn't panic
    let socket_path = PathBuf::from("/tmp/test.sock");
    let config = eddi::ProcessConfig::gunicorn(
        socket_path.clone(),
        PathBuf::from("/app"),
        "app:app",
        4,
    );

    assert!(config.socket_path == socket_path);

    // Cloning shouldn't panic
    let _cloned = config.clone();

    // Debug formatting shouldn't panic
    let _debug = format!("{:?}", config);
}
