#!/bin/bash
# Development runner for eddi
# Builds and runs eddi with logging

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Create logs directory
mkdir -p logs

# Generate log filename
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_FILE="logs/eddi-run-${TIMESTAMP}.log"
LATEST_LOG="logs/eddi-run-latest.log"

echo -e "${BLUE}Building eddi application...${NC}"
cargo build

echo -e "${BLUE}Running eddi application...${NC}"
echo "Output will be logged to $LOG_FILE"

# Create symlink to latest log
ln -sf "$(basename "$LOG_FILE")" "$LATEST_LOG"

# Check for running instances
EDDI_PIDS=$(pgrep -f "target/debug/eddi" 2>/dev/null || true)

if [ -n "$EDDI_PIDS" ]; then
    echo -e "${YELLOW}⚠️  Warning: Found running eddi process(es): $EDDI_PIDS${NC}"
    echo ""
    read -p "Kill existing processes? (y/N) " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        for PID in $EDDI_PIDS; do
            kill -TERM $PID 2>/dev/null || kill -KILL $PID 2>/dev/null || true
        done
        sleep 1
        echo -e "${GREEN}✓ Processes killed${NC}"
    fi
fi

# Clean up Arti locks
ARTI_DIR="$HOME/.local/share/arti"
if [ -d "$ARTI_DIR" ]; then
    if find "$ARTI_DIR" -name "state.lock" 2>/dev/null | grep -q .; then
        echo -e "${YELLOW}⚠️  Found Arti lock files${NC}"
        read -p "Remove lock files? (Y/n) " -n 1 -r
        echo ""
        if [[ ! $REPLY =~ ^[Nn]$ ]]; then
            find "$ARTI_DIR" -name "state.lock" -delete 2>/dev/null || true
            echo -e "${GREEN}✓ Lock files removed${NC}"
        fi
    fi
fi

# Run in background and capture PID
cargo run 2>&1 | tee "$LOG_FILE" &
EDDI_PID=$!

echo "eddi application started with PID: $EDDI_PID"
echo "Press Ctrl+C to stop the eddi application."

# Setup trap to kill eddi on script exit
trap "echo ''; echo 'Stopping eddi...'; kill $EDDI_PID 2>/dev/null || true; exit 0" INT TERM

# Wait for the process
wait $EDDI_PID
