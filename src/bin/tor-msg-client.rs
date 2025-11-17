//! Simple Tor message client
//!
//! Connects to a Tor hidden service and sends/receives messages.
//! Usage: tor-msg-client <onion-address>:9999

use anyhow::{Context, Result};
use std::env;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use arti_client::TorClient;

#[tokio::main]
async fn main() -> Result<()> {
    // Get onion address from command line
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <onion-address>:9999", args[0]);
        eprintln!();
        eprintln!("Example:");
        eprintln!("  {} abcdef1234567890.onion:9999", args[0]);
        std::process::exit(1);
    }

    let target = &args[1];

    // Parse the target address (format: hostname:port)
    let parts: Vec<&str> = target.split(':').collect();
    if parts.len() != 2 {
        eprintln!("Error: Address must be in format <hostname>:<port>");
        eprintln!("Example: abcdef1234567890.onion:9999");
        std::process::exit(1);
    }

    let hostname = parts[0];
    let port: u16 = parts[1].parse().context("Invalid port number")?;

    eprintln!("=== Tor Message Client ===");
    eprintln!();
    eprintln!("[1/3] Bootstrapping to Tor network...");

    // Initialize Tor client
    let tor_client = TorClient::create_bootstrapped(Default::default())
        .await
        .context("Failed to bootstrap Tor client")?;

    eprintln!("[1/3] ✓ Connected to Tor");
    eprintln!();

    // Connect to the hidden service
    eprintln!("[2/3] Connecting to {}:{}...", hostname, port);
    let stream = tor_client
        .connect((hostname, port))
        .await
        .context("Failed to connect to hidden service")?;

    eprintln!("[2/3] ✓ Connected to hidden service");
    eprintln!();
    eprintln!("[3/3] Connection established!");
    eprintln!();
    eprintln!("Type messages and press Enter to send.");
    eprintln!("Press Ctrl+C to quit.");
    eprintln!();

    // Split stream into read and write halves
    let (mut read_half, mut write_half) = tokio::io::split(stream);

    // Spawn a task to read from the server
    let read_task = tokio::spawn(async move {
        let mut buffer = vec![0u8; 4096];
        loop {
            match read_half.read(&mut buffer).await {
                Ok(0) => {
                    eprintln!();
                    eprintln!("Connection closed by server");
                    break;
                }
                Ok(n) => {
                    let msg = String::from_utf8_lossy(&buffer[..n]);
                    print!("{}", msg);
                    // Flush stdout to ensure message is displayed immediately
                    use std::io::Write;
                    std::io::stdout().flush().ok();
                }
                Err(e) => {
                    eprintln!();
                    eprintln!("Error reading from server: {}", e);
                    break;
                }
            }
        }
    });

    // Read from stdin and send to server
    let stdin = tokio::io::stdin();
    let mut stdin_reader = tokio::io::BufReader::new(stdin);
    let mut line = String::new();

    loop {
        line.clear();

        // Read line from stdin
        match tokio::io::AsyncBufReadExt::read_line(&mut stdin_reader, &mut line).await {
            Ok(0) => {
                // EOF
                eprintln!();
                eprintln!("Disconnecting...");
                break;
            }
            Ok(_) => {
                // Send to server
                if let Err(e) = write_half.write_all(line.as_bytes()).await {
                    eprintln!("Error sending message: {}", e);
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error reading from stdin: {}", e);
                break;
            }
        }
    }

    // Cancel read task
    read_task.abort();

    Ok(())
}
