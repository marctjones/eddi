# eddi

A secure, self-contained application launcher that exposes web applications only as Tor hidden services.

## Overview

`eddi` is a Rust-based command-line tool that bridges web applications (Python, PHP, .NET, etc.) to the Tor network without exposing any TCP ports. It uses the Arti Tor library and Unix Domain Sockets to create a secure, isolated environment.

## Current Status

🚧 **In Development** - Task 3 Complete

- ✅ Task 1: Flask demo app specification
- ✅ Task 2: Arti "Hello World" proof of concept
- ✅ Task 3: UDS and child process management
- ⬜ Task 4: Complete Arti-to-UDS bridge

See [GEMINI.md](GEMINI.md) for the full project plan.

## Quick Links

- **Project Plan**: [GEMINI.md](GEMINI.md)
- **Task 2 Documentation**: [TASK2.md](TASK2.md)
- **Task 3 Documentation**: [TASK3.md](TASK3.md)
- **GitHub**: https://github.com/marctjones/eddi

## Building

```bash
cargo check    # Verify compilation
cargo test     # Run tests
cargo build    # Build the binary
```

## License

MIT OR Apache-2.0