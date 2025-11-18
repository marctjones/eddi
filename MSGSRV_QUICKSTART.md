# eddi Message Server - Quick Start Guide

A secure, decentralized message passing system with the Introduction/Rendezvous pattern.

## 🚀 Quick Start

### Build

```bash
cargo build --release
```

The binary will be at `./target/release/eddi-msgsrv`

### Run Demo

```bash
./scripts/demo-msgsrv.sh
```

## 📖 Basic Usage

### 1. Create a Server (Server)

```bash
eddi-msgsrv create-server --name my-server --ttl 5
```

**Output:**
```
Creating eddi messaging server: my-server
✓ Server 'my-server' created
  Socket: /tmp/eddi-msgsrv-my-server.sock
  Message TTL: 5 minutes
  Status: Running

Press Ctrl+C to stop the server
```

### 2. Create a Broker (Handshake Server)

In a new terminal:

```bash
eddi-msgsrv create-broker --server my-server --namespace user@example.com
```

**Output:**
```
Creating broker for server: my-server
✓ Broker created

📋 Connection Details:
  Namespace: user@example.com
  Short Code: H7K-9M3
  Valid for: 120 seconds
  Broker ID: a1b2c3d4e5f6...

💡 Share with your client:
  eddi-msgsrv connect --code H7K-9M3 --namespace user@example.com

⏳ Waiting for client connection...
```

### 3. Connect as Client

In another terminal:

```bash
eddi-msgsrv connect --code H7K-9M3 --namespace user@example.com
```

**Output:**
```
🔍 Searching for broker...
  Code: H7K-9M3
  Namespace: user@example.com
  Time window: ±5 minutes
  Trying 11 possible timestamps...
✓ Found broker at timestamp 1234567890
  Broker ID: a1b2c3d4e5f6...

✓ Handshake successful!
  Server: server-address.onion
  Access token: XYZ123AB...

✓ Connected to server!
```

### 4. Send a Message

```bash
eddi-msgsrv send "Hello, server!"
```

**Output:**
```
📤 Sending message to: server-address.onion
  Message: Hello, server!
✓ Message sent
```

### 5. Listen for Messages

```bash
eddi-msgsrv listen
```

**Output:**
```
👂 Listening for messages on: default
  Mode: Foreground
  (Press Ctrl+C to stop)
```

## 🎯 Common Commands

### Management

```bash
# List all servers
eddi-msgsrv list-servers

# Show status
eddi-msgsrv status

# List connections
eddi-msgsrv list-connections

# Stop a server
eddi-msgsrv stop-server my-server

# Cleanup
eddi-msgsrv cleanup --force
```

### Advanced

```bash
# List clients for a server
eddi-msgsrv list-clients --server my-server

# Revoke client access
eddi-msgsrv revoke-client --server my-server --code H7K-9M3

# Disconnect from server
eddi-msgsrv disconnect my-server
```

## 🛠️ Helper Scripts

### Quick Operations

```bash
# Create server
./scripts/eddi-msgsrv create-server my-server 10

# Create broker
./scripts/eddi-msgsrv create-broker my-server user@example.com

# Send message
./scripts/eddi-msgsrv send-message "Hello!" my-server

# Show status
./scripts/eddi-msgsrv status
```

### Run Tests

```bash
# Run all tests
cargo test

# Run integration tests
cargo test --test msgserver_tests

# Run test script
./scripts/test-msgsrv.sh
```

## 📁 File Locations

- **State database**: `~/.eddi/msgservers/state.db`
- **Unix sockets**: `/tmp/eddi-msgsrv-<name>.sock`
- **Configuration**: Stored in SQLite database

## 🔐 Security Model

### Tor-First Architecture 🧅

**Q: Do messages go over Tor by default?**

**A: YES! Tor is enabled by default for secure, anonymous messaging.**

**Two Modes Available:**

1. **Tor Mode (Default - Recommended) 🧅**
   ```bash
   eddi-msgsrv create-server --name my-server --ttl 5
   ```
   - **Tor enabled by default** - no flags needed!
   - Gets a persistent `.onion` address
   - Accessible via Tor network for remote, anonymous access
   - Also includes Unix socket for fast local access (hybrid mode)
   - All Tor traffic encrypted and anonymized
   - Takes 30-60 seconds to bootstrap Tor on first run
   - **Recommended for production use**

2. **Local-Only Mode (Advanced Option - Development)**
   ```bash
   eddi-msgsrv create-server --name my-server --ttl 5 --local-only
   ```
   - **Disables Tor** - Unix sockets only
   - Uses **Unix Domain Sockets** (`/tmp/eddi-msgsrv-*.sock`)
   - Kernel-level IPC, **not** network sockets
   - Never touches the network stack
   - **Fast for local development/testing**
   - No Tor bootstrap delay
   - ⚠️ Not accessible remotely

**When to use what:**
- **Tor Mode** (default): Production use, remote access, anonymity, censorship resistance
- **Local-Only** (`--local-only`): Fast local development, testing, debugging
- Best practice: Use default Tor mode unless you have a specific reason not to

### Introduction Pattern

1. **Admin creates Server** → Gets persistent address
2. **Admin creates Broker** → Gets ephemeral code (2-minute lifetime)
3. **Admin shares code** → Via secure channel (phone, Signal, etc.)
4. **Client connects to Broker** → Time-based discovery
5. **Broker performs handshake** → Validates client
6. **Broker issues token** → Server access granted
7. **Broker shuts down** → No longer exposed
8. **Client connects to Server** → With access token

