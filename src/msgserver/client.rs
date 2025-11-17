// Client connection management

use crate::msgserver::message::{Message, ProtocolMessage};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

/// A connected client
pub struct ClientConnection {
    pub id: String,
    pub authenticated: bool,
    tx: mpsc::UnboundedSender<ProtocolMessage>,
}

impl ClientConnection {
    /// Create a new client connection
    pub fn new(tx: mpsc::UnboundedSender<ProtocolMessage>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            authenticated: false,
            tx,
        }
    }

    /// Send a message to the client
    pub fn send(&self, message: ProtocolMessage) -> Result<()> {
        self.tx
            .send(message)
            .context("Failed to send message to client")
    }

    /// Broadcast a message to the client
    pub fn broadcast(&self, message: Message) -> Result<()> {
        self.send(ProtocolMessage::Broadcast { message })
    }
}

/// Manages all connected clients
pub struct ClientManager {
    pub clients: Arc<RwLock<HashMap<String, ClientConnection>>>,
}

impl ClientManager {
    /// Create a new client manager
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a new client
    pub async fn add_client(&self, client: ClientConnection) -> String {
        let id = client.id.clone();
        let mut clients = self.clients.write().await;
        clients.insert(id.clone(), client);
        tracing::info!("Client {} connected", id);
        id
    }

    /// Remove a client
    pub async fn remove_client(&self, id: &str) {
        let mut clients = self.clients.write().await;
        clients.remove(id);
        tracing::info!("Client {} disconnected", id);
    }

    /// Mark client as authenticated
    pub async fn authenticate_client(&self, id: &str) -> Result<()> {
        let mut clients = self.clients.write().await;
        if let Some(client) = clients.get_mut(id) {
            client.authenticated = true;
            tracing::info!("Client {} authenticated", id);
            Ok(())
        } else {
            anyhow::bail!("Client not found: {}", id)
        }
    }

    /// Get authenticated client IDs
    pub async fn get_authenticated_clients(&self) -> Vec<String> {
        let clients = self.clients.read().await;
        clients
            .iter()
            .filter(|(_, c)| c.authenticated)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Broadcast a message to all authenticated clients
    pub async fn broadcast(&self, message: Message) {
        let clients = self.clients.read().await;

        let mut failed = Vec::new();

        for (id, client) in clients.iter() {
            if !client.authenticated {
                continue;
            }

            if let Err(e) = client.broadcast(message.clone()) {
                tracing::warn!("Failed to send to client {}: {}", id, e);
                failed.push(id.clone());
            }
        }

        drop(clients);

        // Remove failed clients
        if !failed.is_empty() {
            let mut clients = self.clients.write().await;
            for id in failed {
                clients.remove(&id);
                tracing::info!("Removed failed client {}", id);
            }
        }
    }

    /// Get number of connected clients
    pub async fn client_count(&self) -> usize {
        let clients = self.clients.read().await;
        clients.len()
    }

    /// Get number of authenticated clients
    pub async fn authenticated_count(&self) -> usize {
        let clients = self.clients.read().await;
        clients.iter().filter(|(_, c)| c.authenticated).count()
    }
}

impl Default for ClientManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle a client connection stream
/// Returns a receiver for outgoing messages to the client
pub async fn handle_client_stream(
    stream: UnixStream,
    mut outgoing_rx: mpsc::UnboundedReceiver<ProtocolMessage>,
    incoming_tx: mpsc::UnboundedSender<ProtocolMessage>,
) -> Result<()> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    // Spawn task to handle outgoing messages
    let write_task = tokio::spawn(async move {
        while let Some(msg) = outgoing_rx.recv().await {
            if let Ok(bytes) = msg.to_bytes() {
                if write_half.write_all(&bytes).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming messages
    let mut line = String::new();
    loop {
        line.clear();

        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                match ProtocolMessage::from_bytes(trimmed.as_bytes()) {
                    Ok(msg) => {
                        // Send to incoming channel
                        if incoming_tx.send(msg).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse message: {}", e);
                    }
                }
            }
            Err(e) => {
                tracing::error!("Error reading from client: {}", e);
                break;
            }
        }
    }

    write_task.abort();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_manager() {
        let manager = ClientManager::new();

        let (tx, _rx) = mpsc::unbounded_channel();
        let client = ClientConnection::new(tx);
        let id = client.id.clone();

        manager.add_client(client).await;

        assert_eq!(manager.client_count().await, 1);
        assert_eq!(manager.authenticated_count().await, 0);

        manager.authenticate_client(&id).await.unwrap();
        assert_eq!(manager.authenticated_count().await, 1);

        manager.remove_client(&id).await;
        assert_eq!(manager.client_count().await, 0);
    }
}
