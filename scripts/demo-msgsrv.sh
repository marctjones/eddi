#!/usr/bin/env bash
# Demo script for message server functionality

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

print_step() {
    echo -e "\n${BLUE}===>${NC} ${GREEN}$1${NC}\n"
}

print_info() {
    echo -e "${YELLOW}$1${NC}"
}

print_step "Building eddi-msgsrv..."
cd "$PROJECT_ROOT"
cargo build --bin eddi-msgsrv --quiet 2>&1 | grep -v "warning:" || true

MSGSRV="$PROJECT_ROOT/target/debug/eddi-msgsrv"

print_step "Demo 1: List Fortresses (should be empty)"
$MSGSRV list-fortresses

print_step "Demo 2: Show Status"
$MSGSRV status || true

print_step "Demo 3: List Connections (should be empty)"
$MSGSRV list-connections

print_step "Demo 4: Create Client Code"
print_info "Generating authentication code..."
CODE=$(cargo run --quiet --bin eddi-msgsrv -- help 2>&1 | grep "create-broker" | head -1 || echo "ABC-XYZ")
print_info "Example code: ABC-XYZ"

print_step "Demo 5: Simulate Connection"
print_info "In a real scenario:"
print_info "  1. Server creates fortress: eddi-msgsrv create-fortress --name my-server --ttl 5"
print_info "  2. Server creates broker: eddi-msgsrv create-broker --fortress my-server --namespace user@example.com"
print_info "  3. Server shares code (ABC-XYZ) out-of-band"
print_info "  4. Client connects: eddi-msgsrv connect --code ABC-XYZ --namespace user@example.com"
print_info "  5. Client sends: eddi-msgsrv send 'Hello, fortress!'"
print_info "  6. Client listens: eddi-msgsrv listen"

print_step "Demo 6: CLI Help"
$MSGSRV --help

print_step "✓ Demo complete!"
print_info "\nTo run a real fortress, use:"
print_info "  $MSGSRV create-fortress --name demo --ttl 5"
print_info "\nTo run a real broker, use:"
print_info "  $MSGSRV create-broker --fortress demo --namespace your@email.com"
print_info "\nTo connect as client, use:"
print_info "  $MSGSRV connect --code <CODE> --namespace your@email.com"
