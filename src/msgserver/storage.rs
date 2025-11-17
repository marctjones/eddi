// Persistent state management using SQLite

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use uuid::Uuid;

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub id: String,
    pub name: String,
    pub socket_path: PathBuf,
    pub created_at: SystemTime,
    pub ttl_minutes: u64,
    pub onion_address: Option<String>,
    pub status: ServerStatus,
}

/// Server status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerStatus {
    Running,
    Stopped,
    Error,
}

impl ServerStatus {
    pub fn to_string(&self) -> String {
        match self {
            ServerStatus::Running => "running".to_string(),
            ServerStatus::Stopped => "stopped".to_string(),
            ServerStatus::Error => "error".to_string(),
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "running" => ServerStatus::Running,
            "stopped" => ServerStatus::Stopped,
            "error" => ServerStatus::Error,
            _ => ServerStatus::Error,
        }
    }
}

/// Client authentication code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub id: String,
    pub server_id: String,
    pub code: String,
    pub created_at: SystemTime,
    pub connected_at: Option<SystemTime>,
    pub status: ClientStatus,
}

/// Client status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClientStatus {
    Pending,
    Connected,
    Disconnected,
}

impl ClientStatus {
    pub fn to_string(&self) -> String {
        match self {
            ClientStatus::Pending => "pending".to_string(),
            ClientStatus::Connected => "connected".to_string(),
            ClientStatus::Disconnected => "disconnected".to_string(),
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "pending" => ClientStatus::Pending,
            "connected" => ClientStatus::Connected,
            "disconnected" => ClientStatus::Disconnected,
            _ => ClientStatus::Disconnected,
        }
    }
}

/// Connection configuration (for clients connecting to remote servers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub id: String,
    pub server_name: String,
    pub alias: Option<String>,
    pub code: String,
    pub socket_path: Option<PathBuf>,
    pub onion_address: Option<String>,
    pub connected_at: SystemTime,
    pub status: ClientStatus,
}

/// State manager for persistent storage
pub struct StateManager {
    db_path: PathBuf,
}

impl StateManager {
    /// Create a new state manager
    pub fn new(base_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(base_dir)
            .context("Failed to create msgservers directory")?;

        let db_path = base_dir.join("state.db");

        let manager = Self { db_path };

        manager.initialize_db()?;

        Ok(manager)
    }

    /// Get database connection
    fn get_connection(&self) -> Result<Connection> {
        Connection::open(&self.db_path)
            .context("Failed to open database connection")
    }

