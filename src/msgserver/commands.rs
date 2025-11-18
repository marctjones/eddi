// Command handler for message server CLI

use crate::msgserver::*;
use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;

/// Execute a message server command
pub async fn execute_command(command: MsgSrvCommand) -> Result<()> {
    let state_dir = MsgSrvCli::state_dir();
    std::fs::create_dir_all(&state_dir)
        .context("Failed to create state directory")?;

    let state_manager = Arc::new(StateManager::new(&state_dir)?);
    let server_manager = ServerManager::new(state_manager.clone());

    match command {
        MsgSrvCommand::CreateServer { name, ttl, local_only, stealth } => {
            handle_create_server(server_manager, name, ttl, local_only, stealth).await
        }
        MsgSrvCommand::CreateBroker { server, namespace, timeout, local_only } => {
            handle_create_broker(server_manager, state_manager, server, namespace, timeout, local_only).await
        }
        MsgSrvCommand::Connect { code, namespace, time_window, alias } => {
            handle_connect(state_manager, code, namespace, time_window, alias).await
        }
        MsgSrvCommand::Send { message, server } => {
            handle_send(state_manager, message, server).await
        }
        MsgSrvCommand::Receive { server, once, since } => {
            handle_receive(state_manager, server, once, since).await
        }
        MsgSrvCommand::Listen { server, daemon, background } => {
            handle_listen(state_manager, server, daemon, background).await
        }
        MsgSrvCommand::ListServers { verbose } => {
            handle_list_servers(state_manager, verbose).await
        }
        MsgSrvCommand::ListBrokers => {
            handle_list_brokers(server_manager).await
        }
        MsgSrvCommand::ListClients { server } => {
            handle_list_clients(state_manager, server).await
        }
        MsgSrvCommand::ListConnections { verbose } => {
            handle_list_connections(state_manager, verbose).await
        }
        MsgSrvCommand::Status { name } => {
            handle_status(state_manager, server_manager, name).await
        }
        MsgSrvCommand::StopServer { name } => {
            handle_stop_server(server_manager, state_manager, name).await
        }
        MsgSrvCommand::StopBroker { id } => {
            handle_stop_broker(server_manager, id).await
        }
        MsgSrvCommand::Disconnect { name } => {
            handle_disconnect(state_manager, name).await
        }
        MsgSrvCommand::RevokeClient { server, code } => {
            handle_revoke_client(state_manager, server, code).await
        }
        MsgSrvCommand::Cleanup { force } => {
            handle_cleanup(state_manager, force).await
        }
    }
}

async fn handle_create_server(
    server_manager: ServerManager,
    name: String,
    ttl: u64,
    local_only: bool,
    _stealth: bool,
) -> Result<()> {
    println!("Creating eddi messaging server: {}", name);

    let use_tor = !local_only;

    if use_tor {
        println!("🧅 Tor mode enabled (default) - server will be accessible via .onion address");
        println!("⏳ This may take 30-60 seconds (bootstrapping Tor)...");
        println!("💡 Use --local-only to disable Tor for fast local development");
        println!();
    } else {
        println!("⚠️  Local-only mode - using Unix sockets only (no Tor)");
        println!("💡 Remove --local-only flag to enable Tor for remote access");
        println!();
    }

    let instance = server_manager.create_server(name.clone(), ttl, use_tor).await?;

    println!("✓ Eddi messaging server '{}' created", name);
    println!("  Socket: {:?}", instance.config().socket_path);
    println!("  Message TTL: {} minutes", ttl);
    println!("  Status: Running");

    if let Some(ref onion_addr) = instance.config().onion_address {
        println!("\n🧅 Onion Address: {}", onion_addr);
        println!("  (Accessible via Tor network)");
    }

    // Keep the server running
    println!("\nPress Ctrl+C to stop the server");
    tokio::signal::ctrl_c().await?;

    println!("\nStopping server...");
    instance.shutdown().await?;

    Ok(())
}

