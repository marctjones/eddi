// Message broker for broadcasting and routing

use crate::msgserver::client::ClientManager;
use crate::msgserver::message::{MessageQueue, ProtocolMessage};
use crate::msgserver::storage::{ClientStatus, StateManager};
use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;

/// Handle for communicating with the broker
#[derive(Clone)]
pub struct BrokerHandle {
    pub tx: mpsc::UnboundedSender<BrokerCommand>,
}

impl BrokerHandle {
    pub fn new(tx: mpsc::UnboundedSender<BrokerCommand>) -> Self {
        Self { tx }
    }

    /// Send a command to the broker
    pub fn send_command(&self, cmd: BrokerCommand) -> Result<()> {
        self.tx
            .send(cmd)
            .context("Failed to send command to broker")
    }

    /// Get the command sender
    pub fn get_sender(&self) -> mpsc::UnboundedSender<BrokerCommand> {
        self.tx.clone()
    }
}

/// Commands that can be sent to the broker
#[derive(Debug)]
pub enum BrokerCommand {
    /// Client sent a message
    ClientMessage {
        client_id: String,
        message: ProtocolMessage,
    },
    /// Client disconnected
    ClientDisconnected { client_id: String },
    /// Shutdown the broker
    Shutdown,
}

/// Message broker that handles routing and broadcasting
pub struct MessageBroker {
    queue: Arc<MessageQueue>,
    client_manager: Arc<ClientManager>,
    state_manager: Option<Arc<StateManager>>,
    server_id: Option<String>,
    rx: mpsc::UnboundedReceiver<BrokerCommand>,
}

impl MessageBroker {
    /// Create a new message broker
    pub fn new(
        ttl: Duration,
        max_queue_size: usize,
        state_manager: Option<Arc<StateManager>>,
        server_id: Option<String>,
    ) -> (Self, BrokerHandle) {
        let (tx, rx) = mpsc::unbounded_channel();

        let broker = Self {
            queue: Arc::new(MessageQueue::new(ttl, max_queue_size)),
            client_manager: Arc::new(ClientManager::new()),
            state_manager,
            server_id,
            rx,
        };

        let handle = BrokerHandle::new(tx);

        (broker, handle)
    }

    /// Get the client manager
    pub fn client_manager(&self) -> Arc<ClientManager> {
        self.client_manager.clone()
    }

    /// Get the message queue
    pub fn message_queue(&self) -> Arc<MessageQueue> {
        self.queue.clone()
    }

