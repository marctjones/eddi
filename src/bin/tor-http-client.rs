//! tor-http-client - Pure Rust HTTP client via Tor (Arti)
//!
//! This client demonstrates:
//! - Bootstrapping to Tor network using Arti (pure Rust)
//! - Connecting to .onion hidden services via Tor
//! - Connecting to regular websites via Tor (anonymized)
//! - Making HTTP requests over Tor
//! - No proxy servers - direct Tor connection via Arti

use anyhow::{Context, Result, bail};
use arti_client::{TorClient, TorClientConfig};
use tor_rtcompat::PreferredRuntime;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, error, debug};
use tracing_subscriber;
use clap::Parser;

/// tor-http-client - Test connections to onion services and websites via Tor
///
/// This tool allows you to test eddi servers or any other onion service
/// or clearnet website over the Tor network. It supports full URL paths,
/// not just index pages.
///
/// Examples:
///   tor-http-client http://your-onion-address.onion/status
///   tor-http-client https://check.torproject.org
///   tor-http-client http://httpbin.org/ip
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// URL to fetch (supports .onion and clearnet URLs)
    ///
    /// Examples:
    ///   http://example.onion/status
    ///   https://check.torproject.org
    ///   http://httpbin.org/ip
    url: String,

    /// Show only response headers, not body
    #[arg(short = 'H', long)]
    headers_only: bool,

    /// Maximum response body size in bytes (0 = unlimited)
    #[arg(short = 'l', long, default_value = "1048576")]
    max_body_size: usize,

    /// Connection timeout in seconds
    #[arg(short = 't', long, default_value = "30")]
    timeout: u64,

    /// Show detailed connection information
    #[arg(short = 'v', long)]
    verbose: bool,

    /// Quiet mode - only show response body
    #[arg(short = 'q', long)]
    quiet: bool,
}

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
    headers_only: bool,
    max_body_size: usize,
    quiet: bool,
) -> Result<String> {
    let target_type = if is_onion { "onion service" } else { "website (via Tor exit)" };

    if !quiet {
        info!("Connecting to {}:{} via Tor ({})...", address, port, target_type);
    }

    // Connect via Tor - works for both .onion addresses and regular websites
    // For .onion: direct connection within Tor network
    // For regular sites: connection via Tor exit node (anonymized)
    let mut stream = tor_client
        .connect((address, port))
        .await
        .context("Failed to connect via Tor")?;

    if !quiet {
        info!("✓ Connected to {} via Tor", address);
    }

    // Construct HTTP/1.1 GET request
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: tor-http-client/1.0\r\nConnection: close\r\n\r\n",
        path, address
    );

    if !quiet {
        info!("Sending HTTP request...");
    }

    // Send the HTTP request
    stream.write_all(request.as_bytes())
        .await
        .context("Failed to send HTTP request")?;

    stream.flush()
        .await
        .context("Failed to flush stream")?;

    if !quiet {
        info!("✓ Request sent, waiting for response...");
    }

    // Read the response
    let mut response = Vec::new();
    let mut buffer = [0u8; 4096];
    let mut total_read = 0;

    loop {
        // Check size limit
        if max_body_size > 0 && total_read >= max_body_size {
            if !quiet {
                info!("Reached maximum body size limit ({} bytes), stopping read", max_body_size);
            }
            break;
        }

        match stream.read(&mut buffer).await {
            Ok(0) => break, // EOF
            Ok(n) => {
                response.extend_from_slice(&buffer[..n]);
                total_read += n;
            }
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

    if !quiet {
        info!("✓ Received {} bytes", response.len());
    }

    // Convert to string
    let mut response_str = String::from_utf8_lossy(&response).to_string();

    // If headers_only, truncate at first blank line after headers
    if headers_only {
        if let Some(pos) = response_str.find("\r\n\r\n") {
            response_str.truncate(pos + 4);
        } else if let Some(pos) = response_str.find("\n\n") {
            response_str.truncate(pos + 2);
        }
    }

    Ok(response_str)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments first
    let cli = Cli::parse();

    // Initialize logging based on verbosity
    let log_level = if cli.quiet {
        "error"
    } else if cli.verbose {
        "debug"
    } else {
        "info"
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        )
        .init();

    if !cli.quiet {
        info!("=== Tor HTTP Client (via Arti) ===");
        info!("Target: {}", cli.url);
        info!("");
    }

    // Parse the URL
    let (address, port, path, is_onion) = parse_url(&cli.url)
        .context("Failed to parse URL")?;

    let target_type = if is_onion { "Onion service" } else { "Website (via Tor)" };

    if cli.verbose {
        debug!("Parsed URL:");
        debug!("  Address: {}", address);
        debug!("  Port: {}", port);
        debug!("  Path: {}", path);
        debug!("  Type: {}", target_type);
        debug!("");
    }

    // Step 1: Initialize Arti Tor client and bootstrap
    if !cli.quiet {
        info!("Bootstrapping Tor client via Arti...");
        if !cli.verbose {
            info!("(This may take 10-30 seconds on first run)");
        }
    }

    let config = TorClientConfig::default();

    // Use timeout
    let tor_client = tokio::time::timeout(
        std::time::Duration::from_secs(cli.timeout),
        TorClient::create_bootstrapped(config)
    )
    .await
    .context("Tor bootstrap timeout")??;

    if !cli.quiet {
        info!("✓ Tor client bootstrapped successfully");
        info!("");
        info!("Making HTTP request via Tor...");
    }

    // Step 2: Connect and make HTTP request
    let response = http_get_over_tor(
        &tor_client,
        &address,
        port,
        &path,
        is_onion,
        cli.headers_only,
        cli.max_body_size,
        cli.quiet,
    )
    .await
    .context("Failed to make HTTP request")?;

    if !cli.quiet {
        info!("");
        info!("========================================");
        info!("Response received:");
        info!("========================================");
    }

    println!("{}", response);

    if !cli.quiet {
        info!("========================================");
    }

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
