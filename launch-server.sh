#!/bin/bash
# Launch the Tor message server and save the onion address to a file

set -e

ONION_FILE=".onion_address"
BINARY="target/release/tor-msg-server"

echo "=== Launching Tor Message Server ==="
echo

# Build the binary if it doesn't exist
if [ ! -f "$BINARY" ]; then
    echo "Building server binary..."
    cargo build --release --bin tor-msg-server
    echo
fi

# Remove old onion address file if it exists
rm -f "$ONION_FILE"

echo "Starting server..."
echo "The onion address will be saved to: $ONION_FILE"
echo
echo "----------------------------------------"
echo

# Run the server and capture the first line of stdout (the onion address)
# while still showing all stderr output
$BINARY 2>&1 | while IFS= read -r line; do
    # The first line from stdout is the onion address
    if [ ! -f "$ONION_FILE" ]; then
        # Check if this looks like an onion address
        if [[ "$line" =~ \.onion$ ]]; then
            echo "$line" > "$ONION_FILE"
            echo "✓ Onion address saved to $ONION_FILE"
            echo
        fi
    fi
    # Print all output to terminal
    echo "$line"
done
