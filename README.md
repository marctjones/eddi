# eddi

A secure, self-contained application launcher that exposes web applications only as Tor hidden services.

## Overview

`eddi` is a Rust-based command-line tool that bridges web applications (Python, PHP, .NET, etc.) to the Tor network without exposing any TCP ports. It uses the Arti Tor library and Unix Domain Sockets to create a secure, isolated environment.

## Current Status

✅ **Complete** - All Tasks Finished!

- ✅ Task 1: Flask demo app specification
- ✅ Task 2: Arti "Hello World" proof of concept
- ✅ Task 3: UDS and child process management
- ✅ Task 4: Complete Arti-to-UDS bridge

See [GEMINI.md](GEMINI.md) for the full project plan.

## Quick Links

- **Project Plan**: [GEMINI.md](GEMINI.md)
- **Task 2 Documentation**: [TASK2.md](TASK2.md)
- **Task 3 Documentation**: [TASK3.md](TASK3.md)
- **Task 4 Documentation**: [TASK4.md](TASK4.md) - **Complete implementation!**
- **GitHub**: https://github.com/marctjones/eddi

## Building

```bash
cargo build --release
```

## Running

**Note**: Requires network access to connect to the Tor network.

### Check Tor Connectivity

Before running eddi, verify your system can connect to Tor:

```bash
cargo run --bin tor-check
```

This diagnostic tool will:
- Test DNS resolution
- Attempt to bootstrap to the Tor network
- Provide troubleshooting guidance if connection fails

### Setup

1. Install Python dependencies for the demo Flask app:
```bash
cd test-apps/flask-demo
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt
cd ../..
```

2. Run eddi:
```bash
RUST_LOG=info cargo run --release
```

3. Access your application via Tor Browser using the `.onion` address displayed

See [TASK4.md](TASK4.md) for complete usage documentation.

## Testing

```bash
# Unit tests
cargo test

# Network isolation tests (requires gunicorn)
cargo test -- --ignored
```

## License

MIT OR Apache-2.0