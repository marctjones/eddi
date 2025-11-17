# eddi

A secure, self-contained application launcher that exposes web applications only as Tor hidden services.

## Overview

`eddi` is a Rust-based command-line tool that bridges web applications (Python, PHP, .NET, etc.) to the Tor network without exposing any TCP ports. It uses the Arti Tor library and Unix Domain Sockets to create a secure, isolated environment.

## Features

- ✓ **Pure Tor connectivity** - Uses Arti (Rust Tor implementation), no proxy servers
- ✓ **No IP exposure** - Never uses IP-based protocols
- ✓ **Unix Domain Sockets** - Secure inter-process communication
- ✓ **Process isolation** - Managed child processes
- ✓ **Zero TCP ports** - No network ports exposed on your system

## Quick Start

### 1. Build Everything

```bash
./build.sh
```

This will:
- Build all Rust binaries (eddi, tor-check, tor-http-client, etc.)
- Set up Python virtual environment
- Install Flask demo app dependencies

### 2. Start the Server

```bash
./start-server.sh
```

This will:
- Bootstrap to Tor network via Arti
- Launch a Tor v3 onion service
- Start the Flask web application on Unix Domain Socket
- Display your .onion address

The server runs the Flask app and proxies connections from Tor. Wait 30-60 seconds for the service to become fully reachable.

### 3. Connect via Tor (Pure Arti Client)

In another terminal:

```bash
./tor-connect.sh
```

This client:
- Uses **only** Arti (no proxy servers)
- Connects directly via Tor network
- Never uses IP-based protocols
- Makes pure onion-to-onion connections

You can also specify an onion address directly:

```bash
./tor-connect.sh http://your-address.onion:80
./tor-connect.sh your-address.onion:80/status
```

## Testing

Run all tests:

```bash
./scripts/run-tests.sh
```

This runs:
- Unit tests
- Integration tests
- Network isolation tests
- Process management tests

Or run tests manually:

```bash
# Unit tests only
cargo test

# All tests including network tests
cargo test -- --ignored

# Specific test suite
cargo test process_tests
```

## Project Structure

```
eddi/
├── build.sh              # Build all components
├── start-server.sh       # Start EDDI server
├── tor-connect.sh        # Connect via pure Tor (Arti)
├── src/
│   ├── main.rs          # Main EDDI application
│   ├── process.rs       # Process management
│   └── bin/             # Additional binaries
│       ├── tor-check.rs        # Tor diagnostics
│       ├── tor-http-client.rs  # Pure Arti HTTP client
│       ├── tor-msg-server.rs   # Message relay demo
│       ├── tor-msg-client.rs   # Message client demo
│       └── task3.rs            # UDS demo
├── scripts/             # Utility scripts
│   ├── run-tests.sh            # Test runner
│   ├── run-tor-check.sh        # Tor connectivity check
│   ├── launch-server.sh        # Message server launcher
│   └── connect-client.sh       # Message client launcher
├── docs/                # Documentation
│   ├── GEMINI.md               # Project plan
│   ├── TASK2.md                # Arti POC docs
│   ├── TASK3.md                # UDS docs
│   ├── TASK4.md                # Complete implementation
│   ├── TESTING.md              # Testing guide
│   ├── DEPLOYMENT.md           # Deployment guide
│   ├── SECURITY.md             # Security documentation
│   └── TOR-MESSAGING.md        # Tor messaging docs
├── test-apps/
│   └── flask-demo/      # Demo Flask application
└── tests/               # Test suites
```

## Documentation

- **Project Plan**: [docs/GEMINI.md](docs/GEMINI.md)
- **Testing Guide**: [docs/TESTING.md](docs/TESTING.md)
- **Deployment**: [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md)
- **Security**: [docs/SECURITY.md](docs/SECURITY.md)
- **Task Documentation**: [docs/TASK2.md](docs/TASK2.md), [docs/TASK3.md](docs/TASK3.md), [docs/TASK4.md](docs/TASK4.md)

## Tor Connectivity Check

Before running eddi, verify your system can connect to Tor:

```bash
./scripts/run-tor-check.sh
```

Or run directly:

```bash
cargo run --bin tor-check
```

This diagnostic tool will:
- Test DNS resolution
- Bootstrap to Tor network
- Test remote website access via Tor
- Test onion service connections
- Provide troubleshooting guidance

## Advanced Usage

### Running Individual Components

```bash
# Task 3 demo (UDS and process management)
cargo run --bin task3

# Tor message relay system
./scripts/launch-server.sh  # Terminal 1
./scripts/connect-client.sh # Terminal 2

# Manual HTTP client
cargo run --release --bin tor-http-client http://example.onion:80
```

### Environment Variables

```bash
# Enable debug logging
RUST_LOG=debug ./start-server.sh

# Enable trace logging
RUST_LOG=trace cargo run --bin tor-check
```

## License

MIT OR Apache-2.0