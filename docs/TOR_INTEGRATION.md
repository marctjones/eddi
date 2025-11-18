# Tor Integration - Implementation Plan

## Current Status

The eddi message server currently uses **Unix Domain Sockets** for communication. This provides:

✅ **Secure local communication** - Unix sockets are kernel-level IPC, isolated from network
✅ **Fast performance** - No network overhead
✅ **Process isolation** - Only processes with socket permissions can connect

## Security for Same-Machine Communication

**Q: When messages are sent from the same machine the server is running on, do they go over Tor?**

**A: Currently, NO - but this is actually secure for local use.**

### Current Architecture (Local)

```
Client (same machine) ←→ Unix Socket ←→ Fortress
                    (kernel IPC, not network)
```

- Unix Domain Sockets (`/tmp/eddi-msgsrv-*.sock`) are **not** network sockets
- Communication never touches the network stack
- Isolated at the kernel level (requires socket file permissions)
- Faster than Tor (no encryption/routing overhead)

### Planned Architecture (Remote via Tor)

```
Client (remote) ←→ Tor ←→ .onion ←→ Fortress
              (encrypted, anonymous, routed)
```

For remote access, all communication will go through Tor hidden services:
- Fortress gets a persistent `.onion` address
- Broker gets an ephemeral `.onion` address
- All traffic is encrypted and anonymized
- No IP addresses exposed

## Why Not Use Tor for Local Communication?

1. **Unnecessary overhead** - Tor adds ~300-500ms latency for circuit building
2. **Security equivalence** - Unix sockets are already isolated and secure
3. **Performance** - Local IPC is orders of magnitude faster
4. **Resource usage** - No need to bootstrap Tor for local-only use

## Tor Integration Roadmap

### Phase 1: Foundation ✅ (Complete)

- [x] Message server core
- [x] Unix socket communication
- [x] State management
- [x] CLI commands
- [x] Tests

### Phase 2: Tor Integration 🚧 (In Progress)

#### 2.1 Fortress Onion Services

```rust
// Create fortress with Tor
let tor_client = TorClient::create_bootstrapped().await?;
let (onion_service, request_stream) = tor_client
    .launch_onion_service(config)
    .await?;

// Get .onion address
let onion_addr = onion_service.onion_address();
// Example: abc123def456.onion
```

#### 2.2 Broker Onion Services (Ephemeral)

```rust
// Create ephemeral broker
let broker_onion = create_ephemeral_onion_service(
    &tor_client,
    &short_code,
    timeout: Duration::from_secs(120)
).await?;

// Broker exists only for 2 minutes
// Auto-shuts down after handshake
```

#### 2.3 Client Tor Connector

```rust
// Client connects via Tor
let stream = tor_client
    .connect((onion_address, 80))
    .await?;

// All communication encrypted & anonymous
```

#### 2.4 Hybrid Mode

```rust
// Fortress listens on BOTH
enum ListenMode {
    UnixSocket(PathBuf),      // Fast local access
    OnionService(String),      // Secure remote access
    Both(PathBuf, String),     // Hybrid (recommended)
}
```

### Phase 3: Advanced Features 🔮 (Planned)

#### 3.1 Tor Client Authorization (Stealth Mode)

```rust
// Fortress is invisible without client key
let client_auth_key = fortress.generate_client_auth()?;

// Only clients with this key can even *see* the fortress
fortress.authorize_client(&client_auth_key)?;
```

#### 3.2 Multi-Hop Routing

```rust
// Route through multiple Tor circuits
let circuit_config = CircuitConfig {
    hops: 5,  // Extra hops for anonymity
    guard_selection: GuardSelection::Random,
};
```

#### 3.3 Onion Service v3 Features

- Ed25519 cryptography
- 56-character addresses
- Improved directory protocol
- Client authorization built-in

## Implementation Details

### Arti Integration

eddi uses **Arti** (Rust Tor implementation) for onion services:

```toml
[dependencies]
arti-client = { version = "0.36", features = ["onion-service-service"] }
tor-hsservice = "0.36"
tor-rtcompat = "0.36"
```

### Code Structure

```
src/msgserver/
├── tor.rs           # Tor client wrapper (new)
├── server.rs        # Add onion service support
├── client.rs        # Add Tor connector
└── ...
```

### API Changes

```rust
// Before
eddi-msgsrv create-fortress --name my-server --ttl 5

// After (with Tor)
eddi-msgsrv create-fortress --name my-server --ttl 5 --onion
# Output: Onion address: abc123def456.onion

// Connect via Tor
eddi-msgsrv connect --onion abc123def456.onion --code XYZ-123
```

## Testing Plan

### Unit Tests

```rust
#[tokio::test]
async fn test_tor_bootstrap() {
    let tor = TorManager::new(key_dir).await?;
    assert!(tor.is_bootstrapped());
}

#[tokio::test]
async fn test_onion_service_creation() {
    let (addr, stream) = tor.create_onion_service("test").await?;
    assert!(addr.ends_with(".onion"));
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_end_to_end_tor() {
    // Create fortress with Tor
    let fortress = create_fortress("test", true).await?;

    // Create broker
    let broker = create_broker(&fortress.onion_addr).await?;

    // Client connects via Tor
    let client = connect_via_tor(&broker.code).await?;

    // Send message
    client.send("Hello over Tor!").await?;

    // Verify received
    let msg = fortress.receive().await?;
    assert_eq!(msg.content, "Hello over Tor!");
}
```

## Security Considerations

### Threat Model

**Without Tor (Unix Sockets):**
- ✅ Protects against: Network sniffing, remote attacks
- ❌ Does NOT protect against: Local privilege escalation, kernel exploits
- **Use case**: Single-machine, trusted environment

**With Tor (Onion Services):**
- ✅ Protects against: IP tracking, traffic analysis, censorship, network monitoring
- ✅ Provides: Anonymity, encryption, authentication
- **Use case**: Multi-machine, untrusted networks, remote access

### Best Practices

1. **Local-only deployments**: Use Unix sockets (current)
2. **Remote access**: Use Tor onion services (coming soon)
3. **Hybrid deployments**: Enable both for flexibility
4. **High-security**: Add client authorization (stealth mode)

## Timeline

- **Q1 2024**: Basic Tor integration (fortress + broker onions)
- **Q2 2024**: Client Tor connector
- **Q3 2024**: Hybrid mode (Unix + Tor)
- **Q4 2024**: Client authorization (stealth mode)

## References

- [Arti Documentation](https://docs.rs/arti-client/)
- [Tor Onion Services](https://community.torproject.org/onion-services/)
- [Unix Domain Sockets](https://man7.org/linux/man-pages/man7/unix.7.html)

## FAQ

**Q: Is Unix socket secure enough?**
A: Yes, for local communication. Unix sockets are kernel-level IPC and never touch the network.

**Q: When should I use Tor?**
A: When you need remote access, anonymity, or communication across untrusted networks.

**Q: Can I use both?**
A: Yes! Hybrid mode (coming soon) will support both Unix sockets (fast local) and Tor (secure remote).

**Q: What about performance?**
A: Unix sockets: <1ms latency. Tor: ~300-500ms (circuit building) + network latency.

**Q: Is Tor required?**
A: No. For local-only use, Unix sockets are sufficient and faster.
