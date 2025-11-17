//! tor-http-client - Pure Rust HTTP client via Tor (Arti)
//!
//! This client demonstrates:
//! - Bootstrapping to Tor network using Arti (pure Rust)
//! - Connecting to .onion hidden services via Tor
//! - Connecting to regular websites via Tor (anonymized)
//! - Making HTTP requests over Tor
//! - No proxy servers - direct Tor connection via Arti
//!
//! Usage:
//!   tor-http-client <url>
//!
//! Examples:
//!   tor-http-client http://example.onion/status
//!   tor-http-client https://check.torproject.org
//!   tor-http-client http://httpbin.org/ip

use anyhow::{Context, Result, bail};
use arti_client::{TorClient, TorClientConfig};
use tor_rtcompat::PreferredRuntime;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, error};
use tracing_subscriber;

/// Parse URL and extract the address, port, and path
/// Supports both .onion addresses and regular websites
fn parse_url(url: &str) -> Result<(String, u16, String, bool)> {
    // Determine if HTTPS
    let is_https = url.starts_with("https://");

    // Remove http:// or https:// prefix if present
    let url = url.trim_start_matches("http://").trim_start_matches("https://");

    // First, extract the path (everything after the first '/')
    let (addr_and_port, path) = if let Some(slash_pos) = url.find('/') {
        (url[..slash_pos].to_string(), url[slash_pos..].to_string())
    } else {
        (url.to_string(), "/".to_string())
    };

    // Now split address and port by ':'
    let parts: Vec<&str> = addr_and_port.split(':').collect();

    if parts.is_empty() {
        bail!("Invalid URL format");
    }

    let address = parts[0].to_string();

    // Validate address (must be .onion or regular domain)
    if address.is_empty() {
        bail!("Empty address");
    }

    // Extract port (default based on protocol)
    let port = if parts.len() > 1 {
        parts[1].parse::<u16>()
            .context("Invalid port number")?
    } else {
        // Default port based on protocol
        if is_https { 443 } else { 80 }
    };

    // Check if it's an onion address
    let is_onion = address.ends_with(".onion");

    Ok((address, port, path, is_onion))
}

