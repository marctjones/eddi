# EDDI Tools and Scripts Audit

## Current Inventory

### Rust Binaries (3)

| Binary | Purpose | Lines | Status |
|--------|---------|-------|--------|
| **eddi** | Main server - Launches Tor hidden service + proxies to UDS-bound app | ~350 | ✅ KEEP - Core |
| **tor-check** | Comprehensive Tor connectivity diagnostics (5 checks) | ~800 | ✅ KEEP - Support |
| **tor-http-client** | Pure Rust HTTP client via Tor (onion + clearnet) | ~300 | ✅ KEEP - Support |

### Shell Scripts (8)

| Script | Purpose | Size | Lines | Status |
|--------|---------|------|-------|--------|
| **build.sh** | Build all binaries + setup Python venv | - | 77 | ✅ KEEP - Development |
| **eddi-server** | Launch eddi with auto-cleanup, force mode, build checks | 2.8K | 101 | ✅ KEEP - Primary launcher |
| **eddi-connect** | Connect to onion service (simple wrapper) | 1.3K | 54 | ⚠️ CONSOLIDATE |
| **eddi-cleanup** | Interactive cleanup tool (processes/locks/sockets) | 4.1K | 145 | ✅ KEEP - Support |
| **start-server.sh** | Minimal server launcher (checks binary exists) | 1.6K | 62 | ❌ REMOVE - Duplicate |
| **tor-connect.sh** | Connect to onion (verbose, similar to eddi-connect) | 2.4K | 85 | ❌ REMOVE - Duplicate |
| **scripts/run-tests.sh** | Run cargo tests with logging to logs/ | - | 95 | ✅ KEEP - Development |
| **scripts/run-tor-check.sh** | Run tor-check with logging to logs/ | - | 153 | ✅ KEEP - Development |

### Test Scripts (2)

| Script | Purpose | Status |
|--------|---------|--------|
| **scripts/run-tests.sh** | Run all tests with timestamped logs | ✅ KEEP |
| **scripts/run-tor-check.sh** | Run tor-check with timestamped logs | ✅ KEEP |

## Issues Identified

### 1. Duplicate Server Launchers
- **eddi-server** (2.8K) - Full-featured with force mode, auto-build, cleanup
- **start-server.sh** (1.6K) - Minimal version, just checks binary exists
- **Problem**: Confusing to have two server launchers with different features
- **Recommendation**: Remove `start-server.sh`, keep `eddi-server`

### 2. Duplicate Client Connectors
- **eddi-connect** (1.3K) - Simple, clean interface
- **tor-connect.sh** (2.4K) - More verbose messaging, similar functionality
- **Problem**: Both do the exact same thing - run tor-http-client
- **Recommendation**: Merge into single `eddi-connect` with best features

### 3. Naming Inconsistency
- Some scripts: `eddi-*` (eddi-server, eddi-connect, eddi-cleanup)
- Others: `*.sh` extension (start-server.sh, tor-connect.sh, build.sh)
- **Problem**: Inconsistent naming convention
- **Recommendation**: Use `eddi-*` for user-facing tools, `*.sh` for development scripts

## Recommended Tool Suite

### Core Tools (User-Facing)
```
eddi-server          Launch Tor hidden service (main command)
eddi-connect         Connect to onion services via Tor
eddi-cleanup         Clean up processes/locks/sockets
```

### Development Tools
```
build.sh                    Build all binaries + setup environment
scripts/run-tests.sh        Run tests with logging
scripts/run-tor-check.sh    Run connectivity diagnostics with logging
```

### Rust Binaries
```
target/release/eddi              Main server (used by eddi-server)
target/release/tor-check         Diagnostics (used by run-tor-check.sh)
target/release/tor-http-client   HTTP client (used by eddi-connect)
```

## Proposed Changes

### 1. Remove Duplicates
```bash
rm start-server.sh       # Use eddi-server instead
rm tor-connect.sh        # Use eddi-connect instead
```

### 2. Enhance eddi-connect
Merge best features from both client scripts:
- Simple clean interface from eddi-connect
- Better error messages from tor-connect.sh
- Support for both onion + clearnet URLs (now that tor-http-client supports both)

### 3. Update Documentation
Create clear tool hierarchy:
- **Getting Started**: `build.sh` → `eddi-server` → `eddi-connect`
- **Diagnostics**: `eddi-cleanup`, `scripts/run-tor-check.sh`
- **Development**: `scripts/run-tests.sh`, cargo commands

## Final Tool Suite (Coherent & Minimal)

### User Commands
```
eddi-server              Start the Tor hidden service (primary command)
  --force                Auto-cleanup and force start

eddi-connect <url>       Connect to any URL via Tor
  Examples:
    eddi-connect http://example.onion/status
    eddi-connect https://check.torproject.org

eddi-cleanup             Clean up stale processes/locks/sockets
```

### Developer Commands
```
build.sh                       Build binaries + setup Python
scripts/run-tests.sh           Run unit tests with logging
scripts/run-tor-check.sh       Run Tor diagnostics with logging
  --release                    Use optimized build
  --no-build                   Skip rebuild
```

### Direct Binary Access (Advanced)
```
target/release/eddi              Low-level server binary
target/release/tor-check         Diagnostic tool
target/release/tor-http-client   HTTP client library
```

## Benefits of Cleanup

1. **Less Confusion**: One clear way to do each task
2. **Consistent Naming**: `eddi-*` for tools, `*.sh` for scripts
3. **Clear Separation**: User tools vs development tools
4. **Better Discovery**: Easier to find the right tool
5. **Maintainability**: Less code to maintain

## Migration Guide

Old Command → New Command:
```bash
./start-server.sh          →  ./eddi-server
./tor-connect.sh <url>     →  ./eddi-connect <url>
./build.sh                 →  ./build.sh (unchanged)
./eddi-cleanup             →  ./eddi-cleanup (unchanged)
```

## Summary

**Current State**: 8 shell scripts, some overlapping
**Proposed State**: 6 tools (3 user + 3 dev), no overlap
**Remove**: 2 duplicate scripts
**Enhance**: 1 script (eddi-connect)
**Result**: Coherent, minimal, maintainable toolset
