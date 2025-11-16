//! eddi - Secure, self-contained application launcher for Tor hidden services
//!
//! This library provides utilities for managing web application processes
//! bound to Unix Domain Sockets and exposing them via Arti onion services.

pub mod process;

pub use process::{ChildProcessManager, ProcessConfig};
