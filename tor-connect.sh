#!/bin/bash
# Connect to EDDI server via Tor using Arti (no proxies, pure Tor connection)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

ONION_FILE=".onion_address"
BINARY="target/release/tor-http-client"

echo "========================================="
echo "EDDI Tor Client (Pure Arti Connection)"
echo "========================================="
echo ""
echo "This client:"
echo "  ✓ Uses Arti Tor library (not proxy servers)"
echo "  ✓ Connects directly via Tor network"
echo "  ✓ Never uses IP-based protocols"
echo "  ✓ Pure onion-to-onion communication"
echo ""
echo "=========================================";
echo ""

# Build the binary if it doesn't exist
if [ ! -f "$BINARY" ]; then
    echo -e "${BLUE}Building tor-http-client binary...${NC}"
    cargo build --release --bin tor-http-client
    echo ""
fi

# Check if user provided onion address as argument
if [ $# -eq 1 ]; then
    ONION_ADDR="$1"
    echo -e "${GREEN}Using provided address: $ONION_ADDR${NC}"
    echo ""
# Otherwise check for saved onion address file
elif [ -f "$ONION_FILE" ]; then
    ONION_ADDR=$(cat "$ONION_FILE")
    echo -e "${GREEN}Using saved address from $ONION_FILE${NC}"
    echo -e "${GREEN}Address: $ONION_ADDR${NC}"
    echo ""
else
    echo -e "${RED}Error: No onion address provided${NC}"
    echo ""
    echo "Usage:"
    echo "  $0 <onion-address>"
    echo ""
    echo "Example:"
    echo "  $0 http://example.onion:80"
    echo "  $0 example.onion:80/status"
    echo ""
    echo "Or start the EDDI server first (./start-server.sh)"
    echo "which will save the address to $ONION_FILE"
    echo ""
    exit 1
fi

# Add http:// prefix if not present
if [[ ! "$ONION_ADDR" =~ ^https?:// ]]; then
    ONION_ADDR="http://$ONION_ADDR"
fi

# Add port :80 if not present
if [[ ! "$ONION_ADDR" =~ :[0-9]+(/|$) ]]; then
    # Insert :80 before the path or at the end
    ONION_ADDR=$(echo "$ONION_ADDR" | sed 's|\(\.onion\)\(/\|$\)|\1:80\2|')
fi

echo -e "${BLUE}Connecting to: $ONION_ADDR${NC}"
echo ""
echo -e "${YELLOW}Note: First connection may take 10-30 seconds${NC}"
echo -e "${YELLOW}      (bootstrapping Tor network)${NC}"
echo ""
echo "----------------------------------------"
echo ""

# Run the client
# This uses ONLY Arti - no proxy servers, no IP protocols
exec "$BINARY" "$ONION_ADDR"
