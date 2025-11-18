// CLI commands for message server

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Message server CLI commands
#[derive(Debug, Parser)]
#[command(name = "msgsrv")]
#[command(about = "Secure message passing system with Tor hidden services")]
pub struct MsgSrvCli {
    #[command(subcommand)]
    pub command: MsgSrvCommand,
}

/// Message server subcommands
#[derive(Debug, Subcommand)]
pub enum MsgSrvCommand {
    /// Create a new eddi messaging server (persistent emsgsrv)
    CreateServer {
        /// Server name
        #[arg(short, long)]
        name: String,

        /// Message TTL in minutes (default: 5)
        #[arg(short, long, default_value = "5")]
        ttl: u64,

        /// Disable Tor and use local Unix sockets only (advanced option)
        #[arg(long)]
        local_only: bool,

        /// Enable stealth mode (Tor client authorization)
        #[arg(long)]
        stealth: bool,
    },

    /// Create a broker (ephemeral handshake server)
    CreateBroker {
        /// Eddi messaging server to connect to
        #[arg(short, long)]
        server: String,

        /// Namespace for broker discovery (e.g., email@example.com)
        #[arg(short, long)]
        namespace: String,

        /// Broker timeout in seconds (default: 120)
        #[arg(short, long, default_value = "120")]
        timeout: u64,

        /// Disable Tor and use local Unix sockets only (advanced option)
        #[arg(long)]
        local_only: bool,
    },

    /// Connect to an eddi messaging server via broker
    Connect {
        /// Short code for broker discovery (e.g., ABC-XYZ)
        #[arg(short, long)]
        code: String,

        /// Namespace (should match broker's namespace)
        #[arg(short, long)]
        namespace: String,

        /// Time window for broker search in minutes (default: 5)
        #[arg(short = 'w', long, default_value = "5")]
        time_window: i64,

        /// Alias for this connection (optional)
        #[arg(short, long)]
        alias: Option<String>,
    },

    /// Send a message
    Send {
        /// Message content
        message: String,

        /// Server name or alias (default: last connected)
        #[arg(short, long)]
        server: Option<String>,
    },

    /// Receive messages
    Receive {
        /// Server name or alias (default: last connected)
        #[arg(short, long)]
        server: Option<String>,

        /// Only retrieve once and exit
        #[arg(long)]
        once: bool,

        /// Show only messages since timestamp
        #[arg(long)]
        since: Option<u64>,
    },

    /// Listen for messages (continuous mode)
    Listen {
        /// Server name or alias (default: last connected)
        #[arg(short, long)]
        server: Option<String>,

        /// Run as system daemon
        #[arg(long)]
        daemon: bool,

        /// Run in background (detach from terminal)
        #[arg(long)]
        background: bool,
    },

    /// List all eddi messaging servers
    ListServers {
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },

    /// List active brokers
    ListBrokers,

    /// List clients for an eddi messaging server
    ListClients {
        /// Server name
        #[arg(short, long)]
        server: String,
    },

    /// List connections
    ListConnections {
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },

    /// Show status of servers and connections
    Status {
        /// Server name (optional, shows all if not specified)
        name: Option<String>,
    },

    /// Stop an eddi messaging server
    StopServer {
        /// Server name
        name: String,
    },

    /// Stop a broker
    StopBroker {
        /// Broker ID or server name
        id: String,
    },

    /// Disconnect from a server
    Disconnect {
        /// Connection name or alias
        name: String,
    },

    /// Revoke client access
    RevokeClient {
        /// Server name
        #[arg(short, long)]
        server: String,

        /// Client code or token
        #[arg(short, long)]
        code: String,
    },

    /// Clean up stopped servers and stale connections
    Cleanup {
        /// Actually delete (default is dry-run)
        #[arg(long)]
        force: bool,
    },
}

impl MsgSrvCli {
    /// Parse from command-line arguments
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Get the state directory
    pub fn state_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".eddi").join("msgservers")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        // Test create-server command (default Tor mode)
        let args = vec![
            "msgsrv",
            "create-server",
            "--name",
            "my-server",
            "--ttl",
            "10",
        ];

        let cli = MsgSrvCli::try_parse_from(args);
        assert!(cli.is_ok());

        // Test create-server with local-only flag
        let args_local = vec![
            "msgsrv",
            "create-server",
            "--name",
            "my-server",
            "--ttl",
            "10",
            "--local-only",
        ];

        let cli_local = MsgSrvCli::try_parse_from(args_local);
        assert!(cli_local.is_ok());
    }

    #[test]
    fn test_state_dir() {
        let dir = MsgSrvCli::state_dir();
        assert!(dir.ends_with(".eddi/msgservers"));
    }
}
