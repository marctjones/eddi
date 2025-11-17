#!/bin/bash
# Connect to the Tor message server using the saved onion address

set -e

ONION_FILE=".onion_address"
BINARY="target/release/tor-msg-client"

echo "=== Connecting to Tor Message Server ==="
echo

# Build the binary if it doesn't exist
if [ ! -f "$BINARY" ]; then
    echo "Building client binary..."
    cargo build --release --bin tor-msg-client
    echo
fi

# Check if onion address file exists
if [ ! -f "$ONION_FILE" ]; then
    echo "Error: Onion address file not found: $ONION_FILE"
    echo
    echo "Please start the server first using ./launch-server.sh"
    echo "Or provide the onion address manually:"
    echo "  $BINARY <onion-address>:9999"
    exit 1
fi

# Read the onion address
ONION_ADDR=$(cat "$ONION_FILE")

echo "Connecting to: $ONION_ADDR:9999"
echo
echo "----------------------------------------"
echo

# Run the client
$BINARY "$ONION_ADDR:9999"