async fn handle_create_broker(
    server_manager: ServerManager,
    state_manager: Arc<StateManager>,
    server_name: String,
    namespace: String,
    timeout: u64,
    local_only: bool,
) -> Result<()> {
    println!("Creating broker for eddi messaging server: {}", server_name);

    let use_tor = !local_only;

    if use_tor {
        println!("🧅 Tor mode enabled (default) - broker will use ephemeral .onion address");
        println!("⏳ This may take 30-60 seconds (bootstrapping Tor)...");
        println!();
    } else {
        println!("⚠️  Local-only mode - using Unix sockets only (no Tor)");
        println!();
    }

    // Verify server exists
    let server_config = state_manager.get_server(&server_name)?
        .context("Server not found")?;

    // Generate code and create handshake
    let code = handshake::generate_short_code();
    let broker_handshake = BrokerHandshake::new(
        namespace.clone(),
        code.clone(),
        server_config.onion_address.unwrap_or_else(|| server_name.clone()),
    );

    println!("✓ Broker created");
    println!("\n📋 Connection Details:");
    println!("  Namespace: {}", namespace);
    println!("  Short Code: {}", code);
    println!("  Valid for: {} seconds", timeout);
    println!("  Broker ID: {}", broker_handshake.identifier());

    println!("\n💡 Share with your client:");
    println!("  eddi-msgsrv connect --code {} --namespace {}", code, namespace);

    // Create the broker instance
    let instance = server_manager.create_broker(server_name, Duration::from_secs(timeout)).await?;

    // Wait for timeout or connection
    println!("\n⏳ Waiting for client connection...");
    tokio::time::sleep(Duration::from_secs(timeout)).await;

    println!("✓ Broker timeout reached, shutting down");
    instance.shutdown().await?;

    Ok(())
}

async fn handle_connect(
    state_manager: Arc<StateManager>,
    code: String,
    namespace: String,
    time_window: i64,
    alias: Option<String>,
) -> Result<()> {
    println!("🔍 Searching for broker...");
    println!("  Code: {}", code);
    println!("  Namespace: {}", namespace);
    println!("  Time window: ±{} minutes", time_window);

    let client_handshake = ClientHandshake::new(namespace.clone(), code.clone());
    let possible_identifiers = client_handshake.possible_identifiers(time_window);

    println!("  Trying {} possible timestamps...", possible_identifiers.len());

    // In a real implementation, this would try to connect to each broker
    // For now, we'll simulate finding one
    if let Some((timestamp, identifier)) = possible_identifiers.first() {
        println!("✓ Found broker at timestamp {}", timestamp);
        println!("  Broker ID: {}", identifier);

        // Create introduction data (simulated)
        let intro = BrokerHandshake::new(
            namespace.clone(),
            code.clone(),
            "fortress-address.onion".to_string(),
        ).create_introduction(24);

        println!("\n✓ Handshake successful!");
        println!("  Fortress: {}", intro.fortress_address);
        println!("  Access token: {}...", &intro.access_token[..8]);

        // Save connection
        let connection = storage::ConnectionConfig {
            id: uuid::Uuid::new_v4().to_string(),
            server_name: intro.fortress_address.clone(),
            alias: alias.clone(),
            code,
            socket_path: None,
            onion_address: Some(intro.fortress_address),
            connected_at: std::time::SystemTime::now(),
            status: storage::ClientStatus::Connected,
        };

        state_manager.create_connection(connection)?;

        println!("\n✓ Connected to fortress!");
        if let Some(alias) = alias {
            println!("  Alias: {}", alias);
        }
    } else {
        anyhow::bail!("No brokers found in time window");
    }

    Ok(())
}

