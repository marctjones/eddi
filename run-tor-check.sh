#!/bin/bash
# run-tor-check.sh - Run the Tor connectivity check tool with logging
#
# This script runs the tor-check binary and outputs results both to the terminal
# and to a timestamped log file that can be accessed by LLM coding tools
# from different terminal sessions.
#
# The tor-check tool validates:
#   1. Bootstrap connection to Tor network
#   2. Access remote websites over Tor (clearnet)
#   3. Access existing Tor hidden services (.onion)
#   4. Publish Tor hidden services
#   5. Verify round-trip communication with own hidden service
#
# Usage:
#   ./run-tor-check.sh [options]
#
# Options:
#   --no-build    Skip building and run existing binary
#   --release     Build and run in release mode (faster, but slower to compile)
#
# Examples:
#   ./run-tor-check.sh              # Build and run in debug mode
#   ./run-tor-check.sh --release    # Build and run in release mode
#   ./run-tor-check.sh --no-build   # Run without rebuilding

set -euo pipefail

# Colors for terminal output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse command line arguments
BUILD=true
RELEASE_MODE=""
RELEASE_FLAG=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --no-build)
            BUILD=false
            shift
            ;;
        --release)
            RELEASE_MODE="--release"
            RELEASE_FLAG=" (release)"
            shift
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Usage: $0 [--no-build] [--release]"
            exit 1
            ;;
    esac
done

# Create logs directory if it doesn't exist
LOGS_DIR="logs"
mkdir -p "$LOGS_DIR"

# Generate timestamp for log file
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
LOG_FILE="$LOGS_DIR/tor-check-$TIMESTAMP.log"

# Also create a symlink to the latest log for easy access
LATEST_LOG="$LOGS_DIR/tor-check-latest.log"

echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}            Running Tor Connectivity Check Tool                 ${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${YELLOW}Log file:${NC} $LOG_FILE"
echo -e "${YELLOW}Latest log:${NC} $LATEST_LOG"
echo ""

# Write header to log file
cat > "$LOG_FILE" <<EOF
════════════════════════════════════════════════════════════════
         Tor Connectivity Check Tool - Run Log
════════════════════════════════════════════════════════════════
Timestamp: $(date)
Working Directory: $(pwd)
Build Mode: ${RELEASE_FLAG:-debug}
════════════════════════════════════════════════════════════════

EOF

# Build the binary if requested
if [ "$BUILD" = true ]; then
    echo -e "${BLUE}Building tor-check binary$RELEASE_FLAG...${NC}" | tee -a "$LOG_FILE"
    echo "" | tee -a "$LOG_FILE"

    if cargo build --bin tor-check $RELEASE_MODE 2>&1 | tee -a "$LOG_FILE"; then
        echo "" | tee -a "$LOG_FILE"
        echo -e "${GREEN}✅ Build successful${NC}" | tee -a "$LOG_FILE"
        echo "" | tee -a "$LOG_FILE"
    else
        BUILD_RESULT=$?
        echo "" | tee -a "$LOG_FILE"
        echo -e "${RED}❌ Build failed (exit code: $BUILD_RESULT)${NC}" | tee -a "$LOG_FILE"
        echo "" | tee -a "$LOG_FILE"
        ln -sf "$(basename "$LOG_FILE")" "$LATEST_LOG"
        exit $BUILD_RESULT
    fi
fi

echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}" | tee -a "$LOG_FILE"
echo -e "${BLUE}                  Starting Tor Checks                           ${NC}" | tee -a "$LOG_FILE"
echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}" | tee -a "$LOG_FILE"
echo "" | tee -a "$LOG_FILE"

# Run tor-check with output going to both terminal and log file
# The '2>&1' redirects stderr to stdout so errors are also captured
if cargo run --bin tor-check $RELEASE_MODE 2>&1 | tee -a "$LOG_FILE"; then
    CHECK_RESULT=0
else
    CHECK_RESULT=$?
fi

# Update the symlink to point to the latest log
ln -sf "$(basename "$LOG_FILE")" "$LATEST_LOG"

# Print summary
echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"

if [ $CHECK_RESULT -eq 0 ]; then
    echo -e "${GREEN}✅ All Tor connectivity checks passed!${NC}"
    EXIT_CODE=0
else
    echo -e "${RED}❌ Some Tor connectivity checks failed (exit code: $CHECK_RESULT)${NC}"
    EXIT_CODE=$CHECK_RESULT
fi

echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${YELLOW}Full output saved to:${NC} $LOG_FILE"
echo -e "${YELLOW}Quick access via:${NC} $LATEST_LOG"
echo ""
echo -e "${BLUE}To view the log:${NC}"
echo -e "  cat $LOG_FILE"
echo -e "  # or"
echo -e "  cat $LATEST_LOG"
echo ""
echo -e "${BLUE}Note:${NC} Tor connectivity checks may take several minutes to complete."
echo -e "      This is normal, especially for hidden service operations."
echo ""

exit $EXIT_CODE
