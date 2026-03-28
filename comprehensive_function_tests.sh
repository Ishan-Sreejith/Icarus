#!/bin/bash
set -u -o pipefail

PROJECT_DIR="/Users/ishan/IdeaProjects/CoRe Main/CoRe Backup V1.0 copy"
cd "$PROJECT_DIR"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

PASSED=0
FAILED=0

echo -e "${BLUE}Building release...${NC}"
cargo build --release 2>&1 | grep -E "Finished|error" || true

BIN="./target/release/fforge"
if [ ! -x "$BIN" ]; then
  echo -e "${RED}Release binary not found: $BIN${NC}"
  exit 1
fi

run_test() {
  local name="$1"
  local file="$2"
  local expect="$3"

  echo -e "\n${BLUE}Testing: $name${NC}"
  if [ ! -f "$file" ]; then
    echo -e "${RED}✗ FAIL${NC} (missing file: $file)"
    ((FAILED++))
    return 0
  fi

  set +e
  output=$("$BIN" "$file" 2>&1)
  status=$?
  set -e

  if [ $status -ne 0 ]; then
    echo -e "${RED}✗ FAIL${NC} (exit $status)"
    echo "$output" | tail -20
    ((FAILED++))
    return 0
  fi

  if [ -n "$expect" ] && ! echo "$output" | grep -Fq "$expect"; then
    echo -e "${RED}✗ FAIL${NC} (missing expected output)"
    echo "Expected: $expect"
    echo "Output tail:"
    echo "$output" | tail -20
    ((FAILED++))
    return 0
  fi

  echo -e "${GREEN}✓ PASS${NC}"
  ((PASSED++))
  return 0
}

echo -e "\n${BLUE}════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}ICARUS FEATURE TEST SUITE${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"

run_test "Feature showcase" "feature_showcase.fr" "Feature Test Complete"
run_test "Full feature demo" "examples/full_features.fr" "=== Done ==="
run_test "Examples: test_all" "examples/test_all.fr" "All tests done."
run_test "VM test_all" "vm/test_all.fr" "All tests done."
run_test "Main sample" "main.fr" ""
run_test "JIT advanced" "examples/test_jit_advanced.fr" ""
run_test "JIT loops" "examples/test_jit_loop.fr" ""
run_test "JIT arithmetic" "examples/test_jit_arithmetic.fr" ""

cat > /tmp/test_conversions.fr << 'EOF'
say: "Conversions start"
var n: 123
var s: str: n
say: s
var ns: "456"
var nn: num: ns
say: nn + 1
say: "Conversions done"
EOF

run_test "Type conversions" "/tmp/test_conversions.fr" "Conversions done"

echo -e "\n${BLUE}════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}TEST SUMMARY${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo -e "Total:  $((PASSED + FAILED))"

if [ $FAILED -eq 0 ]; then
  echo -e "\n${GREEN}✓ All feature tests passed${NC}"
  exit 0
else
  echo -e "\n${RED}✗ Some feature tests failed${NC}"
  exit 1
fi
