#!/usr/bin/env bash
# Integration test script for message server

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

# Test results
TESTS_PASSED=0
TESTS_FAILED=0

# Print functions
print_test() {
    echo -e "${BLUE}[TEST]${NC} $1"
}

print_pass() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((TESTS_PASSED++))
}

print_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
    ((TESTS_FAILED++))
}

# Cleanup function
cleanup() {
    print_test "Cleaning up test environment..."
    rm -rf ~/.eddi/msgservers-test 2>/dev/null || true
    rm -f /tmp/eddi-msgsrv-test-*.sock 2>/dev/null || true
}

# Setup test environment
setup() {
    print_test "Setting up test environment..."
    cleanup
    export EDDI_MSGSRV_STATE_DIR=~/.eddi/msgservers-test
    mkdir -p "$EDDI_MSGSRV_STATE_DIR"
}

# Build project
build() {
    print_test "Building project..."
    cd "$PROJECT_ROOT"
    cargo build --quiet 2>&1 | grep -v "warning:" || true
}

# Test: Code generation
test_code_generation() {
    print_test "Testing code generation..."

    cd "$PROJECT_ROOT"
    cargo test --quiet --lib handshake::tests::test_generate_short_code 2>&1 | grep -q "test result: ok"

    if [ $? -eq 0 ]; then
        print_pass "Code generation works"
    else
        print_fail "Code generation failed"
    fi
}

# Test: Broker identifier
test_broker_identifier() {
    print_test "Testing broker identifier generation..."

    cd "$PROJECT_ROOT"
    cargo test --quiet --lib handshake::tests::test_broker_identifier_deterministic 2>&1 | grep -q "test result: ok"

    if [ $? -eq 0 ]; then
        print_pass "Broker identifier is deterministic"
    else
        print_fail "Broker identifier test failed"
    fi
}

# Test: Message queue
test_message_queue() {
    print_test "Testing message queue..."

    cd "$PROJECT_ROOT"
    cargo test --quiet --lib message::tests::test_message_queue 2>&1 | grep -q "test result: ok"

    if [ $? -eq 0 ]; then
        print_pass "Message queue works"
    else
        print_fail "Message queue test failed"
    fi
}

# Test: State management
test_state_management() {
    print_test "Testing state management..."

    cd "$PROJECT_ROOT"
    cargo test --quiet --lib storage::tests::test_state_manager 2>&1 | grep -q "test result: ok"

    if [ $? -eq 0 ]; then
        print_pass "State management works"
    else
        print_fail "State management test failed"
    fi
}

# Test: Client manager
test_client_manager() {
    print_test "Testing client manager..."

    cd "$PROJECT_ROOT"
    cargo test --quiet --lib client::tests::test_client_manager 2>&1 | grep -q "test result: ok"

    if [ $? -eq 0 ]; then
        print_pass "Client manager works"
    else
        print_fail "Client manager test failed"
    fi
}

# Test: CLI parsing
test_cli_parsing() {
    print_test "Testing CLI parsing..."

    cd "$PROJECT_ROOT"
    cargo test --quiet --lib cli::tests::test_cli_parsing 2>&1 | grep -q "test result: ok"

    if [ $? -eq 0 ]; then
        print_pass "CLI parsing works"
    else
        print_fail "CLI parsing test failed"
    fi
}

# Print results
print_results() {
    echo
    echo "================================"
    echo "Test Results:"
    echo "  Passed: $TESTS_PASSED"
    echo "  Failed: $TESTS_FAILED"
    echo "================================"

    if [ $TESTS_FAILED -eq 0 ]; then
        echo -e "${GREEN}All tests passed!${NC}"
        return 0
    else
        echo -e "${RED}Some tests failed!${NC}"
        return 1
    fi
}

# Main test runner
main() {
    echo "================================"
    echo "eddi Message Server Test Suite"
    echo "================================"
    echo

    setup
    build

    # Run tests
    test_code_generation
    test_broker_identifier
    test_message_queue
    test_state_management
    test_client_manager
    test_cli_parsing

    # Cleanup
    cleanup

    # Print results
    print_results
}

# Run tests
main
exit $?
