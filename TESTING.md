# Testing Guide

This document describes the test suite organization, best practices, and coverage for the eddi project.

## Table of Contents

- [Overview](#overview)
- [Test Organization](#test-organization)
- [Running Tests](#running-tests)
- [Test Coverage](#test-coverage)
- [Writing Tests](#writing-tests)
- [CI/CD Integration](#cicd-integration)

## Overview

The eddi test suite follows Rust testing best practices with:
- **Unit tests**: Testing individual components in isolation
- **Integration tests**: Testing component interactions
- **Property-based tests**: Testing invariants across many inputs
- **Ignored tests**: Tests requiring external dependencies (gunicorn, network)

### Test Philosophy

1. **Fast by default**: Most tests run quickly without external dependencies
2. **Comprehensive coverage**: All public APIs and critical paths tested
3. **Clear failure messages**: Tests provide actionable error information
4. **Maintainable**: Tests are well-organized and documented

## Test Organization

### Directory Structure

```
eddi/
├── src/
│   ├── lib.rs                  # Module exports
│   ├── process.rs              # Unit tests inline (#[cfg(test)])
│   ├── main.rs                 # Unit tests inline
│   └── bin/
│       ├── task3.rs            # Unit tests inline
│       └── tor-check.rs        # Tested via integration tests
└── tests/
    ├── test_utils.rs           # Shared test utilities
    ├── process_tests.rs        # Process management tests
    ├── integration_tests.rs    # End-to-end tests
    ├── tor_check_tests.rs      # Diagnostic tool tests
    └── network_isolation_test.rs  # Security tests
```

### Test Categories

#### 1. Unit Tests (Inline)

Located in source files with `#[cfg(test)]`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Test implementation
    }
}
```

**Examples:**
- `src/process.rs`: ProcessConfig creation
- `src/main.rs`: EddiConfig defaults
- `src/bin/task3.rs`: HTTP parsing

#### 2. Integration Tests (`tests/`)

Test component interactions:

```rust
// tests/integration_tests.rs
#[test]
fn test_full_workflow() {
    // Test multiple components together
}
```

#### 3. Ignored Tests

Tests requiring external dependencies:

```rust
#[test]
#[ignore]  // Run with: cargo test -- --ignored
fn test_with_gunicorn() {
    // Requires gunicorn installed
}
```

#### 4. Async Tests

Tests using Tokio runtime:

```rust
#[tokio::test]
async fn test_async_feature() {
    // Async test implementation
}
```

## Running Tests

### Quick Test Commands

```bash
# Run all unit tests (fast)
cargo test

# Run all tests including ignored ones
cargo test -- --ignored

# Run specific test file
cargo test --test integration_tests

# Run specific test
cargo test test_gunicorn_config

# Run with output
cargo test -- --nocapture

# Run with specific log level
RUST_LOG=debug cargo test

# Run tests in parallel (default)
cargo test

# Run tests serially
cargo test -- --test-threads=1
```

### Test Categories

```bash
# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test '*'

# Specific integration test file
cargo test --test process_tests

# Binary tests
cargo test --bins

# Documentation tests
cargo test --doc
```

### Network Isolation Tests

```bash
# Critical security tests (require gunicorn)
cargo test --test network_isolation_test -- --ignored

# These verify NO TCP/UDP ports are opened
cargo test test_no_tcp_sockets_opened -- --ignored
cargo test test_no_udp_sockets_opened -- --ignored
```

## Test Coverage

### Current Coverage

**Unit Tests:**
- ✅ ProcessConfig::gunicorn creation
- ✅ ProcessConfig cloning
- ✅ ProcessConfig debug formatting
- ✅ EddiConfig defaults
- ✅ HTTP response parsing
- ✅ Test utilities

**Integration Tests:**
- ✅ Full project compilation
- ✅ Binary builds
- ✅ Socket path generation
- ✅ Process config invariants
- ✅ Public API accessibility
- ⚠️ Flask on UDS (requires gunicorn) - ignored
- ⚠️ Process spawning (requires commands) - ignored

**Security Tests:**
- ✅ TCP socket parsing
- ⚠️ NO TCP sockets verification (requires gunicorn) - ignored
- ⚠️ NO UDP sockets verification (requires gunicorn) - ignored
- ⚠️ Unix socket communication (requires gunicorn) - ignored

**Tool Tests:**
- ✅ tor-check compilation
- ✅ tor-check version info
- ✅ tor-check exit codes
- ✅ DNS resolution check
- ✅ HOME directory check

### Coverage Metrics

To generate coverage report (requires `tarpaulin`):

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage
cargo tarpaulin --out Html --output-dir coverage

# Generate and upload to Codecov
cargo tarpaulin --out Xml
```

### Coverage Goals

- **Target**: 80%+ coverage on core logic
- **Critical paths**: 100% coverage (process management, security checks)
- **Main binary**: 60%+ (hard to test without network)

## Writing Tests

### Test Naming Conventions

```rust
// Unit test: test_<component>_<behavior>
#[test]
fn test_config_creation() { }

#[test]
fn test_process_spawning_fails_on_invalid_command() { }

// Integration test: test_<feature>_<scenario>
#[test]
fn test_full_workflow_with_flask_app() { }

// Async test
#[tokio::test]
async fn test_async_socket_connection() { }

// Ignored test
#[test]
#[ignore]
fn test_requires_external_dependency() { }
```

### Test Structure

Follow the **Arrange-Act-Assert** pattern:

```rust
#[test]
fn test_example() {
    // Arrange: Set up test data
    let config = ProcessConfig::gunicorn(
        PathBuf::from("/tmp/test.sock"),
        PathBuf::from("/app"),
        "app:app",
        2,
    );

    // Act: Perform the action
    let result = config.command;

    // Assert: Verify the outcome
    assert_eq!(result, "gunicorn");
}
```

### Testing Best Practices

#### 1. Use Test Utilities

```rust
use test_utils::*;

#[test]
fn test_with_temp_dir() {
    let dir = temp_dir();
    // Use temporary directory
    // Automatically cleaned up when dropped
}
```

#### 2. Clean Up Resources

```rust
#[test]
fn test_with_socket() {
    let socket_path = temp_socket_path();

    // ... use socket ...

    // Always clean up
    cleanup_socket(&socket_path);
}
```

#### 3. Test Error Cases

```rust
#[test]
fn test_spawn_nonexistent_command() {
    let config = ProcessConfig {
        command: "does-not-exist".to_string(),
        // ...
    };

    let result = ChildProcessManager::spawn(&config);

    assert!(result.is_err(), "Should fail gracefully");
}
```

#### 4. Use Assertions Effectively

```rust
// Good: Descriptive message
assert!(value > 0, "Value should be positive, got {}", value);

// Good: Specific assertion
assert_eq!(actual, expected);

// Avoid: Generic assertion
assert!(result.is_ok());  // What went wrong if it fails?
```

#### 5. Test Invariants

```rust
#[test]
fn test_socket_path_invariants() {
    for _ in 0..100 {
        let path = temp_socket_path();

        // Invariant: Always has .sock extension
        assert_eq!(path.extension(), Some(OsStr::new("sock")));

        // Invariant: Always in temp directory
        assert!(path.starts_with(std::env::temp_dir()));
    }
}
```

### Async Testing

```rust
#[tokio::test]
async fn test_async_function() {
    use tokio::time::{timeout, Duration};

    let result = timeout(
        Duration::from_secs(5),
        some_async_function()
    ).await;

    assert!(result.is_ok(), "Should complete within timeout");
}
```

### Mock Data

```rust
// Create test fixtures
pub fn mock_config() -> ProcessConfig {
    ProcessConfig {
        socket_path: PathBuf::from("/tmp/test.sock"),
        app_dir: PathBuf::from("/tmp"),
        command: "echo".to_string(),
        args: vec!["test".to_string()],
    }
}

#[test]
fn test_with_mock() {
    let config = mock_config();
    assert_eq!(config.command, "echo");
}
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run tests
        run: cargo test --all

      - name: Run ignored tests
        run: |
          sudo apt-get update
          sudo apt-get install -y python3 python3-pip
          pip3 install flask gunicorn
          cargo test -- --ignored

      - name: Check code coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out Xml

      - name: Upload coverage
        uses: codecov/codecov-action@v3
```

### Pre-commit Hooks

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Run tests before commit
cargo test

if [ $? -ne 0 ]; then
    echo "Tests failed. Commit aborted."
    exit 1
fi
```

## Troubleshooting Tests

### Common Issues

#### Tests Hang

```bash
# Run with timeout
timeout 60 cargo test

# Check for infinite loops in async code
RUST_LOG=debug cargo test -- --nocapture
```

#### Tests Fail Inconsistently

```bash
# Run serially to avoid race conditions
cargo test -- --test-threads=1

# Check for shared state or resource conflicts
```

#### Ignored Tests Fail

```bash
# Install dependencies
sudo apt-get install python3 python3-pip
pip3 install flask gunicorn

# Verify commands available
which gunicorn python3

# Run specific ignored test
cargo test test_no_tcp_sockets_opened -- --ignored --nocapture
```

### Debugging Tests

```rust
#[test]
fn test_debug() {
    // Print debug info
    eprintln!("Debug: value = {:?}", value);

    // Use dbg! macro
    dbg!(&config);

    // Add temporary assertions
    assert!(false, "Stop here to see output");
}
```

Run with output:
```bash
cargo test test_debug -- --nocapture
```

## Test Documentation

### Documenting Tests

```rust
/// Tests that ProcessConfig correctly generates gunicorn arguments
///
/// This test verifies:
/// - Command is set to "gunicorn"
/// - Workers argument is included
/// - Bind argument uses unix: prefix
/// - App module is in arguments
#[test]
fn test_gunicorn_config() {
    // Test implementation
}
```

### Test TODOs

Mark incomplete tests:

```rust
#[test]
#[ignore]
fn test_future_feature() {
    todo!("Implement when feature X is added");
}
```

## Performance Testing

### Benchmark Tests

```rust
#[test]
fn test_performance() {
    use std::time::Instant;

    let start = Instant::now();

    // Operation to benchmark
    for _ in 0..1000 {
        let _ = ProcessConfig::gunicorn(
            PathBuf::from("/tmp/test.sock"),
            PathBuf::from("/app"),
            "app:app",
            2,
        );
    }

    let elapsed = start.elapsed();
    println!("1000 configs created in {:?}", elapsed);

    // Should be fast
    assert!(elapsed.as_millis() < 100);
}
```

### Criterion Benchmarks (Optional)

Add to `Cargo.toml`:

```toml
[dev-dependencies]
criterion = "0.5"

[[bench]]
name = "process_bench"
harness = false
```

Create `benches/process_bench.rs`:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_config_creation(c: &mut Criterion) {
    c.bench_function("create_gunicorn_config", |b| {
        b.iter(|| {
            ProcessConfig::gunicorn(
                black_box(PathBuf::from("/tmp/test.sock")),
                black_box(PathBuf::from("/app")),
                "app:app",
                2,
            )
        });
    });
}

criterion_group!(benches, benchmark_config_creation);
criterion_main!(benches);
```

Run:
```bash
cargo bench
```

## Continuous Improvement

### Adding New Tests

When adding features:

1. **Write tests first** (TDD)
2. **Test edge cases**
3. **Test error conditions**
4. **Update documentation**
5. **Run full test suite**

### Reviewing Tests

In code review, check:

- [ ] All new code has tests
- [ ] Tests are clear and maintainable
- [ ] Edge cases are covered
- [ ] Error handling is tested
- [ ] Resources are cleaned up
- [ ] Tests run quickly
- [ ] Ignored tests are documented

## Summary

- **Run tests frequently**: `cargo test`
- **Keep tests fast**: Use `#[ignore]` for slow tests
- **Test edge cases**: Don't just test the happy path
- **Clean up resources**: Use RAII and cleanup utilities
- **Document tests**: Explain what and why
- **Maintain coverage**: Aim for 80%+ on core logic

For questions or issues with tests, see the main documentation or open an issue.
