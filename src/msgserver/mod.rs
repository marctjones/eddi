// Message server module for CLI message passing system
//
// This module provides a lightweight message broker that enables
// command-line message passing between multiple clients with support
// for message expiration, authentication, and Tor hidden services.

pub mod message;
pub mod storage;
pub mod client;
pub mod broker;
pub mod server;
pub mod handshake;
pub mod cli;
pub mod commands;

pub use message::{Message, MessageQueue};
pub use storage::{StateManager, ServerConfig, ClientConfig};
pub use client::{ClientConnection, ClientManager};
pub use broker::{MessageBroker, BrokerHandle};
pub use server::{ServerInstance, ServerManager};
pub use handshake::{BrokerHandshake, ClientHandshake, IntroductionData};
pub use cli::{MsgSrvCli, MsgSrvCommand};
pub use commands::execute_command;
