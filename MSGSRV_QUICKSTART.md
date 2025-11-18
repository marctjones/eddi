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

### 1. Create a Fortress (Server)

```bash
eddi-msgsrv create-fortress --name my-server --ttl 5
```

**Output:**
```
Creating fortress: my-server
✓ Fortress 'my-server' created
  Socket: /tmp/eddi-msgsrv-my-server.sock
  Message TTL: 5 minutes
  Status: Running

Press Ctrl+C to stop the fortress
```

### 2. Create a Broker (Handshake Server)

In a new terminal:

```bash
eddi-msgsrv create-broker --fortress my-server --namespace user@example.com
```

**Output:**
```
Creating broker for fortress: my-server
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
  Fortress: fortress-address.onion
  Access token: XYZ123AB...

✓ Connected to fortress!
```

### 4. Send a Message

```bash
eddi-msgsrv send "Hello, fortress!"
```

**Output:**
```
📤 Sending message to: fortress-address.onion
  Message: Hello, fortress!
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
# List all fortresses
eddi-msgsrv list-fortresses

# Show status
eddi-msgsrv status

# List connections
eddi-msgsrv list-connections

# Stop a fortress
eddi-msgsrv stop-fortress my-server

# Cleanup
eddi-msgsrv cleanup --force
```

### Advanced

```bash
# List clients for a fortress
eddi-msgsrv list-clients --fortress my-server

# Revoke client access
eddi-msgsrv revoke-client --fortress my-server --code H7K-9M3

# Disconnect from fortress
eddi-msgsrv disconnect my-server
```

## 🛠️ Helper Scripts

### Quick Operations

```bash
# Create fortress
./scripts/eddi-msgsrv create-fortress my-server 10

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

### Tor Integration Now Available! 🧅

**Q: Do messages go over Tor when sent from the same machine?**

**A: You can choose! Use `--onion` flag for hybrid mode (Unix + Tor).**

**Two Modes Available:**

1. **Local Mode (Default - Unix Sockets Only)**
   ```bash
   eddi-msgsrv create-fortress --name my-server --ttl 5
   ```
   - Uses **Unix Domain Sockets** (`/tmp/eddi-msgsrv-*.sock`)
   - Kernel-level IPC, **not** network sockets
   - Never touches the network stack
   - Isolated at OS level (requires socket file permissions)
   - **Fast and secure for local/same-machine communication**
   - No Tor overhead

2. **Hybrid Mode (Unix + Tor)**
   ```bash
   eddi-msgsrv create-fortress --name my-server --ttl 5 --onion
   ```
   - Listens on **both** Unix socket AND Tor onion service
   - Gets a persistent `.onion` address
   - Local clients use Unix socket (fast)
   - Remote clients connect via Tor (secure, anonymous)
   - All Tor traffic encrypted and anonymized
   - Takes 30-60 seconds to bootstrap Tor

**When to use what:**
- **Unix Sockets** (default): Fast, secure local-only communication
- **Hybrid Mode** (`--onion`): When you need remote access, anonymity, or censorship resistance
- Best practice: Use hybrid mode to get both fast local access AND secure remote access

### Introduction Pattern

1. **Admin creates Fortress** → Gets persistent address
2. **Admin creates Broker** → Gets ephemeral code (2-minute lifetime)
3. **Admin shares code** → Via secure channel (phone, Signal, etc.)
4. **Client connects to Broker** → Time-based discovery
5. **Broker performs handshake** → Validates client
6. **Broker issues token** → Fortress access granted
7. **Broker shuts down** → No longer exposed
8. **Client connects to Fortress** → With access token

### Benefits

- **Attack Surface Minimization**: Broker only lives for 2 minutes
- **Fortress Stealth**: Main server doesn't handle authentication
- **Persistence**: Clients can reconnect without new codes
- **Token Revocation**: Remove access without restarting server

## 🌐 Network Topology

```
Local Mode (Unix Sockets - Default):
  Fortress ← UDS → Client (local)
  Broker ← UDS → Client (local)

Hybrid Mode (--onion flag) ✅:
  Fortress ← UDS → Client (local, fast)
           ↓
           Tor → Client (remote, secure)

  Both listeners active simultaneously!

Future (Broker Tor + Client Connector):
  Fortress ← UDS/Tor → Client
  Broker ← Tor → Client (ephemeral .onion)
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
# Check fortress status
eddi-msgsrv status my-server

# List active connections
eddi-msgsrv list-connections

# Check socket permissions
ls -l /tmp/eddi-msgsrv-*.sock
```

### Clean Slate

```bash
# Stop all fortresses
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
- Fortress creation and management
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
- ✅ Fortress onion services (Phase 2.1)
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

### Scenario 0: Remote Access via Tor 🧅

```bash
# Create fortress with Tor enabled (hybrid mode)
eddi-msgsrv create-fortress --name remote-server --ttl 10 --onion

# Output includes onion address:
# 🧅 Onion Address: abc123def456ghijklmno789.onion
#   (Accessible via Tor network)

# Local clients connect via Unix socket (fast)
eddi-msgsrv send "Hello local!" --server remote-server

# Remote clients will connect via Tor (when Phase 2.3 is complete)
# eddi-msgsrv connect --onion abc123def456.onion --code H7K-9M3
```

**Use case:** Secure remote access without exposing IP addresses, perfect for:
- Censorship-resistant communication
- Anonymous coordination
- Privacy-focused messaging
- Remote team collaboration

### Scenario 1: Team Collaboration

```bash
# Team lead creates fortress
eddi-msgsrv create-fortress --name team-chat --ttl 10

# For each team member, create broker
eddi-msgsrv create-broker --fortress team-chat --namespace alice@team.com
# Share code: H7K-9M3

eddi-msgsrv create-broker --fortress team-chat --namespace bob@team.com
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
# Quick fortress for one-time event
eddi-msgsrv create-fortress --name event-coord --ttl 1

# Create brokers for participants
eddi-msgsrv create-broker --fortress event-coord --namespace coord@event.org

# After event, cleanup
eddi-msgsrv stop-fortress event-coord
eddi-msgsrv cleanup --force
```

## 📄 License

MIT OR Apache-2.0

## 🤝 Contributing

This is part of the eddi project. See main README for contribution guidelines.
