#!/bin/bash
# Comprehensive Vortex Testing Script

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

VORTEX_BIN="./target/release/vortex"
TESTS_PASSED=0
TESTS_FAILED=0

echo -e "${BLUE}╔════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     Vortex Container Runtime Tests        ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════╝${NC}\n"

# Helper functions
test_start() {
    echo -e "${YELLOW}▶ Test: $1${NC}"
}

test_pass() {
    echo -e "${GREEN}✅ PASS: $1${NC}\n"
    ((TESTS_PASSED++))
}

test_fail() {
    echo -e "${RED}❌ FAIL: $1${NC}\n"
    ((TESTS_FAILED++))
}

cleanup_container() {
    sudo $VORTEX_BIN stop --id "$1" 2>/dev/null || true
}

# Check if running as non-root
if [ "$EUID" -eq 0 ]; then
    echo -e "${RED}Error: Don't run this script as root${NC}"
    echo "The script will use sudo when needed"
    exit 1
fi

# Test 1: Build
echo -e "${BLUE}═══ Build Tests ═══${NC}\n"
test_start "Building release binary"
if cargo build --release --bin vortex 2>&1 | tail -5; then
    test_pass "Build successful"
else
    test_fail "Build failed"
    exit 1
fi

# Test 2: Help command (no root)
echo -e "${BLUE}═══ Help Tests (No Root Required) ═══${NC}\n"
test_start "Show help"
if $VORTEX_BIN --help > /dev/null 2>&1; then
    test_pass "Help command works"
else
    test_fail "Help command failed"
fi

# Test 3: Namespace info (no root)
test_start "Show namespace info"
if $VORTEX_BIN namespaces > /dev/null 2>&1; then
    test_pass "Namespace info works"
else
    test_fail "Namespace info failed"
fi

# Test 4-10: Container tests (require root)
echo -e "${BLUE}═══ Container Tests (Require Root) ═══${NC}\n"

# Test 4: Basic echo
test_start "Run basic echo command"
OUTPUT=$(sudo $VORTEX_BIN run --id test-echo -- /bin/echo "Hello Vortex" 2>&1)
if echo "$OUTPUT" | grep -q "Hello Vortex"; then
    test_pass "Basic echo works"
else
    test_fail "Basic echo failed"
fi
cleanup_container "test-echo"

# Test 5: Custom hostname
test_start "Run with custom hostname"
OUTPUT=$(sudo $VORTEX_BIN run --id test-hostname --hostname vortex-test -- /bin/hostname 2>&1)
if echo "$OUTPUT" | grep -q "vortex-test"; then
    test_pass "Custom hostname works"
else
    test_fail "Custom hostname failed"
fi
cleanup_container "test-hostname"

# Test 6: CPU limits
test_start "Run with CPU limits"
if sudo $VORTEX_BIN run --id test-cpu --cpu 0.5 -- /bin/echo "CPU limited" > /dev/null 2>&1; then
    test_pass "CPU limits work"
else
    test_fail "CPU limits failed"
fi
cleanup_container "test-cpu"

# Test 7: Memory limits
test_start "Run with memory limits"
if sudo $VORTEX_BIN run --id test-mem --memory 256 -- /bin/echo "Memory limited" > /dev/null 2>&1; then
    test_pass "Memory limits work"
else
    test_fail "Memory limits failed"
fi
cleanup_container "test-mem"

# Test 8: Shell commands
test_start "Run shell command with pipes"
OUTPUT=$(sudo $VORTEX_BIN run --id test-shell -- /bin/sh -c 'echo "PID: $$"' 2>&1)
if echo "$OUTPUT" | grep -q "PID:"; then
    test_pass "Shell commands work"
else
    test_fail "Shell commands failed"
fi
cleanup_container "test-shell"

# Test 9: Multiple sequential containers
test_start "Run multiple sequential containers"
SUCCESS=true
for i in {1..3}; do
    if ! sudo $VORTEX_BIN run --id "test-seq-$i" -- /bin/echo "Container $i" > /dev/null 2>&1; then
        SUCCESS=false
    fi
    cleanup_container "test-seq-$i"
done
if $SUCCESS; then
    test_pass "Sequential containers work"
else
    test_fail "Sequential containers failed"
fi

# Test 10: Without namespaces
test_start "Run without namespaces"
if sudo $VORTEX_BIN run --id test-no-ns --no-namespaces -- /bin/echo "No isolation" > /dev/null 2>&1; then
    test_pass "No namespace mode works"
else
    test_fail "No namespace mode failed"
fi
cleanup_container "test-no-ns"

# Test 11: Stats command (container doesn't exist)
test_start "Stats for non-existent container"
if sudo $VORTEX_BIN stats --id nonexistent 2>&1 | grep -q "Failed to create controller"; then
    test_pass "Stats error handling works"
else
    test_fail "Stats error handling failed"
fi

# Test 12: List empty containers
test_start "List containers (should be empty)"
OUTPUT=$(sudo $VORTEX_BIN list 2>&1)
if echo "$OUTPUT" | grep -q "No containers running"; then
    test_pass "List empty works"
else
    test_fail "List empty failed"
fi

# Test 13: Long-running container with monitoring
echo -e "${BLUE}═══ Monitoring Tests ═══${NC}\n"
test_start "Run container with monitoring"
if timeout 5s sudo $VORTEX_BIN run --id test-monitor --monitor -- /bin/sleep 3 > /dev/null 2>&1; then
    test_pass "Monitoring works"
else
    test_fail "Monitoring failed"
fi
cleanup_container "test-monitor"

# Test 14: Stress test (create and destroy quickly)
echo -e "${BLUE}═══ Stress Tests ═══${NC}\n"
test_start "Rapid create/destroy cycles"
SUCCESS=true
for i in {1..10}; do
    if ! sudo $VORTEX_BIN run --id "stress-$i" -- /bin/true > /dev/null 2>&1; then
        SUCCESS=false
        break
    fi
    cleanup_container "stress-$i"
done
if $SUCCESS; then
    test_pass "Stress test works"
else
    test_fail "Stress test failed"
fi

# Summary
echo -e "${BLUE}═══════════════════════════════════════════${NC}"
echo -e "${BLUE}           Test Summary                    ${NC}"
echo -e "${BLUE}═══════════════════════════════════════════${NC}"
echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
echo -e "${RED}Failed: $TESTS_FAILED${NC}"
TOTAL=$((TESTS_PASSED + TESTS_FAILED))
echo -e "Total:  $TOTAL"
echo -e "${BLUE}═══════════════════════════════════════════${NC}\n"

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}🎉 All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}⚠️  Some tests failed${NC}"
    exit 1
fi