    /// Initialize database schema
    fn initialize_db(&self) -> Result<()> {
        let conn = self.get_connection()?;

        // Servers table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS servers (
                id TEXT PRIMARY KEY,
                name TEXT UNIQUE NOT NULL,
                socket_path TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                ttl_minutes INTEGER NOT NULL,
                onion_address TEXT,
                status TEXT NOT NULL
            )",
            [],
        )?;

        // Clients table (authentication codes for server)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS clients (
                id TEXT PRIMARY KEY,
                server_id TEXT NOT NULL,
                code TEXT UNIQUE NOT NULL,
                created_at INTEGER NOT NULL,
                connected_at INTEGER,
                status TEXT NOT NULL,
                FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Connections table (client connections to remote servers)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS connections (
                id TEXT PRIMARY KEY,
                server_name TEXT NOT NULL,
                alias TEXT,
                code TEXT NOT NULL,
                socket_path TEXT,
                onion_address TEXT,
                connected_at INTEGER NOT NULL,
                status TEXT NOT NULL
            )",
            [],
        )?;

        // Create indices
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_servers_name ON servers(name)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clients_server_id ON clients(server_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clients_code ON clients(code)",
            [],
        )?;

        Ok(())
    }

    // ========== Server Management ==========

    /// Create a new server
    pub fn create_server(&self, config: ServerConfig) -> Result<()> {
        let conn = self.get_connection()?;

        let created_at = config.created_at
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO servers (id, name, socket_path, created_at, ttl_minutes, onion_address, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                config.id,
                config.name,
                config.socket_path.to_string_lossy().to_string(),
                created_at,
                config.ttl_minutes as i64,
                config.onion_address,
                config.status.to_string(),
            ],
        )?;

        Ok(())
    }

    /// Get server by name
    pub fn get_server(&self, name: &str) -> Result<Option<ServerConfig>> {
        let conn = self.get_connection()?;

        let result: Option<ServerConfig> = conn
            .query_row(
                "SELECT id, name, socket_path, created_at, ttl_minutes, onion_address, status
                 FROM servers WHERE name = ?1",
                params![name],
                |row| {
                    let created_at = SystemTime::UNIX_EPOCH
                        + std::time::Duration::from_secs(row.get::<_, i64>(3)? as u64);

                    Ok(ServerConfig {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        socket_path: PathBuf::from(row.get::<_, String>(2)?),
                        created_at,
                        ttl_minutes: row.get::<_, i64>(4)? as u64,
                        onion_address: row.get(5)?,
                        status: ServerStatus::from_string(&row.get::<_, String>(6)?),
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    /// Get server by ID
    pub fn get_server_by_id(&self, id: &str) -> Result<Option<ServerConfig>> {
        let conn = self.get_connection()?;

        let result: Option<ServerConfig> = conn
            .query_row(
                "SELECT id, name, socket_path, created_at, ttl_minutes, onion_address, status
                 FROM servers WHERE id = ?1",
                params![id],
                |row| {
                    let created_at = SystemTime::UNIX_EPOCH
                        + std::time::Duration::from_secs(row.get::<_, i64>(3)? as u64);

                    Ok(ServerConfig {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        socket_path: PathBuf::from(row.get::<_, String>(2)?),
                        created_at,
                        ttl_minutes: row.get::<_, i64>(4)? as u64,
                        onion_address: row.get(5)?,
                        status: ServerStatus::from_string(&row.get::<_, String>(6)?),
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    /// List all servers
    pub fn list_servers(&self) -> Result<Vec<ServerConfig>> {
        let conn = self.get_connection()?;

        let mut stmt = conn.prepare(
            "SELECT id, name, socket_path, created_at, ttl_minutes, onion_address, status
             FROM servers ORDER BY created_at DESC",
        )?;

        let servers = stmt
            .query_map([], |row| {
                let created_at = SystemTime::UNIX_EPOCH
                    + std::time::Duration::from_secs(row.get::<_, i64>(3)? as u64);

                Ok(ServerConfig {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    socket_path: PathBuf::from(row.get::<_, String>(2)?),
                    created_at,
                    ttl_minutes: row.get::<_, i64>(4)? as u64,
                    onion_address: row.get(5)?,
                    status: ServerStatus::from_string(&row.get::<_, String>(6)?),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(servers)
    }

    /// Update server status
    pub fn update_server_status(&self, id: &str, status: ServerStatus) -> Result<()> {
        let conn = self.get_connection()?;

        conn.execute(
            "UPDATE servers SET status = ?1 WHERE id = ?2",
            params![status.to_string(), id],
        )?;

        Ok(())
    }

    /// Update server onion address
    pub fn update_server_onion(&self, id: &str, onion_address: &str) -> Result<()> {
        let conn = self.get_connection()?;

        conn.execute(
            "UPDATE servers SET onion_address = ?1 WHERE id = ?2",
            params![onion_address, id],
        )?;

        Ok(())
    }

    /// Delete server
    pub fn delete_server(&self, name: &str) -> Result<()> {
        let conn = self.get_connection()?;

        conn.execute("DELETE FROM servers WHERE name = ?1", params![name])?;

        Ok(())
    }

    // ========== Client Management ==========

    /// Create a new client authentication code
    pub fn create_client(&self, server_id: &str) -> Result<ClientConfig> {
        let conn = self.get_connection()?;

        let client = ClientConfig {
            id: Uuid::new_v4().to_string(),
            server_id: server_id.to_string(),
            code: generate_client_code(),
            created_at: SystemTime::now(),
            connected_at: None,
            status: ClientStatus::Pending,
        };

        let created_at = client.created_at
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO clients (id, server_id, code, created_at, connected_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                client.id,
                client.server_id,
                client.code,
                created_at,
                None::<i64>,
                client.status.to_string(),
            ],
        )?;

        Ok(client)
    }

    /// Get client by code
    pub fn get_client_by_code(&self, code: &str) -> Result<Option<ClientConfig>> {
        let conn = self.get_connection()?;

        let result: Option<ClientConfig> = conn
            .query_row(
                "SELECT id, server_id, code, created_at, connected_at, status
                 FROM clients WHERE code = ?1",
                params![code],
                |row| {
                    let created_at = SystemTime::UNIX_EPOCH
                        + std::time::Duration::from_secs(row.get::<_, i64>(3)? as u64);

                    let connected_at: Option<i64> = row.get(4)?;
                    let connected_at = connected_at.map(|t| {
                        SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(t as u64)
                    });

                    Ok(ClientConfig {
                        id: row.get(0)?,
                        server_id: row.get(1)?,
                        code: row.get(2)?,
                        created_at,
                        connected_at,
                        status: ClientStatus::from_string(&row.get::<_, String>(5)?),
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    /// List clients for a server
    pub fn list_clients(&self, server_id: &str) -> Result<Vec<ClientConfig>> {
        let conn = self.get_connection()?;

        let mut stmt = conn.prepare(
            "SELECT id, server_id, code, created_at, connected_at, status
             FROM clients WHERE server_id = ?1 ORDER BY created_at DESC",
        )?;

        let clients = stmt
            .query_map(params![server_id], |row| {
                let created_at = SystemTime::UNIX_EPOCH
                    + std::time::Duration::from_secs(row.get::<_, i64>(3)? as u64);

                let connected_at: Option<i64> = row.get(4)?;
                let connected_at = connected_at.map(|t| {
                    SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(t as u64)
                });

                Ok(ClientConfig {
                    id: row.get(0)?,
                    server_id: row.get(1)?,
                    code: row.get(2)?,
                    created_at,
                    connected_at,
                    status: ClientStatus::from_string(&row.get::<_, String>(5)?),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(clients)
    }

    /// Update client status
    pub fn update_client_status(&self, id: &str, status: ClientStatus) -> Result<()> {
        let conn = self.get_connection()?;

        let connected_at = if status == ClientStatus::Connected {
            Some(
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
            )
        } else {
            None
        };

        conn.execute(
            "UPDATE clients SET status = ?1, connected_at = ?2 WHERE id = ?3",
            params![status.to_string(), connected_at, id],
        )?;

        Ok(())
    }

    // ========== Connection Management ==========

    /// Create a new connection
    pub fn create_connection(&self, config: ConnectionConfig) -> Result<()> {
        let conn = self.get_connection()?;

        let connected_at = config.connected_at
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO connections (id, server_name, alias, code, socket_path, onion_address, connected_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                config.id,
                config.server_name,
                config.alias,
                config.code,
                config.socket_path.as_ref().map(|p| p.to_string_lossy().to_string()),
                config.onion_address,
                connected_at,
                config.status.to_string(),
            ],
        )?;

        Ok(())
    }

    /// Get connection by alias or server name
    pub fn get_connection(&self, name: &str) -> Result<Option<ConnectionConfig>> {
        let conn = self.get_connection()?;

        let result: Option<ConnectionConfig> = conn
            .query_row(
                "SELECT id, server_name, alias, code, socket_path, onion_address, connected_at, status
                 FROM connections WHERE server_name = ?1 OR alias = ?1",
                params![name],
                |row| {
                    let connected_at = SystemTime::UNIX_EPOCH
                        + std::time::Duration::from_secs(row.get::<_, i64>(6)? as u64);

                    Ok(ConnectionConfig {
                        id: row.get(0)?,
                        server_name: row.get(1)?,
                        alias: row.get(2)?,
                        code: row.get(3)?,
                        socket_path: row.get::<_, Option<String>>(4)?.map(PathBuf::from),
                        onion_address: row.get(5)?,
                        connected_at,
                        status: ClientStatus::from_string(&row.get::<_, String>(7)?),
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    /// List all connections
    pub fn list_connections(&self) -> Result<Vec<ConnectionConfig>> {
        let conn = self.get_connection()?;

        let mut stmt = conn.prepare(
            "SELECT id, server_name, alias, code, socket_path, onion_address, connected_at, status
             FROM connections ORDER BY connected_at DESC",
        )?;

        let connections = stmt
            .query_map([], |row| {
                let connected_at = SystemTime::UNIX_EPOCH
                    + std::time::Duration::from_secs(row.get::<_, i64>(6)? as u64);

                Ok(ConnectionConfig {
                    id: row.get(0)?,
                    server_name: row.get(1)?,
                    alias: row.get(2)?,
                    code: row.get(3)?,
                    socket_path: row.get::<_, Option<String>>(4)?.map(PathBuf::from),
                    onion_address: row.get(5)?,
                    connected_at,
                    status: ClientStatus::from_string(&row.get::<_, String>(7)?),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(connections)
    }

    /// Delete connection
    pub fn delete_connection(&self, name: &str) -> Result<()> {
        let conn = self.get_connection()?;

        conn.execute(
            "DELETE FROM connections WHERE server_name = ?1 OR alias = ?1",
            params![name],
        )?;

        Ok(())
    }
}

/// Generate a random client code
fn generate_client_code() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // No confusing chars
    const CODE_LEN: usize = 12;

    let mut rng = rand::thread_rng();

    (0..CODE_LEN)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_state_manager() {
        let dir = tempdir().unwrap();
        let manager = StateManager::new(dir.path()).unwrap();

        // Create server
        let server = ServerConfig {
            id: Uuid::new_v4().to_string(),
            name: "test-server".to_string(),
            socket_path: PathBuf::from("/tmp/test.sock"),
            created_at: SystemTime::now(),
            ttl_minutes: 5,
            onion_address: None,
            status: ServerStatus::Running,
        };

        manager.create_server(server.clone()).unwrap();

        // Get server
        let retrieved = manager.get_server("test-server").unwrap().unwrap();
        assert_eq!(retrieved.name, server.name);

        // List servers
        let servers = manager.list_servers().unwrap();
        assert_eq!(servers.len(), 1);

        // Create client
        let client = manager.create_client(&server.id).unwrap();
        assert!(!client.code.is_empty());

        // Get client by code
        let retrieved_client = manager.get_client_by_code(&client.code).unwrap().unwrap();
        assert_eq!(retrieved_client.code, client.code);
    }
}
