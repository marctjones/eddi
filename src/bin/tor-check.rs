//! Tor Connectivity Check Tool
//!
//! This tool tests whether the system can connect to the Tor network
//! and provides diagnostic information for troubleshooting.

use anyhow::Result;
use std::time::Duration;
use tracing::{info, warn, error};
use tracing_subscriber;

use arti_client::TorClient;

/// Test Tor connectivity with detailed diagnostics
async fn check_tor_connectivity() -> Result<bool> {
    info!("=== Tor Connectivity Check ===");
    info!("");

    // Step 1: Network connectivity
    info!("Step 1: Checking basic network connectivity...");
    // We'll rely on Arti to check this when it tries to connect

    // Step 2: Attempt to bootstrap Tor client
    info!("Step 2: Attempting to bootstrap Tor client...");
    info!("  This may take 10-30 seconds...");

    let start = std::time::Instant::now();

    match tokio::time::timeout(
        Duration::from_secs(60),
        TorClient::create_bootstrapped(Default::default())
    ).await {
        Ok(Ok(_tor_client)) => {
            let elapsed = start.elapsed();
            info!("");
            info!("✅ SUCCESS: Connected to Tor network!");
            info!("   Bootstrap time: {:.2}s", elapsed.as_secs_f64());
            info!("");
            info!("Tor connectivity is working correctly.");
            info!("You can now run 'eddi' to start your onion service.");
            Ok(true)
        }
        Ok(Err(e)) => {
            error!("");
            error!("❌ FAILED: Could not connect to Tor network");
            error!("   Error: {}", e);
            error!("");
            error!("Possible causes:");
            error!("  1. No internet connectivity");
            error!("  2. Firewall blocking outbound connections");
            error!("  3. Tor directory authorities unreachable");
            error!("  4. DNS resolution issues");
            error!("");
            error!("Troubleshooting steps:");
            error!("  - Check internet: ping 8.8.8.8");
            error!("  - Check DNS: nslookup torproject.org");
            error!("  - Check firewall rules");
            error!("  - Try: curl https://www.torproject.org");
            error!("");
            Ok(false)
        }
        Err(_) => {
            error!("");
            error!("❌ TIMEOUT: Tor bootstrap took longer than 60 seconds");
            error!("");
            error!("This usually indicates:");
            error!("  - Slow/unstable internet connection");
            error!("  - Network filtering/throttling");
            error!("  - Tor directory authorities overloaded");
            error!("");
            error!("Try again in a few minutes.");
            error!("");
            Ok(false)
        }
    }
}

/// Test if we're in a sandboxed/restricted environment
fn check_environment() {
    info!("Environment diagnostics:");

    // Check if we can resolve DNS
    match std::net::ToSocketAddrs::to_socket_addrs(&("torproject.org", 443)) {
        Ok(mut addrs) => {
            if let Some(addr) = addrs.next() {
                info!("  ✅ DNS resolution: OK (torproject.org -> {})", addr.ip());
            } else {
                warn!("  ⚠️  DNS resolution: No addresses returned");
            }
        }
        Err(e) => {
            warn!("  ⚠️  DNS resolution: FAILED ({})", e);
        }
    }

    // Check home directory (for Arti state)
    match std::env::var("HOME") {
        Ok(home) => {
            info!("  ✅ HOME directory: {}", home);
            let arti_dir = format!("{}/.local/share/arti", home);
            if std::path::Path::new(&arti_dir).exists() {
                info!("  ✅ Arti state directory exists: {}", arti_dir);
            } else {
                info!("  ℹ️  Arti state directory will be created: {}", arti_dir);
            }
        }
        Err(_) => {
            warn!("  ⚠️  HOME environment variable not set");
        }
    }

    info!("");
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    info!("Tor Connectivity Diagnostic Tool");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));
    info!("");

    // Check environment first
    check_environment();

    // Attempt Tor connection
    match check_tor_connectivity().await {
        Ok(true) => {
            info!("=== Check Complete: PASSED ✅ ===");
            std::process::exit(0);
        }
        Ok(false) => {
            error!("=== Check Complete: FAILED ❌ ===");
            std::process::exit(1);
        }
        Err(e) => {
            error!("Error during check: {}", e);
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_check() {
        // This should not panic
        check_environment();
    }
}