    /// Run the broker event loop
    pub async fn run(mut self) {
        tracing::info!("Message broker started");

        // Start cleanup task
        let queue = self.queue.clone();
        queue.start_cleanup_task(Duration::from_secs(30));

        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                BrokerCommand::ClientMessage { client_id, message } => {
                    if let Err(e) = self.handle_client_message(&client_id, message).await {
                        tracing::error!("Error handling client message: {}", e);
                    }
                }
                BrokerCommand::ClientDisconnected { client_id } => {
                    tracing::info!("Client {} disconnected", client_id);
                    self.client_manager.remove_client(&client_id).await;
                }
                BrokerCommand::Shutdown => {
                    tracing::info!("Broker shutting down");
                    break;
                }
            }
        }

        tracing::info!("Message broker stopped");
    }

    /// Handle a message from a client
    async fn handle_client_message(
        &self,
        client_id: &str,
        message: ProtocolMessage,
    ) -> Result<()> {
        match message {
            ProtocolMessage::Auth { code, client_id: provided_client_id } => {
                self.handle_auth(client_id, &code, &provided_client_id).await?;
            }
            ProtocolMessage::Send { content } => {
                self.handle_send(client_id, content).await?;
            }
            ProtocolMessage::Receive { since } => {
                self.handle_receive(client_id, since).await?;
            }
            ProtocolMessage::Ping => {
                self.send_to_client(client_id, ProtocolMessage::Pong).await?;
            }
            _ => {
                tracing::warn!("Unexpected message type from client {}", client_id);
            }
        }

        Ok(())
    }

    /// Handle authentication request
    async fn handle_auth(
        &self,
        client_id: &str,
        code: &str,
        provided_client_id: &str,
    ) -> Result<()> {
        // Validate the authentication code
        if let Some(state_manager) = &self.state_manager {
            if let Some(client_config) = state_manager.get_client_by_code(code)? {
                // Check if this code is for this server
                if let Some(server_id) = &self.server_id {
                    if &client_config.server_id != server_id {
                        self.send_auth_response(client_id, false, "Invalid code").await?;
                        return Ok(());
                    }
                }

                // Mark client as authenticated
                self.client_manager.authenticate_client(client_id).await?;

                // Update state
                state_manager.update_client_status(&client_config.id, ClientStatus::Connected)?;

                self.send_auth_response(client_id, true, "Authenticated").await?;

                tracing::info!(
                    "Client {} authenticated with code {}",
                    provided_client_id,
                    code
                );
            } else {
                self.send_auth_response(client_id, false, "Invalid code").await?;
            }
        } else {
            // No state manager, accept all connections (for testing)
            self.client_manager.authenticate_client(client_id).await?;
            self.send_auth_response(client_id, true, "Authenticated").await?;
        }

        Ok(())
    }

    /// Send authentication response
    async fn send_auth_response(
        &self,
        client_id: &str,
        success: bool,
        message: &str,
    ) -> Result<()> {
        let response = ProtocolMessage::AuthResponse {
            success,
            message: message.to_string(),
            server_id: self.server_id.clone(),
        };

        self.send_to_client(client_id, response).await
    }

    /// Handle send message request
    async fn handle_send(&self, client_id: &str, content: String) -> Result<()> {
        // Add message to queue
        let message = self.queue.push(client_id.to_string(), content).await;

        tracing::info!("Message {} from {} queued", message.id, client_id);

        // Broadcast to all authenticated clients
        self.client_manager.broadcast(message).await;

        Ok(())
    }

    /// Handle receive request
    async fn handle_receive(&self, client_id: &str, since: Option<SystemTime>) -> Result<()> {
        let messages = if let Some(since_time) = since {
            self.queue.get_since(since_time).await
        } else {
            self.queue.get_all().await
        };

        let response = ProtocolMessage::ReceiveResponse { messages };

        self.send_to_client(client_id, response).await
    }

    /// Send a message to a specific client
    async fn send_to_client(&self, client_id: &str, message: ProtocolMessage) -> Result<()> {
        let clients = self.client_manager.clients.read().await;

        if let Some(client) = clients.get(client_id) {
            client.send(message)?;
        } else {
            anyhow::bail!("Client not found: {}", client_id);
        }

        Ok(())
    }
}

/// Fortress-specific broker with access token validation
pub struct FortressBroker {
    inner: MessageBroker,
    valid_tokens: Arc<tokio::sync::RwLock<std::collections::HashSet<String>>>,
}

impl FortressBroker {
    /// Create a new fortress broker
    pub fn new(
        ttl: Duration,
        max_queue_size: usize,
        state_manager: Arc<StateManager>,
        server_id: String,
    ) -> (Self, BrokerHandle) {
        let (inner, handle) = MessageBroker::new(
            ttl,
            max_queue_size,
            Some(state_manager),
            Some(server_id),
        );

        let fortress = Self {
            inner,
            valid_tokens: Arc::new(tokio::sync::RwLock::new(std::collections::HashSet::new())),
        };

        (fortress, handle)
    }

    /// Add a valid access token
    pub async fn add_token(&self, token: String) {
        let mut tokens = self.valid_tokens.write().await;
        tokens.insert(token);
    }

    /// Remove a token (revoke access)
    pub async fn revoke_token(&self, token: &str) {
        let mut tokens = self.valid_tokens.write().await;
        tokens.remove(token);
    }

    /// Validate a token
    pub async fn validate_token(&self, token: &str) -> bool {
        let tokens = self.valid_tokens.read().await;
        tokens.contains(token)
    }

    /// Run the fortress broker
    pub async fn run(self) {
        self.inner.run().await;
    }

    /// Get client manager
    pub fn client_manager(&self) -> Arc<ClientManager> {
        self.inner.client_manager()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_broker_message_flow() {
        let (broker, handle) = MessageBroker::new(Duration::from_secs(60), 100, None, None);

        // Spawn broker
        tokio::spawn(async move {
            broker.run().await;
        });

        // Send a command
        handle
            .send_command(BrokerCommand::ClientMessage {
                client_id: "test".to_string(),
                message: ProtocolMessage::Ping,
            })
            .unwrap();

        // Wait a bit
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Shutdown
        handle.send_command(BrokerCommand::Shutdown).unwrap();
    }
}