/// Make an HTTP GET request over a Tor stream
async fn http_get_over_tor(
    tor_client: &TorClient<PreferredRuntime>,
    address: &str,
    port: u16,
    path: &str,
    is_onion: bool,
) -> Result<String> {
    let target_type = if is_onion { "onion service" } else { "website (via Tor exit)" };
    info!("Connecting to {}:{} via Tor ({})...", address, port, target_type);

    // Connect via Tor - works for both .onion addresses and regular websites
    // For .onion: direct connection within Tor network
    // For regular sites: connection via Tor exit node (anonymized)
    let mut stream = tor_client
        .connect((address, port))
        .await
        .context("Failed to connect via Tor")?;

    info!("✓ Connected to {} via Tor", address);

    // Construct HTTP/1.1 GET request
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: tor-http-client/1.0\r\nConnection: close\r\n\r\n",
        path, address
    );

    info!("Sending HTTP request...");

    // Send the HTTP request
    stream.write_all(request.as_bytes())
        .await
        .context("Failed to send HTTP request")?;

    stream.flush()
        .await
        .context("Failed to flush stream")?;

    info!("✓ Request sent, waiting for response...");

    // Read the response
    let mut response = Vec::new();
    let mut buffer = [0u8; 4096];

    loop {
        match stream.read(&mut buffer).await {
            Ok(0) => break, // EOF
            Ok(n) => response.extend_from_slice(&buffer[..n]),
            Err(e) => {
                // END cell with MISC reason is normal graceful closure
                let err_str = e.to_string();
                if err_str.contains("END cell with reason MISC") || err_str.contains("END") {
                    // Normal connection closure from server
                    break;
                } else {
                    error!("Error reading response: {}", e);
                    break;
                }
            }
        }
    }

    info!("✓ Received {} bytes", response.len());

    // Convert to string
    let response_str = String::from_utf8_lossy(&response).to_string();

    Ok(response_str)
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

    // Get the onion URL from command line arguments
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <url>", args[0]);
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  Onion services:");
        eprintln!("    {} http://example.onion:80", args[0]);
        eprintln!("    {} example.onion:80/status", args[0]);
        eprintln!();
        eprintln!("  Regular websites (via Tor exit):");
        eprintln!("    {} https://check.torproject.org", args[0]);
        eprintln!("    {} http://httpbin.org/ip", args[0]);
        eprintln!();
        eprintln!("This client uses Arti to connect directly via Tor.");
        eprintln!("No proxy servers or IP-based protocols are used.");
        std::process::exit(1);
    }

    let onion_url = &args[1];

    info!("=== Tor HTTP Client (via Arti) ===");
    info!("Target: {}", onion_url);
    info!("");

    // Parse the URL
    let (address, port, path, is_onion) = parse_url(onion_url)
        .context("Failed to parse URL")?;

    let target_type = if is_onion { "Onion service" } else { "Website (via Tor)" };
    info!("Parsed URL:");
    info!("  Address: {}", address);
    info!("  Port: {}", port);
    info!("  Path: {}", path);
    info!("  Type: {}", target_type);
    info!("");

    // Step 1: Initialize Arti Tor client and bootstrap
    info!("Step 1: Bootstrapping Tor client via Arti...");
    info!("(This may take 10-30 seconds on first run)");

    let config = TorClientConfig::default();
    let tor_client = TorClient::create_bootstrapped(config)
        .await
        .context("Failed to bootstrap Tor client")?;

    info!("✓ Tor client bootstrapped successfully");
    info!("");

    // Step 2: Connect and make HTTP request
    info!("Step 2: Making HTTP request via Tor...");

    let response = http_get_over_tor(&tor_client, &address, port, &path, is_onion)
        .await
        .context("Failed to make HTTP request")?;

    info!("");
    info!("========================================");
    info!("Response received:");
    info!("========================================");
    println!("{}", response);
    info!("========================================");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_url_onion() {
        // Test onion with http:// prefix
        let (addr, port, path, is_onion) = parse_url("http://example.onion:80").unwrap();
        assert_eq!(addr, "example.onion");
        assert_eq!(port, 80);
        assert_eq!(path, "/");
        assert!(is_onion);

        // Test onion with path
        let (addr, port, path, is_onion) = parse_url("example.onion:80/status").unwrap();
        assert_eq!(addr, "example.onion");
        assert_eq!(port, 80);
        assert_eq!(path, "/status");
        assert!(is_onion);

        // Test onion without port (default to 80)
        let (addr, port, path, is_onion) = parse_url("example.onion").unwrap();
        assert_eq!(addr, "example.onion");
        assert_eq!(port, 80);
        assert_eq!(path, "/");
        assert!(is_onion);

        // Test onion without port but with path
        let (addr, port, path, is_onion) = parse_url("example.onion/status").unwrap();
        assert_eq!(addr, "example.onion");
        assert_eq!(port, 80);
        assert_eq!(path, "/status");
        assert!(is_onion);

        // Test onion with long path
        let (addr, port, path, is_onion) = parse_url("http://example.onion/api/v1/health").unwrap();
        assert_eq!(addr, "example.onion");
        assert_eq!(port, 80);
        assert_eq!(path, "/api/v1/health");
        assert!(is_onion);

        // Test onion with https (default port 443)
        let (addr, port, path, is_onion) = parse_url("https://example.onion").unwrap();
        assert_eq!(addr, "example.onion");
        assert_eq!(port, 443);
        assert_eq!(path, "/");
        assert!(is_onion);
    }

    #[test]
    fn test_parse_url_regular() {
        // Test regular domain with http
        let (addr, port, path, is_onion) = parse_url("http://example.com").unwrap();
        assert_eq!(addr, "example.com");
        assert_eq!(port, 80);
        assert_eq!(path, "/");
        assert!(!is_onion);

        // Test regular domain with https
        let (addr, port, path, is_onion) = parse_url("https://check.torproject.org").unwrap();
        assert_eq!(addr, "check.torproject.org");
        assert_eq!(port, 443);
        assert_eq!(path, "/");
        assert!(!is_onion);

        // Test regular domain with path
        let (addr, port, path, is_onion) = parse_url("https://httpbin.org/ip").unwrap();
        assert_eq!(addr, "httpbin.org");
        assert_eq!(port, 443);
        assert_eq!(path, "/ip");
        assert!(!is_onion);

        // Test regular domain with explicit port
        let (addr, port, path, is_onion) = parse_url("http://example.com:8080/api").unwrap();
        assert_eq!(addr, "example.com");
        assert_eq!(port, 8080);
        assert_eq!(path, "/api");
        assert!(!is_onion);
    }
}
