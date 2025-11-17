// Integration tests for message server

use eddi::msgserver::*;
use clap::Parser;
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;

#[tokio::test]
async fn test_fortress_creation() {
    let dir = tempdir().unwrap();
    let state_manager = Arc::new(StateManager::new(dir.path()).unwrap());
    let server_manager = ServerManager::new(state_manager.clone());

    // Create a fortress
    let fortress = server_manager
        .create_fortress("test-fortress".to_string(), 5)
        .await
        .unwrap();

    assert_eq!(fortress.config().name, "test-fortress");
    assert_eq!(fortress.config().ttl_minutes, 5);

    // Stop fortress
    server_manager.stop_server("test-fortress").await.unwrap();
}

#[tokio::test]
async fn test_broker_creation() {
    let dir = tempdir().unwrap();
    let state_manager = Arc::new(StateManager::new(dir.path()).unwrap());
    let server_manager = ServerManager::new(state_manager.clone());

    // First create a fortress
    let _fortress = server_manager
        .create_fortress("test-fortress".to_string(), 5)
        .await
        .unwrap();

    // Create a broker
    let broker = server_manager
        .create_broker("test-fortress".to_string(), Duration::from_secs(60))
        .await
        .unwrap();

    assert!(broker.config().name.starts_with("broker-"));

    // Cleanup
    server_manager.stop_server("test-fortress").await.unwrap();
}

#[tokio::test]
async fn test_message_queue_expiration() {
    let queue = Arc::new(MessageQueue::new(Duration::from_millis(100), 10));

    // Add a message
    queue.push("client1".to_string(), "test message".to_string()).await;

    // Message should exist
    let messages = queue.get_all().await;
    assert_eq!(messages.len(), 1);

    // Wait for expiration
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Message should be expired
    let messages = queue.get_all().await;
    assert_eq!(messages.len(), 0);
}

#[tokio::test]
async fn test_client_authentication() {
    let dir = tempdir().unwrap();
    let state_manager = Arc::new(StateManager::new(dir.path()).unwrap());

    // Create server config
    let server_config = ServerConfig {
        id: "test-server".to_string(),
        name: "test".to_string(),
        socket_path: dir.path().join("test.sock"),
        created_at: std::time::SystemTime::now(),
        ttl_minutes: 5,
        onion_address: None,
        status: storage::ServerStatus::Running,
    };

    state_manager.create_server(server_config.clone()).unwrap();

    // Create client code
    let client = state_manager.create_client(&server_config.id).unwrap();
    assert!(!client.code.is_empty());

    // Verify client can be retrieved by code
    let retrieved = state_manager.get_client_by_code(&client.code).unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().code, client.code);
}

#[tokio::test]
async fn test_handshake_code_generation() {
    use handshake::*;

    // Generate short code
    let code = generate_short_code();
    assert_eq!(code.len(), 7); // 6 chars + hyphen
    assert!(code.contains('-'));

    // Generate access token
    let token = generate_access_token();
    assert_eq!(token.len(), 32);

    // Test broker identifier is deterministic
    let id1 = generate_broker_identifier("test@example.com", 1234567890, "ABC-XYZ");
    let id2 = generate_broker_identifier("test@example.com", 1234567890, "ABC-XYZ");
    assert_eq!(id1, id2);

    // Different codes produce different identifiers
    let id3 = generate_broker_identifier("test@example.com", 1234567890, "DEF-123");
    assert_ne!(id1, id3);
}

#[tokio::test]
async fn test_broker_handshake_flow() {
    use handshake::*;

    let broker = BrokerHandshake::new(
        "test@example.com".to_string(),
        "ABC-XYZ".to_string(),
        "test123.onion".to_string(),
    );

    // Validate correct code
    assert!(broker.validate_code("ABC-XYZ"));
    assert!(!broker.validate_code("WRONG"));

    // Create introduction
    let intro = broker.create_introduction(24);
    assert_eq!(intro.fortress_address, "test123.onion");
    assert!(!intro.access_token.is_empty());
    assert!(intro.expires_at > std::time::SystemTime::now());
}

#[tokio::test]
async fn test_client_handshake_time_window() {
    use handshake::*;

    let client = ClientHandshake::new(
        "test@example.com".to_string(),
        "ABC-XYZ".to_string(),
    );

    // Generate possible identifiers within time window
    let identifiers = client.possible_identifiers(2);

    // Should have at least 5 timestamps (±2 minutes at 1-minute intervals)
    assert!(identifiers.len() >= 5);

    // All identifiers should be hex strings
    for (_, id) in &identifiers {
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }
}

#[tokio::test]
async fn test_server_manager_multi_instance() {
    let dir = tempdir().unwrap();
    let state_manager = Arc::new(StateManager::new(dir.path()).unwrap());
    let server_manager = ServerManager::new(state_manager.clone());

    // Create multiple fortresses
    server_manager
        .create_fortress("fortress1".to_string(), 5)
        .await
        .unwrap();

    server_manager
        .create_fortress("fortress2".to_string(), 10)
        .await
        .unwrap();

    // List servers
    let servers = server_manager.list_servers().await;
    assert_eq!(servers.len(), 2);

    // Stop all servers
    server_manager.stop_server("fortress1").await.unwrap();
    server_manager.stop_server("fortress2").await.unwrap();

    let servers = server_manager.list_servers().await;
    assert_eq!(servers.len(), 0);
}

#[test]
fn test_cli_parsing() {
    use cli::*;

    // Test create-fortress command
    let args = vec![
        "msgsrv",
        "create-fortress",
        "--name",
        "test",
        "--ttl",
        "10",
    ];

    let cli = MsgSrvCli::try_parse_from(args);
    assert!(cli.is_ok());

    if let Ok(cli) = cli {
        match cli.command {
            MsgSrvCommand::CreateFortress { name, ttl, .. } => {
                assert_eq!(name, "test");
                assert_eq!(ttl, 10);
            }
            _ => panic!("Wrong command parsed"),
        }
    }
}

#[tokio::test]
async fn test_state_persistence() {
    let dir = tempdir().unwrap();
    let state_manager = StateManager::new(dir.path()).unwrap();

    // Create server
    let server_config = ServerConfig {
        id: "test-server".to_string(),
        name: "test".to_string(),
        socket_path: dir.path().join("test.sock"),
        created_at: std::time::SystemTime::now(),
        ttl_minutes: 5,
        onion_address: Some("test.onion".to_string()),
        status: storage::ServerStatus::Running,
    };

    state_manager.create_server(server_config.clone()).unwrap();

    // Create new state manager instance (simulates restart)
    let state_manager2 = StateManager::new(dir.path()).unwrap();

    // Verify server persisted
    let retrieved = state_manager2.get_server("test").unwrap();
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.name, "test");
    assert_eq!(retrieved.onion_address, Some("test.onion".to_string()));
}
