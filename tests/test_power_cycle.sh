#!/bin/bash
# Integration test for powermon - tests full on/off/status cycle
# Run from project root: ./tests/test_power_cycle.sh
#
# Requirements:
# - Must be run in TTY or Wayland environment
# - Binary must be built: cargo build --release

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
POWERMON="$PROJECT_ROOT/target/release/powermon"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

pass() { echo -e "${GREEN}✓ PASS${NC}: $1"; }
fail() { echo -e "${RED}✗ FAIL${NC}: $1"; exit 1; }
info() { echo -e "${YELLOW}→${NC} $1"; }

# Check binary exists
if [[ ! -x "$POWERMON" ]]; then
    echo "Error: powermon binary not found at $POWERMON"
    echo "Run: cargo build --release"
    exit 1
fi

# Cleanup function
cleanup() {
    info "Cleaning up..."
    pkill -9 powermon 2>/dev/null || true
    rm -f "/run/user/$(id -u)/powermon.pid" 2>/dev/null || true
    sleep 0.2
}

# Cleanup on exit
trap cleanup EXIT

echo "=========================================="
echo "  powermon Integration Test"
echo "=========================================="
echo ""

# Initial cleanup
cleanup

# Test 1: Help command
info "Testing --help..."
OUTPUT=$("$POWERMON" --help 2>&1)
echo "$OUTPUT" | grep -q "Control monitor power state" || fail "--help missing description"
echo "$OUTPUT" | grep -q "on" || fail "--help missing 'on' command"
echo "$OUTPUT" | grep -q "off" || fail "--help missing 'off' command"
echo "$OUTPUT" | grep -q "status" || fail "--help missing 'status' command"
pass "--help shows all commands"

# Test 2: Initial status (should be On after cleanup)
info "Testing initial status..."
OUTPUT=$("$POWERMON" status 2>&1)
EXIT_CODE=$?
[[ $EXIT_CODE -eq 0 ]] || fail "status command failed with exit code $EXIT_CODE"
echo "$OUTPUT" | grep -q "On" || fail "Initial status should be 'On', got: $OUTPUT"
pass "Initial status is On"

# Test 3: Turn display off
info "Testing 'off' command..."
"$POWERMON" off
EXIT_CODE=$?
[[ $EXIT_CODE -eq 0 ]] || fail "off command failed with exit code $EXIT_CODE"
pass "Display turned off (exit code 0)"

sleep 0.5

# Test 4: Status should now be Off
info "Testing status after off..."
OUTPUT=$("$POWERMON" status 2>&1)
echo "$OUTPUT" | grep -q "Off" || fail "Status after off should be 'Off', got: $OUTPUT"
pass "Status is Off after 'off' command"

# Test 5: JSON output
info "Testing JSON output..."
OUTPUT=$("$POWERMON" status --json 2>&1)
echo "$OUTPUT" | grep -q '"power":"off"' || fail "JSON should show off, got: $OUTPUT"
pass "JSON output correct: $OUTPUT"

# Test 6: Idempotent off (already off)
info "Testing idempotent off..."
OUTPUT=$("$POWERMON" off 2>&1)
EXIT_CODE=$?
[[ $EXIT_CODE -eq 0 ]] || fail "Idempotent off failed with exit code $EXIT_CODE"
echo "$OUTPUT" | grep -qi "already off" || fail "Should indicate 'already off', got: $OUTPUT"
pass "Idempotent off works"

# Test 7: Turn display on
info "Testing 'on' command..."
"$POWERMON" on
EXIT_CODE=$?
[[ $EXIT_CODE -eq 0 ]] || fail "on command failed with exit code $EXIT_CODE"
pass "Display turned on (exit code 0)"

sleep 0.3

# Test 8: Status should now be On
info "Testing status after on..."
OUTPUT=$("$POWERMON" status 2>&1)
echo "$OUTPUT" | grep -q "On" || fail "Status after on should be 'On', got: $OUTPUT"
pass "Status is On after 'on' command"

# Test 9: JSON output when on
info "Testing JSON output when on..."
OUTPUT=$("$POWERMON" status --json 2>&1)
echo "$OUTPUT" | grep -q '"power":"on"' || fail "JSON should show on, got: $OUTPUT"
pass "JSON output correct: $OUTPUT"

# Test 10: Idempotent on (already on)
info "Testing idempotent on..."
OUTPUT=$("$POWERMON" on 2>&1)
EXIT_CODE=$?
[[ $EXIT_CODE -eq 0 ]] || fail "Idempotent on failed with exit code $EXIT_CODE"
echo "$OUTPUT" | grep -qi "already on" || fail "Should indicate 'already on', got: $OUTPUT"
pass "Idempotent on works"

echo ""
echo "=========================================="
echo -e "  ${GREEN}All tests passed!${NC}"
echo "=========================================="
