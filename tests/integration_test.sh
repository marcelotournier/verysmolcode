#!/bin/bash
# Integration test for VerySmolCode
# Runs the TUI in tmux and sends a command to build a todo list app
#
# Prerequisites:
# - GEMINI_API_KEY set
# - tmux installed
# - cargo built binary available
#
# Usage: ./tests/integration_test.sh

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo -e "${YELLOW}VerySmolCode Integration Test${NC}"
echo "=============================="

# Check prerequisites
if [ -z "$GEMINI_API_KEY" ]; then
    echo -e "${RED}FAIL: GEMINI_API_KEY not set${NC}"
    exit 1
fi

if ! command -v tmux &> /dev/null; then
    echo -e "${RED}FAIL: tmux not installed${NC}"
    exit 1
fi

# Build the binary
echo "Building vsc..."
source "$HOME/.cargo/env" 2>/dev/null || true
cargo build --release 2>&1 | tail -1
VSC="$(pwd)/target/release/vsc"

if [ ! -f "$VSC" ]; then
    echo -e "${RED}FAIL: Binary not found at $VSC${NC}"
    exit 1
fi

# Create a temp directory for the test project
TEST_DIR=$(mktemp -d /tmp/vsc_integration_XXXXXX)
echo "Test directory: $TEST_DIR"

# Create tmux session
SESSION="vsc_test_$$"
tmux new-session -d -s "$SESSION" -x 120 -y 40

# Set working directory and start vsc
tmux send-keys -t "$SESSION" "cd $TEST_DIR && GEMINI_API_KEY=$GEMINI_API_KEY $VSC" Enter

# Wait for TUI to initialize
sleep 5

# Capture initial screen
echo "TUI initialized, screen capture:"
tmux capture-pane -t "$SESSION" -p 2>/dev/null | head -5

# Send the test command using tmux buffer (more reliable than send-keys for long text)
echo "Sending test command..."
# First verify the TUI is responsive by sending a simple keystroke
tmux send-keys -t "$SESSION" -l "Create app.py and requirements.txt for a bottle.py todo app with add list done delete routes."
sleep 1
echo "Text entered, capturing screen..."
tmux capture-pane -t "$SESSION" -p 2>/dev/null | tail -5
echo "Pressing Enter..."
tmux send-keys -t "$SESSION" Enter

# Wait for the agent to work (generous timeout for free tier)
echo "Waiting for agent to complete (up to 180 seconds)..."
TIMEOUT=180
ELAPSED=0
while [ $ELAPSED -lt $TIMEOUT ]; do
    sleep 10
    ELAPSED=$((ELAPSED + 10))
    echo "  ... ${ELAPSED}s elapsed"

    # Capture tmux screen for debugging
    if [ $((ELAPSED % 30)) -eq 0 ]; then
        echo "--- tmux screen capture ---"
        tmux capture-pane -t "$SESSION" -p 2>/dev/null | tail -15
        echo "--- end capture ---"
    fi

    # Check if files were created
    if [ -f "$TEST_DIR/app.py" ] && [ -f "$TEST_DIR/requirements.txt" ]; then
        echo -e "${GREEN}Files detected!${NC}"
        # Give agent a moment to finish writing
        sleep 3
        break
    fi

    # Show files at timeout
    if [ $ELAPSED -eq $TIMEOUT ]; then
        echo "Files in test dir:"
        ls -la "$TEST_DIR/" 2>/dev/null || echo "(empty)"
    fi
done

# Quit vsc
tmux send-keys -t "$SESSION" '/quit' Enter
sleep 1

# Kill tmux session
tmux kill-session -t "$SESSION" 2>/dev/null || true

# ===== Verification =====
echo ""
echo "Verification:"
echo "============="

PASS=0
FAIL=0

# Check 1: app.py exists
if [ -f "$TEST_DIR/app.py" ]; then
    echo -e "${GREEN}[PASS] app.py created${NC}"
    PASS=$((PASS + 1))
else
    echo -e "${RED}[FAIL] app.py not found${NC}"
    FAIL=$((FAIL + 1))
fi

# Check 2: requirements.txt exists
if [ -f "$TEST_DIR/requirements.txt" ]; then
    echo -e "${GREEN}[PASS] requirements.txt created${NC}"
    PASS=$((PASS + 1))
else
    echo -e "${RED}[FAIL] requirements.txt not found${NC}"
    FAIL=$((FAIL + 1))
fi

# Check 3: app.py contains bottle import
if [ -f "$TEST_DIR/app.py" ] && grep -q "bottle" "$TEST_DIR/app.py"; then
    echo -e "${GREEN}[PASS] app.py imports bottle${NC}"
    PASS=$((PASS + 1))
else
    echo -e "${RED}[FAIL] app.py doesn't import bottle${NC}"
    FAIL=$((FAIL + 1))
fi

# Check 4: requirements.txt contains bottle
if [ -f "$TEST_DIR/requirements.txt" ] && grep -qi "bottle" "$TEST_DIR/requirements.txt"; then
    echo -e "${GREEN}[PASS] requirements.txt contains bottle${NC}"
    PASS=$((PASS + 1))
else
    echo -e "${RED}[FAIL] requirements.txt doesn't contain bottle${NC}"
    FAIL=$((FAIL + 1))
fi

# Check 5: app.py has route definitions
if [ -f "$TEST_DIR/app.py" ] && grep -q "route\|@get\|@post" "$TEST_DIR/app.py"; then
    echo -e "${GREEN}[PASS] app.py has route definitions${NC}"
    PASS=$((PASS + 1))
else
    echo -e "${RED}[FAIL] app.py has no route definitions${NC}"
    FAIL=$((FAIL + 1))
fi

# Check 6: Python syntax is valid
if [ -f "$TEST_DIR/app.py" ]; then
    if python3 -c "import ast; ast.parse(open('$TEST_DIR/app.py').read())" 2>/dev/null; then
        echo -e "${GREEN}[PASS] app.py has valid Python syntax${NC}"
        PASS=$((PASS + 1))
    else
        echo -e "${RED}[FAIL] app.py has syntax errors${NC}"
        FAIL=$((FAIL + 1))
    fi
else
    echo -e "${RED}[FAIL] app.py missing, can't check syntax${NC}"
    FAIL=$((FAIL + 1))
fi

# ===== Pytest checks (if basic checks passed) =====
if [ $PASS -ge 5 ] && command -v python3 &> /dev/null; then
    echo ""
    echo "Running pytest validation..."
    pip install bottle pytest -q 2>/dev/null || true
    export VSC_TEST_DIR="$TEST_DIR"
    python3 -m pytest "$SCRIPT_DIR/test_vibecoded_app.py" -v --tb=short 2>&1 || true
fi

# Cleanup
rm -rf "$TEST_DIR"

echo ""
echo "Results: $PASS passed, $FAIL failed out of $((PASS + FAIL)) checks"

if [ $FAIL -eq 0 ]; then
    echo -e "${GREEN}Integration test PASSED!${NC}"
    exit 0
else
    echo -e "${RED}Integration test FAILED!${NC}"
    exit 1
fi