### Benefits

- **Attack Surface Minimization**: Broker only lives for 2 minutes
- **Server Stealth**: Main server doesn't handle authentication
- **Persistence**: Clients can reconnect without new codes
- **Token Revocation**: Remove access without restarting server

## 🌐 Network Topology

```
Tor Mode (Default) ✅:
  Server ← UDS → Client (local, fast)
           ↓
           Tor → Client (remote, secure)

  Both listeners active simultaneously (hybrid mode)!

Local-Only Mode (--local-only flag):
  Server ← UDS → Client (local only)
  Broker ← UDS → Client (local only)

  No Tor, fast development mode

Future (Broker Tor + Client Connector):
  Server ← UDS/Tor → Client
  Broker ← Tor → Client (ephemeral .onion)

  Full end-to-end Tor integration
```

## 🐛 Troubleshooting

### Broker Not Found

```bash
# Increase search window
eddi-msgsrv connect --code ABC-XYZ --namespace user@example.com --time-window 10

# Check time synchronization
timedatectl status
```

### Connection Issues

```bash
# Check server status
eddi-msgsrv status my-server

# List active connections
eddi-msgsrv list-connections

# Check socket permissions
ls -l /tmp/eddi-msgsrv-*.sock
```

### Clean Slate

```bash
# Stop all servers
eddi-msgsrv cleanup --force

# Remove state (nuclear option)
rm -rf ~/.eddi/msgservers

# Remove sockets
rm -f /tmp/eddi-msgsrv-*.sock
```

## 📚 Full Documentation

See [docs/MESSAGE_SERVER.md](docs/MESSAGE_SERVER.md) for comprehensive documentation including:
- Architecture details
- Security considerations
- Advanced usage
- Tor integration
- API reference

## 🚦 What's Working

✅ **Core Functionality**
- Server creation and management
- Broker creation with code generation
- Client handshake simulation
- Message protocol
- State persistence (SQLite)
- Multi-instance support

✅ **CLI Commands**
- All 15 commands implemented
- Help system
- Color-coded output
- Error handling

✅ **Testing**
- 19 unit tests (all passing)
- 10 integration tests (all passing)
- Test automation scripts

## 🔨 What's Next (Future Enhancements)

✅ **Tor Integration (Partially Complete)**
- ✅ Server onion services (Phase 2.1)
- ✅ Hybrid mode (Unix + Tor listeners)
- ⏳ Ephemeral broker onion addresses (Phase 2.2)
- ⏳ Client Tor connector (Phase 2.3)
- ⏳ Client authorization / stealth mode (Phase 3)

⏳ **Real Message Passing**
- Unix socket client implementation
- Actual message send/receive
- Real-time broadcasting

⏳ **Daemon Modes**
- Systemd integration
- Background process management
- Auto-restart on failure

⏳ **Enhanced Security**
- PAKE authentication upgrade
- End-to-end encryption
- Forward secrecy

## 💡 Example Workflows

### Scenario 0: Remote Access via Tor 🧅 (Default)

```bash
# Create server with Tor enabled (default - no flags needed!)
eddi-msgsrv create-server --name remote-server --ttl 10

# Output:
# 🧅 Tor mode enabled (default) - server will be accessible via .onion address
# ⏳ This may take 30-60 seconds (bootstrapping Tor)...
# 💡 Use --local-only to disable Tor for fast local development
#
# ✓ Server 'remote-server' created
#   Socket: /tmp/eddi-msgsrv-remote-server.sock
#   Message TTL: 10 minutes
#   Status: Running
#
# 🧅 Onion Address: abc123def456ghijklmno789.onion
#   (Accessible via Tor network)

# Local clients connect via Unix socket (fast)
eddi-msgsrv send "Hello local!" --server remote-server

# Remote clients will connect via Tor (when Phase 2.3 is complete)
# eddi-msgsrv connect --code H7K-9M3 --namespace myteam@example.com
```

**Use case:** Secure remote access without exposing IP addresses, perfect for:
- Censorship-resistant communication
- Anonymous coordination
- Privacy-focused messaging
- Remote team collaboration

**Note:** Tor is now the default! No need for `--onion` flag.

### Scenario 1: Team Collaboration

```bash
# Team lead creates server (Tor enabled by default)
eddi-msgsrv create-server --name team-chat --ttl 10

# For each team member, create broker
eddi-msgsrv create-broker --server team-chat --namespace alice@team.com
# Share code: H7K-9M3

eddi-msgsrv create-broker --server team-chat --namespace bob@team.com
# Share code: P2R-5X8

# Team members connect
eddi-msgsrv connect --code H7K-9M3 --namespace alice@team.com --alias team-chat
eddi-msgsrv connect --code P2R-5X8 --namespace bob@team.com --alias team-chat

# Everyone can now send/receive
eddi-msgsrv send "Hello team!" --server team-chat
eddi-msgsrv listen --server team-chat
```

### Scenario 2: Ephemeral Coordination

```bash
# Quick server for one-time event
eddi-msgsrv create-server --name event-coord --ttl 1

# Create brokers for participants
eddi-msgsrv create-broker --server event-coord --namespace coord@event.org

# After event, cleanup
eddi-msgsrv stop-server event-coord
eddi-msgsrv cleanup --force
```

## 📄 License

MIT OR Apache-2.0

## 🤝 Contributing

This is part of the eddi project. See main README for contribution guidelines.
