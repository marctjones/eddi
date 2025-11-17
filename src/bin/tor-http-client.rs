//! tor-http-client - Simple HTTP client that connects via Arti Tor
//!
//! This client demonstrates:
//! - Bootstrapping to Tor network using Arti
//! - Connecting to onion services via Tor (no IP-based protocols)
//! - Making HTTP requests over Tor
//! - No proxy servers - direct Tor connection via Arti
//!
//! Usage:
//!   tor-http-client <onion-address-with-port>
//!   Example: tor-http-client "http://example.onion:80"

use anyhow::{Context, Result, bail};
use arti_client::{TorClient, TorClientConfig};
use tor_rtcompat::PreferredRuntime;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, error};
use tracing_subscriber;

/// Parse the onion URL and extract the address and port
fn parse_onion_url(url: &str) -> Result<(String, u16, String)> {
    // Remove http:// or https:// prefix if present
    let url = url.trim_start_matches("http://").trim_start_matches("https://");

    // Split by ':' to separate address and port
    let parts: Vec<&str> = url.split(':').collect();

    if parts.is_empty() {
        bail!("Invalid URL format");
    }

    let address = parts[0].to_string();

    // Validate it's an onion address
    if !address.ends_with(".onion") {
        bail!("Address must be a .onion address (Tor hidden service)");
    }

    // Extract port (default to 80)
    let port = if parts.len() > 1 {
        // Remove any path component
        let port_part = parts[1].split('/').next().unwrap_or("80");
        port_part.parse::<u16>()
            .context("Invalid port number")?
    } else {
        80
    };

    // Extract path (default to "/")
    let path = if let Some(slash_pos) = url.find('/') {
        url[slash_pos..].to_string()
    } else {
        "/".to_string()
    };

    Ok((address, port, path))
}

/// Make an HTTP GET request over a Tor stream
async fn http_get_over_tor(
    tor_client: &TorClient<PreferredRuntime>,
    onion_address: &str,
    port: u16,
    path: &str,
) -> Result<String> {
    info!("Connecting to {}:{} via Tor...", onion_address, port);

    // Connect to the onion service via Tor
    // This uses ONLY Tor - no IP-based protocols, no proxy servers
    let mut stream = tor_client
        .connect((onion_address, port))
        .await
        .context("Failed to connect to onion service via Tor")?;

    info!("✓ Connected to onion service via Tor");

    // Construct HTTP/1.1 GET request
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, onion_address
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
        eprintln!("Usage: {} <onion-url>", args[0]);
        eprintln!();
        eprintln!("Example:");
        eprintln!("  {} http://example.onion:80", args[0]);
        eprintln!("  {} example.onion:80/status", args[0]);
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
    let (onion_address, port, path) = parse_onion_url(onion_url)
        .context("Failed to parse onion URL")?;

    info!("Parsed URL:");
    info!("  Address: {}", onion_address);
    info!("  Port: {}", port);
    info!("  Path: {}", path);
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

    let response = http_get_over_tor(&tor_client, &onion_address, port, &path)
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
    fn test_parse_onion_url() {
        // Test with http:// prefix
        let (addr, port, path) = parse_onion_url("http://example.onion:80").unwrap();
        assert_eq!(addr, "example.onion");
        assert_eq!(port, 80);
        assert_eq!(path, "/");

        // Test with path
        let (addr, port, path) = parse_onion_url("example.onion:80/status").unwrap();
        assert_eq!(addr, "example.onion");
        assert_eq!(port, 80);
        assert_eq!(path, "/status");

        // Test without port (default to 80)
        let (addr, port, path) = parse_onion_url("example.onion").unwrap();
        assert_eq!(addr, "example.onion");
        assert_eq!(port, 80);
        assert_eq!(path, "/");

        // Test non-onion address (should fail)
        assert!(parse_onion_url("example.com").is_err());
    }
}
