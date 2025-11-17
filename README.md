# eddi

**A secure application launcher that exposes web apps only as Tor hidden services.**

No TCP ports. No IP exposure. Pure Tor connectivity via Arti (Rust Tor implementation).

---

## Quick Start

### 1. Start the Server

```bash
./eddi-server
```

This will:
- Build binaries (first time only)
- Bootstrap to Tor network
- Launch a Tor v3 onion service
- Start the Flask demo app
- Display your `.onion` address

**Note:** First start takes 30-60 seconds to bootstrap to Tor.

### 2. Connect via Tor

In another terminal:

```bash
./eddi-connect
```

This pure Tor client will:
- Auto-detect your server's onion address
- Connect through Tor network (no proxies)
- Fetch the web page

Or connect to any URL via Tor:

```bash
# Tor hidden services (.onion)
./eddi-connect http://example.onion:80
./eddi-connect example.onion/status

# Regular websites (via Tor anonymously)
./eddi-connect https://check.torproject.org
./eddi-connect http://httpbin.org/ip
```

---

## What Makes EDDI Special

- **Pure Tor** - Uses Arti (Rust Tor implementation), not proxy servers
- **No IP Exposure** - Never uses IP-based protocols
- **Unix Domain Sockets** - Secure inter-process communication
- **Zero TCP Ports** - No network ports exposed on your system
- **Process Isolation** - Managed child processes

---

## Manual Build & Test

### Build Everything

```bash
./build.sh
```

Builds:
- Rust binaries (eddi, tor-check, tor-http-client)
- Python virtual environment
- Flask demo dependencies

### Run Tests

```bash
./scripts/run-tests.sh
```

Runs comprehensive test suite with detailed logging.

### Check Tor Connectivity

```bash
./scripts/run-tor-check.sh
# or
cargo run --bin tor-check
```

Diagnostic tool that verifies:
- DNS resolution
- Tor network bootstrap
- Hidden service connections
- Network isolation

---

## Advanced Usage

### Manual HTTP Client

Connect to any URL via Tor directly:

```bash
# Tor hidden services
cargo run --release --bin tor-http-client http://example.onion:80

# Regular websites (anonymized via Tor)
cargo run --release --bin tor-http-client https://check.torproject.org
```

### Environment Variables

```bash
# Enable debug logging
RUST_LOG=debug ./eddi-server

# Enable trace logging
RUST_LOG=trace cargo run --bin tor-check
```

---

## Project Structure

```
eddi/
├── eddi-server          # Launch Tor hidden service (main command)
├── eddi-connect         # Connect to any URL via Tor (.onion + clearnet)
├── eddi-cleanup         # Clean up processes/locks/sockets
├── build.sh             # Build all binaries + setup environment
├── src/
│   ├── main.rs          # Main EDDI application
│   ├── process.rs       # Process management
│   └── bin/             # Additional binaries
│       ├── tor-check.rs        # Tor diagnostics tool
│       └── tor-http-client.rs  # Pure Arti HTTP client
├── scripts/             # Development utilities
│   ├── run-tests.sh            # Test runner with logging
│   └── run-tor-check.sh        # Diagnostics runner with logging
├── docs/                # Documentation
└── test-apps/           # Demo applications
    └── flask-demo/      # TorPaste - Flask pastebin demo
```

---

## Troubleshooting

### "State already locked" Error

If you see `State already locked` or `Another process is managing the directory`:

**Quick Fix:**
```bash
# Option 1: Use force mode (kills old processes automatically)
./eddi-server --force

# Option 2: Use the cleanup tool (interactive)
./eddi-cleanup
```

**What's happening:**
- Another eddi instance is still running, or
- A previous instance crashed and left lock files behind

**Manual cleanup:**
```bash
# 1. Find and kill running processes
pgrep -f eddi
kill <PID>

# 2. Remove Arti lock files
find ~/.local/share/arti -name "state.lock" -delete

# 3. Clean up sockets
rm -f /tmp/eddi*.sock
```

### DNS Resolution Issues

If you see "failed to resolve" errors:

```bash
# Check DNS configuration
cat /etc/resolv.conf

# Add DNS servers if empty
echo "nameserver 8.8.8.8" | sudo tee /etc/resolv.conf
```

### Tor Bootstrap Timeout

If Tor takes too long to bootstrap:

1. Check network connectivity
2. Verify DNS is working
3. Check firewall allows outbound connections
4. Try with debug logging: `RUST_LOG=debug ./eddi-server`

### Python/Flask Issues

If Flask demo fails:

```bash
# Rebuild Python environment
cd test-apps/flask-demo
rm -rf venv
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt
```

---

## Documentation

- **[TESTING.md](docs/TESTING.md)** - Testing guide
- **[DEPLOYMENT.md](docs/DEPLOYMENT.md)** - Deployment guide
- **[SECURITY.md](docs/SECURITY.md)** - Security documentation
- **[GEMINI.md](docs/GEMINI.md)** - Original project plan

---

## Requirements

- **Rust** - Install from [rustup.rs](https://rustup.rs/)
- **Python 3** - For Flask demo app
- **Network Access** - To connect to Tor directory authorities

---

## License

MIT OR Apache-2.0
