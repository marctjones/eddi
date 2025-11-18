# eddi Message Server

A secure, decentralized message passing system built on Tor hidden services using the Introduction/Rendezvous pattern.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Security Model](#security-model)
- [Getting Started](#getting-started)
- [Commands](#commands)
- [Advanced Usage](#advanced-usage)
- [Troubleshooting](#troubleshooting)

## Overview

The eddi message server provides secure, ephemeral message passing between clients using a three-server architecture:

- **Fortress (Server C)**: Long-running message server with persistent `.onion` address
- **Broker (Server A)**: Ephemeral handshake server for client authentication
- **Client (Server B)**: Users connecting via broker to access the fortress

### Key Features

- **Zero Trust Introduction**: Broker validates clients before providing fortress access
- **Ephemeral Brokers**: Handshake servers exist only temporarily (default: 2 minutes)
- **Message Expiration**: Messages automatically expire (default: 5 minutes)
- **Multi-Instance**: Run multiple independent fortresses simultaneously
- **Time-Based Discovery**: Clients find brokers using namespace + code + timestamp
- **Optional Tor Integration**: Local Unix sockets or Tor hidden services

## Architecture

### The Introduction Pattern

```
1. Admin creates Fortress → Gets long .onion address
2. Admin creates Broker → Gets short code (ABC-XYZ)
3. Admin shares code out-of-band (phone, secure channel)
4. Client connects to Broker using code → Handshake
5. Broker sends Fortress address + access token → Shuts down
6. Client connects to Fortress with token → Persistent access
```

### Benefits

1. **Attack Surface Minimization**: Broker is only exposed for 2 minutes
2. **Fortress Stealth**: Main server doesn't handle authentication traffic
3. **Persistence**: Clients can reconnect without new codes
4. **Key Rotation**: Revoke client tokens without restarting server

## Security Model

### Broker Discovery

Brokers are discoverable via:
- **Namespace**: Email or identifier (e.g., `user@example.com`)
- **Short Code**: 6-character code (e.g., `ABC-XYZ`)
- **Timestamp**: Current time rounded to minute

Formula: `SHA256(namespace + timestamp + code)[0:16]`

Clients brute-force timestamps within a ±5 minute window to find the broker.

### Access Control

1. **Broker Authentication**: Client must provide correct code
2. **Fortress Tokens**: Broker issues time-limited access tokens
3. **Token Revocation**: Admin can revoke individual client tokens
4. **Optional Stealth Mode**: Tor Client Authorization (fortress is invisible without key)

### Message Security

- Messages stored in-memory only
- Automatic expiration (configurable TTL)
- Broadcast to all authenticated clients
- No persistence to disk

## Getting Started

### Prerequisites

- Rust 1.70+
- Unix-like operating system (Linux, macOS)
- Tor installed (optional, for onion services)

### Build

```bash
cd eddi
cargo build --release
```

### Quick Start

#### 1. Create a Fortress

```bash
./target/release/eddi msgsrv create-fortress \
    --name my-server \
    --ttl 10
```

Output:
```
Fortress "my-server" created
Socket: /tmp/eddi-msgsrv-my-server.sock
Status: Running
```

#### 2. Create a Broker

```bash
./target/release/eddi msgsrv create-broker \
    --fortress my-server \
    --namespace user@example.com
```

Output:
```
Broker created for fortress "my-server"
Namespace: user@example.com
Short Code: ABC-XYZ
Valid for: 120 seconds

Share this code with your client:
  Code: ABC-XYZ
  Namespace: user@example.com
```

#### 3. Client Connects

On the client machine:

```bash
./target/release/eddi msgsrv connect \
    --code ABC-XYZ \
    --namespace user@example.com
```

Output:
```
Searching for broker...
Found broker at timestamp 1234567890
Performing handshake...
Received fortress address: my-server
Access token: XYZ123...
Connected to fortress!
```

#### 4. Send Messages

```bash
./target/release/eddi msgsrv send "Hello, fortress!"
```

#### 5. Receive Messages

```bash
# One-time receive
./target/release/eddi msgsrv receive --once

# Continuous listening
./target/release/eddi msgsrv listen
```

## Commands

### Fortress Management

#### Create Fortress

```bash
eddi msgsrv create-fortress --name <NAME> [OPTIONS]
```

Options:
- `--name <NAME>`: Fortress name (required)
- `--ttl <MINUTES>`: Message TTL in minutes (default: 5)
- `--onion`: Enable Tor hidden service
- `--stealth`: Enable Tor client authorization

#### Stop Fortress

```bash
eddi msgsrv stop-fortress <NAME>
```

#### List Fortresses

```bash
eddi msgsrv list-fortresses [--verbose]
```

### Broker Management

#### Create Broker

```bash
eddi msgsrv create-broker --fortress <NAME> --namespace <ID> [OPTIONS]
```

Options:
- `--fortress <NAME>`: Fortress to connect to (required)
- `--namespace <ID>`: Discovery namespace (required)
- `--timeout <SECONDS>`: Broker timeout (default: 120)
- `--onion`: Enable Tor hidden service

#### List Brokers

```bash
eddi msgsrv list-brokers
```

### Client Operations

#### Connect to Fortress

```bash
eddi msgsrv connect --code <CODE> --namespace <ID> [OPTIONS]
```

Options:
- `--code <CODE>`: Short code from broker (required)
- `--namespace <ID>`: Namespace (required)
- `--time-window <MINUTES>`: Search window (default: 5)
- `--alias <NAME>`: Alias for this connection

#### Send Message

```bash
eddi msgsrv send <MESSAGE> [--server <NAME>]
```

#### Receive Messages

```bash
eddi msgsrv receive [OPTIONS]
```

Options:
- `--server <NAME>`: Server name or alias
- `--once`: Retrieve once and exit
- `--since <TIMESTAMP>`: Only messages since timestamp

#### Listen for Messages

```bash
eddi msgsrv listen [OPTIONS]
```

Options:
- `--server <NAME>`: Server name or alias
- `--daemon`: Run as system daemon
- `--background`: Run in background (detach from terminal)

### Administration

#### Show Status

```bash
eddi msgsrv status [NAME]
```

#### List Clients

```bash
eddi msgsrv list-clients --fortress <NAME>
```

#### Revoke Client

```bash
eddi msgsrv revoke-client --fortress <NAME> --code <CODE>
```

#### Disconnect

```bash
eddi msgsrv disconnect <NAME>
```

#### Cleanup

```bash
eddi msgsrv cleanup [--force]
```

## Advanced Usage

### Using Helper Scripts

The `scripts/eddi-msgsrv` helper provides convenience functions:

```bash
# Create fortress
./scripts/eddi-msgsrv create-fortress my-server 10

# Create broker
./scripts/eddi-msgsrv create-broker my-server user@example.com

# Send message
./scripts/eddi-msgsrv send-message "Hello!" my-server

# List fortresses
./scripts/eddi-msgsrv list

# Show status
./scripts/eddi-msgsrv status

# Cleanup
./scripts/eddi-msgsrv cleanup
```

### Multiple Fortresses

Run multiple independent fortresses:

```bash
# Create multiple fortresses
eddi msgsrv create-fortress --name personal-chat --ttl 5
eddi msgsrv create-fortress --name work-team --ttl 10
eddi msgsrv create-fortress --name project-collab --ttl 15

# Each has independent:
# - Unix socket
# - Message queue
# - Client list
# - Access tokens
```

### Tor Integration

Enable Tor hidden services for remote access:

```bash
# Create fortress with Tor
eddi msgsrv create-fortress --name secure-server --ttl 5 --onion

# Create broker with Tor
eddi msgsrv create-broker \
    --fortress secure-server \
    --namespace user@example.com \
    --onion
```

### Stealth Mode

Enable Tor Client Authorization for maximum security:

```bash
# Fortress is invisible without client authorization key
eddi msgsrv create-fortress \
    --name stealth-server \
    --ttl 5 \
    --onion \
    --stealth
```

## Troubleshooting

### Broker Not Found

If client can't find broker:

1. Check time synchronization (NTP)
2. Verify namespace matches exactly
3. Increase time window: `--time-window 10`
4. Ensure broker hasn't timed out (default: 2 minutes)

### Connection Issues

If clients can't connect:

1. Check fortress is running: `eddi msgsrv status`
2. Verify socket permissions: `ls -l /tmp/eddi-msgsrv-*.sock`
3. Check logs for errors
4. Ensure no firewall blocking (for Tor)

### Message Delivery

If messages aren't delivered:

1. Check message hasn't expired (check TTL)
2. Verify client is authenticated: `eddi msgsrv list-clients --fortress <NAME>`
3. Ensure fortress is running
4. Check client is listening

### State Issues

If state becomes corrupted:

```bash
# Backup state
cp -r ~/.eddi/msgservers ~/.eddi/msgservers.backup

# Clean up
eddi msgsrv cleanup --force

# Remove state (nuclear option)
rm -rf ~/.eddi/msgservers
```

## Files and Directories

```
~/.eddi/msgservers/
├── state.db                    # SQLite database
└── ...

/tmp/
├── eddi-msgsrv-<name>.sock    # Unix domain sockets
└── ...
```

## Security Considerations

1. **Out-of-Band Communication**: Always share broker codes via secure channels (phone, Signal, etc.)
2. **Short Broker Lifetime**: Brokers timeout quickly (2 minutes) to minimize attack window
3. **Token Expiration**: Set appropriate token TTL based on use case
4. **Message Expiration**: Set TTL based on sensitivity (shorter = more secure)
5. **Stealth Mode**: Use for maximum security, but requires key distribution
6. **Local Sockets**: For local-only use, Unix sockets are more secure than Tor

## License

MIT OR Apache-2.0
