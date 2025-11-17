#!/bin/bash
set -e

echo "================================"
echo "EDDI Build Script"
echo "================================"
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check for Rust/Cargo
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo not found. Please install Rust from https://rustup.rs/"
    exit 1
fi

# Check for Python
if ! command -v python3 &> /dev/null; then
    echo "Error: python3 not found. Please install Python 3"
    exit 1
fi

echo -e "${BLUE}Step 1: Building Rust binaries...${NC}"
cargo build --release

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Rust binaries built successfully${NC}"
    echo "  Binaries available in: target/release/"
    echo "  - eddi (main application)"
    echo "  - task3 (UDS demo)"
    echo "  - tor-check (Tor diagnostics)"
    echo "  - tor-msg-server (Tor message server)"
    echo "  - tor-msg-client (Tor message client)"
else
    echo "Error: Rust build failed"
    exit 1
fi

echo ""
echo -e "${BLUE}Step 2: Setting up Python environment for Flask demo...${NC}"
cd test-apps/flask-demo

# Create virtual environment if it doesn't exist
if [ ! -d "venv" ]; then
    echo "Creating Python virtual environment..."
    python3 -m venv venv
fi

# Activate virtual environment and install dependencies
source venv/bin/activate
pip install -q --upgrade pip
pip install -q -r requirements.txt

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Python dependencies installed${NC}"
else
    echo "Error: Python dependency installation failed"
    deactivate
    exit 1
fi

deactivate
cd ../..

echo ""
echo -e "${GREEN}================================${NC}"
echo -e "${GREEN}Build completed successfully!${NC}"
echo -e "${GREEN}================================${NC}"
echo ""
echo "Next steps:"
echo "  - Run tests: ./scripts/run-tests.sh"
echo "  - Start server: ./start-server.sh"
echo "  - Check Tor connectivity: ./scripts/run-tor-check.sh"
echo ""
