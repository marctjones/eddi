#!/bin/bash
set -e

echo "================================"
echo "EDDI Server Launcher"
echo "================================"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

BINARY="target/release/eddi"

# Check if the binary exists
if [ ! -f "$BINARY" ]; then
    echo -e "${RED}Error: eddi binary not found at $BINARY${NC}"
    echo ""
    echo "Please build the project first:"
    echo "  ./build.sh"
    echo ""
    exit 1
fi

# Check if Python dependencies are installed
FLASK_APP_DIR="test-apps/flask-demo"
if [ ! -d "$FLASK_APP_DIR/venv" ]; then
    echo -e "${YELLOW}Warning: Python virtual environment not found${NC}"
    echo "It's recommended to run ./build.sh first to set up dependencies"
    echo ""
fi

# Clean up any existing socket file
SOCKET_PATH="/tmp/eddi.sock"
if [ -S "$SOCKET_PATH" ]; then
    echo "Cleaning up existing socket file..."
    rm -f "$SOCKET_PATH"
fi

echo -e "${BLUE}Starting EDDI server...${NC}"
echo ""
echo "This will:"
echo "  1. Bootstrap connection to Tor network via Arti"
echo "  2. Launch a Tor v3 onion service"
echo "  3. Start the Flask web application on Unix Domain Socket"
echo "  4. Proxy connections from Tor to the Flask app"
echo ""
echo -e "${YELLOW}Note: This may take 30-60 seconds to become fully reachable${NC}"
echo ""
echo "----------------------------------------"
echo ""

# Run the eddi server
# The eddi binary will handle:
# - Starting gunicorn with the Flask app on UDS
# - Creating the onion service
# - Proxying connections
exec "$BINARY"
