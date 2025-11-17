#!/bin/bash
# run-tests.sh - Run all unit and integration tests with logging
#
# This script runs all Cargo tests and outputs results both to the terminal
# and to a timestamped log file that can be accessed by LLM coding tools
# from different terminal sessions.
#
# Usage:
#   ./run-tests.sh [cargo test options]
#
# Examples:
#   ./run-tests.sh                    # Run all tests
#   ./run-tests.sh -- --nocapture     # Run with output capture disabled
#   ./run-tests.sh integration_tests  # Run specific test

set -euo pipefail

# Colors for terminal output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Create logs directory if it doesn't exist
LOGS_DIR="logs"
mkdir -p "$LOGS_DIR"

# Generate timestamp for log file
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
LOG_FILE="$LOGS_DIR/test-run-$TIMESTAMP.log"

# Also create a symlink to the latest log for easy access
LATEST_LOG="$LOGS_DIR/test-run-latest.log"

echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}                  Running Eddi Unit Tests                       ${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${YELLOW}Log file:${NC} $LOG_FILE"
echo -e "${YELLOW}Latest log:${NC} $LATEST_LOG"
echo ""

# Write header to log file
cat > "$LOG_FILE" <<EOF
════════════════════════════════════════════════════════════════
              Eddi Unit Tests - Run Log
════════════════════════════════════════════════════════════════
Timestamp: $(date)
Working Directory: $(pwd)
Command: cargo test $@
════════════════════════════════════════════════════════════════

EOF

echo -e "${BLUE}Running: ${NC}cargo test $@"
echo ""

# Run tests with output going to both terminal and log file
# We use 'tee' to split the output stream
# The '2>&1' redirects stderr to stdout so errors are also captured
if cargo test "$@" 2>&1 | tee -a "$LOG_FILE"; then
    TEST_RESULT=0
else
    TEST_RESULT=$?
fi

# Update the symlink to point to the latest log
ln -sf "$(basename "$LOG_FILE")" "$LATEST_LOG"

# Print summary
echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"

if [ $TEST_RESULT -eq 0 ]; then
    echo -e "${GREEN}✅ All tests passed!${NC}"
    EXIT_CODE=0
else
    echo -e "${RED}❌ Some tests failed (exit code: $TEST_RESULT)${NC}"
    EXIT_CODE=$TEST_RESULT
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

exit $EXIT_CODE
