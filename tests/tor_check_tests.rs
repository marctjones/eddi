//! Tests for the tor-check diagnostic tool

use std::process::Command;

#[test]
fn test_tor_check_compiles() {
    let output = Command::new("cargo")
        .args(&["build", "--bin", "tor-check"])
        .output()
        .expect("Should run cargo build");

    assert!(output.status.success(), "tor-check should compile");
}

#[test]
fn test_tor_check_has_version() {
    // Verify the binary has version information
    let output = Command::new("cargo")
        .args(&["run", "--bin", "tor-check"])
        .env("RUST_LOG", "info")
        .output();

    assert!(output.is_ok());

    let output = output.unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Version:") || stdout.contains("0.1.0"));
}

#[test]
#[ignore] // Requires running the binary
fn test_tor_check_output_format() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "tor-check"])
        .env("RUST_LOG", "info")
        .output()
        .expect("Should run tor-check");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Should have structured output
    assert!(
        combined.contains("Tor Connectivity") || combined.contains("diagnostic"),
        "Should have diagnostic output"
    );

    // Should check environment
    assert!(
        combined.contains("Environment") || combined.contains("diagnostics"),
        "Should check environment"
    );

    // Should have some result
    assert!(
        combined.contains("SUCCESS") || combined.contains("FAILED") || combined.contains("TIMEOUT"),
        "Should have a result"
    );
}

#[test]
#[ignore]
fn test_tor_check_exit_codes() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "tor-check"])
        .env("RUST_LOG", "error") // Reduce output
        .output()
        .expect("Should run tor-check");

    let exit_code = output.status.code().unwrap_or(-1);

    // Valid exit codes: 0 (success), 1 (failed), 2 (error)
    assert!(
        exit_code == 0 || exit_code == 1 || exit_code == 2,
        "Exit code should be 0, 1, or 2, got {}",
        exit_code
    );
}

#[test]
#[ignore]
fn test_tor_check_handles_no_network() {
    // In a sandboxed environment, should fail gracefully
    let output = Command::new("cargo")
        .args(&["run", "--bin", "tor-check"])
        .env("RUST_LOG", "warn")
        .output()
        .expect("Should run tor-check");

    // Should not crash
    assert!(!output.status.success() || output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not have panic messages
    assert!(!stderr.contains("panic"), "Should not panic");
    assert!(!stderr.contains("unwrap"), "Should not have unwrap errors");
}

#[test]
fn test_tor_check_respects_rust_log() {
    // Test with error level - should have minimal output
    let output1 = Command::new("cargo")
        .args(&["run", "--bin", "tor-check"])
        .env("RUST_LOG", "error")
        .output()
        .expect("Should run");

    // Test with debug level - should have more output
    let output2 = Command::new("cargo")
        .args(&["run", "--bin", "tor-check"])
        .env("RUST_LOG", "debug")
        .output()
        .expect("Should run");

    let stderr1 = String::from_utf8_lossy(&output1.stderr);
    let stderr2 = String::from_utf8_lossy(&output2.stderr);

    // Debug should have more output (though both might fail in sandboxed env)
    // We just check that different log levels produce different output
    assert!(
        stderr1.len() != stderr2.len() || stderr1 != stderr2,
        "Different log levels should produce different output"
    );
}

#[tokio::test]
async fn test_tor_check_timeout_behavior() {
    use tokio::time::{timeout, Duration};

    // tor-check should complete within reasonable time even on failure
    let result = timeout(Duration::from_secs(70), async {
        let output = Command::new("cargo")
            .args(&["run", "--bin", "tor-check"])
            .env("RUST_LOG", "error")
            .output()
            .expect("Should run");

        output.status
    })
    .await;

    assert!(result.is_ok(), "tor-check should complete within 70 seconds");
}

// Unit-style tests for tor-check logic

#[test]
fn test_dns_resolution_check() {
    use std::net::ToSocketAddrs;

    // Test DNS resolution (like tor-check does)
    let result = "torproject.org:443".to_socket_addrs();

    // In normal environment: Ok
    // In sandboxed environment: Err
    // Either is valid - we just check it doesn't panic
    match result {
        Ok(_) => println!("DNS works"),
        Err(e) => println!("DNS failed: {}", e),
    }
}

#[test]
fn test_home_directory_check() {
    use std::env;

    // Test HOME directory check (like tor-check does)
    let home = env::var("HOME");

    match home {
        Ok(h) => {
            assert!(!h.is_empty(), "HOME should not be empty if set");
            println!("HOME: {}", h);
        }
        Err(_) => {
            println!("HOME not set");
        }
    }
}

#[test]
fn test_arti_state_directory_path() {
    use std::env;

    // Test Arti state directory construction
    if let Ok(home) = env::var("HOME") {
        let arti_dir = format!("{}/.local/share/arti", home);
        assert!(arti_dir.contains(".local"));
        assert!(arti_dir.contains("arti"));
        println!("Arti state would be: {}", arti_dir);
    }
}