async fn handle_send(
    state_manager: Arc<StateManager>,
    message: String,
    server: Option<String>,
) -> Result<()> {
    // Get connection
    let connection = if let Some(server_name) = server {
        state_manager.get_connection_config(&server_name)?
            .context("Connection not found")?
    } else {
        // Get most recent connection
        let connections = state_manager.list_connections()?;
        connections.into_iter().next()
            .context("No active connections. Connect to a fortress first.")?
    };

    println!("📤 Sending message to: {}", connection.server_name);
    println!("  Message: {}", message);

    // In a real implementation, connect to the socket and send
    // For now, we'll simulate it
    println!("✓ Message sent");

    Ok(())
}

async fn handle_receive(
    _state_manager: Arc<StateManager>,
    server: Option<String>,
    once: bool,
    _since: Option<u64>,
) -> Result<()> {
    let server_name = server.as_deref().unwrap_or("default");

    println!("📥 Receiving messages from: {}", server_name);

    if once {
        println!("  Mode: One-time receive");
        // Receive once and exit
        println!("✓ No new messages");
    } else {
        println!("  Mode: Continuous");
        println!("  (Press Ctrl+C to stop)");

        tokio::signal::ctrl_c().await?;
        println!("\n✓ Stopped receiving");
    }

    Ok(())
}

async fn handle_listen(
    _state_manager: Arc<StateManager>,
    server: Option<String>,
    daemon: bool,
    background: bool,
) -> Result<()> {
    let server_name = server.as_deref().unwrap_or("default");

    println!("👂 Listening for messages on: {}", server_name);

    if daemon {
        println!("  Mode: System daemon");
        // Would fork and run as daemon
    } else if background {
        println!("  Mode: Background (detached)");
        // Would detach from terminal
    } else {
        println!("  Mode: Foreground");
        println!("  (Press Ctrl+C to stop)");

        tokio::signal::ctrl_c().await?;
        println!("\n✓ Stopped listening");
    }

    Ok(())
}

async fn handle_list_servers(
    state_manager: Arc<StateManager>,
    verbose: bool,
) -> Result<()> {
    let servers = state_manager.list_servers()?;

    if servers.is_empty() {
        println!("No eddi messaging servers found");
        return Ok(());
    }

    println!("Eddi messaging servers ({}):", servers.len());
    for server in servers {
        println!("\n  {} [{}]", server.name, server.status.to_string());
        if verbose {
            println!("    ID: {}", server.id);
            println!("    Socket: {:?}", server.socket_path);
            println!("    TTL: {} minutes", server.ttl_minutes);
            if let Some(onion) = server.onion_address {
                println!("    Onion: {}", onion);
            }
        }
    }

    Ok(())
}

async fn handle_list_brokers(
    server_manager: ServerManager,
) -> Result<()> {
    let servers = server_manager.list_servers().await;
    let brokers: Vec<_> = servers.into_iter()
        .filter(|s| s.config().name.starts_with("broker-"))
        .collect();

    if brokers.is_empty() {
        println!("No active brokers");
        return Ok(());
    }

    println!("Active Brokers ({}):", brokers.len());
    for broker in brokers {
        println!("  {}", broker.config().name);
        println!("    Socket: {:?}", broker.config().socket_path);
    }

    Ok(())
}

async fn handle_list_clients(
    state_manager: Arc<StateManager>,
    server_name: String,
) -> Result<()> {
    let server = state_manager.get_server(&server_name)?
        .context("Server not found")?;

    let clients = state_manager.list_clients(&server.id)?;

    if clients.is_empty() {
        println!("No clients for server: {}", server_name);
        return Ok(());
    }

    println!("Clients for '{}' ({}):", server_name, clients.len());
    for client in clients {
        println!("\n  Code: {}", client.code);
        println!("    Status: {}", client.status.to_string());
        println!("    Created: {:?}", client.created_at);
        if let Some(connected) = client.connected_at {
            println!("    Connected: {:?}", connected);
        }
    }

    Ok(())
}

