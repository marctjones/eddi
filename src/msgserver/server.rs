// Server instances for Fortress and Broker

use crate::msgserver::broker::{BrokerCommand, BrokerHandle, FortressBroker, MessageBroker};
use crate::msgserver::client::{handle_client_stream, ClientConnection, ClientManager};
use crate::msgserver::storage::{ServerConfig, ServerStatus, StateManager};
use crate::msgserver::tor::TorManager;
use crate::msgserver::cli::MsgSrvCli;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use futures::StreamExt;
use tor_hsservice::StreamRequest;
use tor_proto::client::stream::IncomingStreamRequest;
use tor_cell::relaycell::msg::Connected;

/// A running server instance (Fortress or Broker)
pub struct ServerInstance {
    config: ServerConfig,
    broker_handle: BrokerHandle,
    shutdown_tx: mpsc::UnboundedSender<()>,
}

impl ServerInstance {
    /// Create a new eddi messaging server instance (emsgsrv)
    pub async fn new_server(
        name: String,
        socket_path: PathBuf,
        ttl_minutes: u64,
        state_manager: Arc<StateManager>,
        use_tor: bool,
    ) -> Result<Self> {
        let server_id = Uuid::new_v4().to_string();

        // Initialize Tor if requested
        let (onion_address, tor_stream) = if use_tor {
            tracing::info!("🧅 Initializing Tor for server: {}", name);

            let key_dir = MsgSrvCli::state_dir().join("tor-keys");
            let tor = Arc::new(TorManager::new(key_dir).await?);

            let (addr, stream) = tor.create_onion_service(&name).await?;

            tracing::info!("🧅 Server onion address: {}", addr);
            (Some(addr), Some(stream))
        } else {
            tracing::info!("📍 Server will use Unix sockets only (local access)");
            (None, None)
        };

        let config = ServerConfig {
            id: server_id.clone(),
            name: name.clone(),
            socket_path: socket_path.clone(),
            created_at: SystemTime::now(),
            ttl_minutes,
            onion_address: onion_address.clone(),
            status: ServerStatus::Running,
        };

        // Save to state
        state_manager.create_server(config.clone())?;

        // Update state with onion address if we have one
        if let Some(ref addr) = onion_address {
            state_manager.update_server_onion(&server_id, addr)?;
        }

        // Create broker
        let (broker, handle) = FortressBroker::new(
            Duration::from_secs(ttl_minutes * 60),
            1000, // Max queue size
            state_manager.clone(),
            server_id.clone(),
        );

        let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel();

        // Get client manager before moving broker
        let client_manager = broker.client_manager();

        // Spawn broker task
        tokio::spawn(async move {
            broker.run().await;
        });

        // Spawn Unix socket listener
        let broker_tx = handle.clone();
        let client_manager_clone = client_manager.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::run_listener(
                socket_path,
                broker_tx,
                client_manager,
                &mut shutdown_rx,
            )
            .await
            {
                tracing::error!("Unix socket listener error: {}", e);
            }
        });

        // Spawn Tor onion service listener (if enabled)
        if let Some(mut stream) = tor_stream {
            let broker_tx_tor = handle.clone();
            let socket_path_tor = config.socket_path.clone();

            tokio::spawn(async move {
                tracing::info!("🧅 Starting onion service listener");

                while let Some(request) = stream.next().await {
                    let socket_path_clone = socket_path_tor.clone();
                    let broker_handle_clone = broker_tx_tor.clone();
                    let client_mgr = client_manager_clone.clone();

                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_onion_request(
                            request,
                            socket_path_clone,
                            broker_handle_clone,
                            client_mgr,
                        ).await {
                            tracing::error!("Error handling onion request: {}", e);
                        }
                    });
                }

                tracing::info!("🧅 Onion service listener stopped");
            });
        }

        Ok(Self {
            config,
            broker_handle: handle,
            shutdown_tx,
        })
    }

    /// Handle an onion service request
    async fn handle_onion_request(
        stream_request: StreamRequest,
        socket_path: PathBuf,
        _broker_handle: BrokerHandle,
        _client_manager: Arc<ClientManager>,
    ) -> Result<()> {
        // Check the stream request type and accept only Begin requests
        match stream_request.request() {
            IncomingStreamRequest::Begin(begin) => {
                let port = begin.port();
                tracing::debug!("🧅 Onion connection request on port {}", port);

                // Accept the connection
                let mut onion_stream = stream_request
                    .accept(Connected::new_empty())
                    .await
                    .context("Failed to accept onion stream")?;

                tracing::debug!("🧅 Accepted onion connection, connecting to local Unix socket");

                // Connect to local Unix socket to communicate with the broker
                let mut unix_stream = UnixStream::connect(&socket_path)
                    .await
                    .context("Failed to connect to Unix socket")?;

                tracing::debug!("✓ Connected to Unix socket, proxying data");

                // Proxy bidirectionally between onion stream and Unix socket
                match tokio::io::copy_bidirectional(&mut onion_stream, &mut unix_stream).await {
                    Ok((to_unix, from_unix)) => {
                        tracing::debug!(
                            "🧅 Onion connection closed. Transferred: {} bytes to UDS, {} bytes from UDS",
                            to_unix,
                            from_unix
                        );
                    }
                    Err(e) => {
                        tracing::error!("Error proxying onion stream: {}", e);
                    }
                }

                Ok(())
            }
            _ => {
                tracing::warn!("🧅 Received non-Begin stream request, ignoring");
                Ok(())
            }
        }
    }

    /// Create a new Broker server instance (ephemeral)
    pub async fn new_broker(
        fortress_id: String,
        socket_path: PathBuf,
        timeout: Duration,
    ) -> Result<Self> {
        let server_id = Uuid::new_v4().to_string();

        let config = ServerConfig {
            id: server_id.clone(),
            name: format!("broker-{}", fortress_id),
            socket_path: socket_path.clone(),
            created_at: SystemTime::now(),
            ttl_minutes: 5,
            onion_address: None,
            status: ServerStatus::Running,
        };

        // Create a simple broker without state manager
        let (broker, handle) = MessageBroker::new(
            Duration::from_secs(300),
            10,
            None,
            Some(server_id.clone()),
        );

        let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel();

        // Get client manager before moving broker
        let client_manager = broker.client_manager();

        // Spawn broker task
        tokio::spawn(async move {
            broker.run().await;
        });

        // Spawn listener task with timeout
        let broker_tx = handle.clone();
        let shutdown_tx_clone = shutdown_tx.clone();
        tokio::spawn(async move {
            // Auto-shutdown after timeout
            tokio::spawn(async move {
                tokio::time::sleep(timeout).await;
                tracing::info!("Broker timeout reached, shutting down");
                let _ = shutdown_tx_clone.send(());
            });

            if let Err(e) = Self::run_listener(
                socket_path,
                broker_tx,
                client_manager,
                &mut shutdown_rx,
            )
            .await
            {
                tracing::error!("Broker listener error: {}", e);
            }
        });

        Ok(Self {
            config,
            broker_handle: handle,
            shutdown_tx,
        })
    }

    /// Run the Unix socket listener
    async fn run_listener(
        socket_path: PathBuf,
        broker_handle: BrokerHandle,
        client_manager: Arc<ClientManager>,
        shutdown_rx: &mut mpsc::UnboundedReceiver<()>,
    ) -> Result<()> {
        // Remove old socket if exists
        let _ = std::fs::remove_file(&socket_path);

        let listener = UnixListener::bind(&socket_path)
            .context("Failed to bind Unix socket")?;

        tracing::info!("Listening on {:?}", socket_path);

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, _addr)) => {
                            Self::handle_connection(
                                stream,
                                broker_handle.clone(),
                                client_manager.clone(),
                            ).await;
                        }
                        Err(e) => {
                            tracing::error!("Accept error: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("Listener shutting down");
                    break;
                }
            }
        }

        // Cleanup
        let _ = std::fs::remove_file(&socket_path);

        Ok(())
    }

    /// Handle a new client connection
    async fn handle_connection(
        stream: UnixStream,
        broker_handle: BrokerHandle,
        client_manager: Arc<ClientManager>,
    ) {
        // Create channels for bidirectional communication
        let (outgoing_tx, outgoing_rx) = mpsc::unbounded_channel();
        let (incoming_tx, mut incoming_rx) = mpsc::unbounded_channel();

        // Create client connection
        let client = ClientConnection::new(outgoing_tx.clone());
        let client_id = client_manager.add_client(client).await;

        // Clone for the message handler
        let broker_handle_clone = broker_handle.clone();
        let client_id_clone = client_id.clone();

        // Spawn task to forward incoming messages to broker
        tokio::spawn(async move {
            while let Some(msg) = incoming_rx.recv().await {
                if broker_handle_clone
                    .send_command(BrokerCommand::ClientMessage {
                        client_id: client_id_clone.clone(),
                        message: msg,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });

        // Handle client stream
        if let Err(e) = handle_client_stream(stream, outgoing_rx, incoming_tx).await {
            tracing::error!("Client stream error: {}", e);
        }

        // Notify broker of disconnect
        let _ = broker_handle.send_command(BrokerCommand::ClientDisconnected {
            client_id: client_id.clone(),
        });
    }

    /// Get server configuration
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Shutdown the server
    pub async fn shutdown(&self) -> Result<()> {
        self.broker_handle
            .send_command(BrokerCommand::Shutdown)
            .context("Failed to send shutdown command")?;

        self.shutdown_tx
            .send(())
            .context("Failed to send listener shutdown")?;

        Ok(())
    }
}

/// Manages multiple server instances
pub struct ServerManager {
    servers: Arc<RwLock<std::collections::HashMap<String, Arc<ServerInstance>>>>,
    state_manager: Arc<StateManager>,
}

impl ServerManager {
    /// Create a new server manager
    pub fn new(state_manager: Arc<StateManager>) -> Self {
        Self {
            servers: Arc::new(RwLock::new(std::collections::HashMap::new())),
            state_manager,
        }
    }

    /// Create a new eddi messaging server
    pub async fn create_server(
        &self,
        name: String,
        ttl_minutes: u64,
        use_tor: bool,
    ) -> Result<Arc<ServerInstance>> {
        // Check if server with this name already exists
        if self.state_manager.get_server(&name)?.is_some() {
            anyhow::bail!("Server with name '{}' already exists", name);
        }

        let socket_path = self.get_socket_path(&name);

        let instance = ServerInstance::new_server(
            name.clone(),
            socket_path,
            ttl_minutes,
            self.state_manager.clone(),
            use_tor,
        )
        .await?;

        let instance = Arc::new(instance);

        let mut servers = self.servers.write().await;
        servers.insert(name, instance.clone());

        Ok(instance)
    }

    /// Create a new broker
    pub async fn create_broker(
        &self,
        fortress_name: String,
        timeout: Duration,
    ) -> Result<Arc<ServerInstance>> {
        // Verify fortress exists
        let fortress = self.state_manager.get_server(&fortress_name)?
            .context("Fortress not found")?;

        let broker_name = format!("broker-{}", Uuid::new_v4());
        let socket_path = self.get_socket_path(&broker_name);

        let instance = ServerInstance::new_broker(
            fortress.id,
            socket_path,
            timeout,
        )
        .await?;

        let instance = Arc::new(instance);

        let mut servers = self.servers.write().await;
        servers.insert(broker_name, instance.clone());

        Ok(instance)
    }

    /// Get a server by name
    pub async fn get_server(&self, name: &str) -> Option<Arc<ServerInstance>> {
        let servers = self.servers.read().await;
        servers.get(name).cloned()
    }

    /// List all running servers
    pub async fn list_servers(&self) -> Vec<Arc<ServerInstance>> {
        let servers = self.servers.read().await;
        servers.values().cloned().collect()
    }

    /// Stop a server
    pub async fn stop_server(&self, name: &str) -> Result<()> {
        let instance = {
            let mut servers = self.servers.write().await;
            servers.remove(name)
        };

        if let Some(instance) = instance {
            instance.shutdown().await?;

            // Update state
            self.state_manager.update_server_status(
                &instance.config.id,
                ServerStatus::Stopped,
            )?;

            Ok(())
        } else {
            anyhow::bail!("Server '{}' not found", name)
        }
    }

    /// Get socket path for a server
    fn get_socket_path(&self, name: &str) -> PathBuf {
        PathBuf::from(format!("/tmp/eddi-msgsrv-{}.sock", name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_server_manager() {
        let dir = tempdir().unwrap();
        let state_manager = Arc::new(StateManager::new(dir.path()).unwrap());
        let manager = ServerManager::new(state_manager);

        // Create server (without Tor for testing)
        let server = manager
            .create_server("test-server".to_string(), 5, false)
            .await
            .unwrap();

        assert_eq!(server.config().name, "test-server");

        // List servers
        let servers = manager.list_servers().await;
        assert_eq!(servers.len(), 1);

        // Stop server
        manager.stop_server("test-server").await.unwrap();

        let servers = manager.list_servers().await;
        assert_eq!(servers.len(), 0);
    }
}
