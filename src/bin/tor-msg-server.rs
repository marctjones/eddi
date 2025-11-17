//! Simple Tor message server
//!
//! Launches a Tor hidden service that accepts connections and relays messages
//! between connected clients. The onion address is printed to stdout.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use futures::StreamExt;

use arti_client::TorClient;
use tor_hsservice::config::OnionServiceConfigBuilder;
use tor_hsservice::{StreamRequest, handle_rend_requests};
use tor_proto::client::stream::IncomingStreamRequest;
use tor_cell::relaycell::msg::Connected;
use safelog::DisplayRedacted;

type ClientId = usize;
type ClientMap = Arc<Mutex<HashMap<ClientId, mpsc::UnboundedSender<Vec<u8>>>>>;

/// Handle an incoming client connection
async fn handle_client(
    mut stream: impl AsyncReadExt + AsyncWriteExt + Unpin + Send + 'static,
    client_id: ClientId,
    clients: ClientMap,
) -> Result<()> {
    eprintln!("[Server] Client {} connected", client_id);

    // Create channel for this client
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Add client to the map
    {
        let mut clients_guard = clients.lock().await;
        clients_guard.insert(client_id, tx);
    }

    // Send welcome message
    let welcome = format!("Welcome! You are client #{}. Type messages to broadcast.\n", client_id);
    if let Err(e) = stream.write_all(welcome.as_bytes()).await {
        eprintln!("[Server] Failed to send welcome to client {}: {}", client_id, e);
    }

    // Split the stream for reading and writing
    let (mut read_half, mut write_half) = tokio::io::split(stream);

    // Spawn task to handle outgoing messages to this client
    let client_id_clone = client_id;
    let write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = write_half.write_all(&msg).await {
                eprintln!("[Server] Failed to send to client {}: {}", client_id_clone, e);
                break;
            }
        }
    });

    // Handle incoming messages from this client
    let mut buffer = vec![0u8; 4096];
    loop {
        match read_half.read(&mut buffer).await {
            Ok(0) => {
                // Connection closed
                eprintln!("[Server] Client {} disconnected", client_id);
                break;
            }
            Ok(n) => {
                let msg = &buffer[..n];
                let msg_str = String::from_utf8_lossy(msg);
                eprintln!("[Server] Client {} sent: {}", client_id, msg_str.trim());

                // Broadcast to all clients (including sender for echo)
                let broadcast_msg = format!("[Client {}] {}", client_id, msg_str);
                let broadcast_bytes = broadcast_msg.as_bytes().to_vec();

                let clients_guard = clients.lock().await;
                for (id, tx) in clients_guard.iter() {
                    if let Err(e) = tx.send(broadcast_bytes.clone()) {
                        eprintln!("[Server] Failed to queue message for client {}: {}", id, e);
                    }
                }
            }
            Err(e) => {
                eprintln!("[Server] Error reading from client {}: {}", client_id, e);
                break;
            }
        }
    }

    // Remove client from map
    {
        let mut clients_guard = clients.lock().await;
        clients_guard.remove(&client_id);
    }

    // Cancel write task
    write_task.abort();

    Ok(())
}

/// Handle an incoming stream request from the onion service
async fn handle_stream_request(
    stream_request: StreamRequest,
    clients: ClientMap,
    next_client_id: Arc<Mutex<ClientId>>,
) -> Result<()> {
    match stream_request.request() {
        IncomingStreamRequest::Begin(begin) => {
            let port = begin.port();

            // Only accept connections on port 9999
            if port != 9999 {
                eprintln!("[Server] Rejecting connection on unexpected port {}", port);
                stream_request.shutdown_circuit()?;
                return Ok(());
            }

            // Accept the stream
            let stream = stream_request
                .accept(Connected::new_empty())
                .await
                .context("Failed to accept stream from onion service")?;

            // Assign client ID
            let client_id = {
                let mut id_guard = next_client_id.lock().await;
                let id = *id_guard;
                *id_guard += 1;
                id
            };

            // Handle the client in a separate task
            tokio::spawn(async move {
                if let Err(e) = handle_client(stream, client_id, clients).await {
                    eprintln!("[Server] Error handling client {}: {}", client_id, e);
                }
            });
        }
        IncomingStreamRequest::BeginDir(_) => {
            stream_request.shutdown_circuit()?;
        }
        _ => {
            stream_request.shutdown_circuit()?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    eprintln!("=== Tor Message Server ===");
    eprintln!();

    // Initialize Tor client
    eprintln!("[1/3] Bootstrapping to Tor network...");
    let tor_client = TorClient::create_bootstrapped(Default::default())
        .await
        .context("Failed to bootstrap Tor client")?;
    eprintln!("[1/3] ✓ Connected to Tor");
    eprintln!();

    // Launch onion service
    eprintln!("[2/3] Launching hidden service...");
    let svc_config = OnionServiceConfigBuilder::default()
        .nickname("tor-msg-server".parse()?)
        .build()?;

    let (onion_service, request_stream) = tor_client
        .launch_onion_service(svc_config)
        .context("Failed to launch onion service")?;

    // Wait for onion address
    let onion_address = loop {
        if let Some(addr) = onion_service.onion_address() {
            break addr;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    };

    eprintln!("[2/3] ✓ Hidden service launched");
    eprintln!();

    // Print the onion address (this can be captured by scripts)
    eprintln!("========================================");
    eprintln!("ONION_ADDRESS={}", onion_address.display_unredacted());
    eprintln!("========================================");
    eprintln!();

    // Also print it to stdout for easy capture
    println!("{}", onion_address.display_unredacted());

    eprintln!("[3/3] Waiting for connections on port 9999...");
    eprintln!();
    eprintln!("Server is ready! Clients can connect to:");
    eprintln!("  {}:9999", onion_address.display_unredacted());
    eprintln!();
    eprintln!("Press Ctrl+C to shut down");
    eprintln!();

    // Set up client tracking
    let clients: ClientMap = Arc::new(Mutex::new(HashMap::new()));
    let next_client_id = Arc::new(Mutex::new(1));

    // Handle incoming requests
    let stream_requests = handle_rend_requests(request_stream);
    tokio::pin!(stream_requests);

    while let Some(stream_request) = stream_requests.next().await {
        let clients_clone = Arc::clone(&clients);
        let next_id_clone = Arc::clone(&next_client_id);

        tokio::spawn(async move {
            if let Err(e) = handle_stream_request(stream_request, clients_clone, next_id_clone).await {
                eprintln!("[Server] Error handling stream: {}", e);
            }
        });
    }

    eprintln!();
    eprintln!("Server shutting down...");
    Ok(())
}