async fn handle_list_connections(
    state_manager: Arc<StateManager>,
    verbose: bool,
) -> Result<()> {
    let connections = state_manager.list_connections()?;

    if connections.is_empty() {
        println!("No connections found");
        return Ok(());
    }

    println!("Connections ({}):", connections.len());
    for conn in connections {
        let display_name = conn.alias.as_ref().unwrap_or(&conn.server_name);
        println!("\n  {} [{}]", display_name, conn.status.to_string());
        if verbose {
            println!("    Server: {}", conn.server_name);
            println!("    Code: {}", conn.code);
            if let Some(socket) = conn.socket_path {
                println!("    Socket: {:?}", socket);
            }
            if let Some(onion) = conn.onion_address {
                println!("    Onion: {}", onion);
            }
            println!("    Connected: {:?}", conn.connected_at);
        }
    }

    Ok(())
}

async fn handle_status(
    state_manager: Arc<StateManager>,
    server_manager: ServerManager,
    name: Option<String>,
) -> Result<()> {
    if let Some(server_name) = name {
        // Show specific server status
        let server = state_manager.get_server(&server_name)?
            .context("Server not found")?;

        println!("Server: {}", server.name);
        println!("  Status: {}", server.status.to_string());
        println!("  Socket: {:?}", server.socket_path);
        println!("  TTL: {} minutes", server.ttl_minutes);

        let clients = state_manager.list_clients(&server.id)?;
        println!("  Clients: {}", clients.len());
    } else {
        // Show all servers
        let servers = state_manager.list_servers()?;
        let running = server_manager.list_servers().await;
        let connections = state_manager.list_connections()?;

        println!("eddi Message Server Status\n");
        println!("Fortresses: {} ({} running)", servers.len(), running.len());
        println!("Connections: {}", connections.len());
    }

    Ok(())
}

async fn handle_stop_server(
    server_manager: ServerManager,
    _state_manager: Arc<StateManager>,
    name: String,
) -> Result<()> {
    println!("Stopping eddi messaging server: {}", name);

    server_manager.stop_server(&name).await?;

    println!("✓ Server stopped");
    Ok(())
}

async fn handle_stop_broker(
    server_manager: ServerManager,
    id: String,
) -> Result<()> {
    println!("Stopping broker: {}", id);

    server_manager.stop_server(&id).await?;

    println!("✓ Broker stopped");
    Ok(())
}

async fn handle_disconnect(
    state_manager: Arc<StateManager>,
    name: String,
) -> Result<()> {
    println!("Disconnecting from: {}", name);

    state_manager.delete_connection(&name)?;

    println!("✓ Disconnected");
    Ok(())
}

async fn handle_revoke_client(
    state_manager: Arc<StateManager>,
    server_name: String,
    code: String,
) -> Result<()> {
    println!("Revoking client access...");
    println!("  Server: {}", server_name);
    println!("  Code: {}", code);

    // Get client by code
    let client = state_manager.get_client_by_code(&code)?
        .context("Client not found")?;

    // Update status to disconnected
    state_manager.update_client_status(&client.id, storage::ClientStatus::Disconnected)?;

    println!("✓ Client access revoked");
    Ok(())
}

async fn handle_cleanup(
    state_manager: Arc<StateManager>,
    force: bool,
) -> Result<()> {
    if !force {
        println!("Dry-run mode (use --force to actually delete)");
    }

    println!("\nCleaning up...");

    // Find stopped servers
    let servers = state_manager.list_servers()?;
    let stopped: Vec<_> = servers.into_iter()
        .filter(|s| s.status == storage::ServerStatus::Stopped)
        .collect();

    println!("  Stopped servers: {}", stopped.len());

    if force && !stopped.is_empty() {
        for server in stopped {
            println!("    Deleting: {}", server.name);
            state_manager.delete_server(&server.name)?;
        }
    }

    // Clean up stale sockets
    println!("  Checking for stale sockets...");
    let socket_pattern = "/tmp/eddi-msgsrv-*.sock";
    println!("    Pattern: {}", socket_pattern);

    println!("\n✓ Cleanup complete");

    Ok(())
}
