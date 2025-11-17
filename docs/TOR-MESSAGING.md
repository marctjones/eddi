# Tor Messaging System

A simple peer-to-peer messaging system that works over Tor hidden services.

## Quick Start

### Terminal 1: Launch the Server

```bash
./launch-server.sh
```

This will:
1. Build the server binary (if needed)
2. Start a Tor hidden service
3. Display the `.onion` address
4. Save the address to `.onion_address` file
5. Wait for connections

The output will show debugging information as connections are made and messages are sent.

### Terminal 2 (or different computer): Connect as a Client

```bash
./connect-client.sh
```

This will:
1. Build the client binary (if needed)
2. Read the `.onion` address from the file
3. Connect to the server over Tor
4. Allow you to type messages

Once connected, type messages and press Enter to send them. All connected clients will see all messages.

## How It Works

### Server

The server:
- Creates a Tor hidden service on port 9999
- Accepts multiple client connections
- Broadcasts messages from any client to all other connected clients
- Displays connection and message activity

### Client

The client:
- Connects to the server's .onion address via Tor
- Sends messages you type to the server
- Displays messages from other clients
- Works from any computer with Tor access

### Address Sharing

The server saves its `.onion` address to `.onion_address` in the project directory.

**For same machine**: Just run `./connect-client.sh`

**For different machines**:
1. Copy the `.onion` address from the server terminal
2. On the client machine, run:
   ```bash
   cargo run --release --bin tor-msg-client <onion-address>:9999
   ```

Example:
```bash
cargo run --release --bin tor-msg-client abc123xyz456.onion:9999
```

## Example Session

**Server Terminal:**
```
=== Launching Tor Message Server ===

Building server binary...

Starting server...
The onion address will be saved to: .onion_address

----------------------------------------

=== Tor Message Server ===

[1/3] Bootstrapping to Tor network...
[1/3] ✓ Connected to Tor

[2/3] Launching hidden service...
[2/3] ✓ Hidden service launched

========================================
ONION_ADDRESS=abc123xyz456.onion
========================================

✓ Onion address saved to .onion_address

[3/3] Waiting for connections on port 9999...

Server is ready! Clients can connect to:
  abc123xyz456.onion:9999

Press Ctrl+C to shut down

[Server] Client 1 connected
[Server] Client 1 sent: Hello from client 1
[Server] Client 2 connected
[Server] Client 2 sent: Hi from client 2
```

**Client Terminal:**
```
=== Connecting to Tor Message Server ===

Connecting to: abc123xyz456.onion:9999

----------------------------------------

=== Tor Message Client ===

[1/3] Bootstrapping to Tor network...
[1/3] ✓ Connected to Tor

[2/3] Connecting to abc123xyz456.onion:9999...
[2/3] ✓ Connected to hidden service

[3/3] Connection established!

Type messages and press Enter to send.
Press Ctrl+C to quit.

Welcome! You are client #1. Type messages to broadcast.
Hello from client 1
[Client 1] Hello from client 1
[Client 2] Hi from client 2
```

## Security Notes

- **Current version**: Knowing the `.onion` address is the only security
- **No encryption**: Messages are sent in plaintext (but over Tor)
- **No authentication**: Anyone with the address can connect
- **Future versions** will add proper encryption and authentication

## Debugging

### Server won't start

- Check Tor connectivity: `cargo run --bin tor-check`
- Make sure port 9999 is available (not already in use)
- Check firewall settings

### Client can't connect

- Verify the `.onion` address is correct
- Wait 30-60 seconds after server starts (Tor needs time to publish)
- Check Tor connectivity: `cargo run --bin tor-check`
- Make sure you're using port 9999

### Connection is slow

- Tor hidden services can be slow to establish initially
- First connection may take 30-60 seconds
- Subsequent connections are usually faster

## Building Manually

If you want to build the binaries separately:

```bash
# Build both
cargo build --release --bin tor-msg-server --bin tor-msg-client

# Run server manually
./target/release/tor-msg-server

# Run client manually (replace with actual address)
./target/release/tor-msg-client <onion-address>:9999
```

## Files

- `src/bin/tor-msg-server.rs` - Server implementation
- `src/bin/tor-msg-client.rs` - Client implementation
- `launch-server.sh` - Convenience script to launch server
- `connect-client.sh` - Convenience script to connect client
- `.onion_address` - Generated file containing the server's address
