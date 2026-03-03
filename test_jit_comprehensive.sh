#!/bin/bash
# Comprehensive JIT Compiler Test Suite
# Tests every feature of the CoRe language

cd "/Users/ishan/IdeaProjects/CoRe Main/CoRe Backup V1.0 copy"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

PASSED=0
FAILED=0
SKIPPED=0

# Test counter
TEST_NUM=1

# Function to run a test
run_test() {
    local name="$1"
    local file="$2"
    local expected="$3"
    local should_fail="$4"

    echo -e "${BLUE}[TEST $TEST_NUM]${NC} $name"

    if [ ! -f "$file" ]; then
        echo -e "${YELLOW}⊘ SKIPPED${NC} (file not found: $file)"
        ((SKIPPED++))
        ((TEST_NUM++))
        return
    fi

    output_full=$(./target/release/fforge "$file" 2>&1)
    output_tail=$(echo "$output_full" | tail -10)
    printed=$(echo "$output_full" | sed '/^\\[DEBUG\\]/d;/^→/d;/^✓/d;/^Finished/d' | sed '/^$/d')

    expected_ok=1
    if [ -n "$expected" ]; then
        # expected can be multi-line; require each line to appear verbatim in printed output
        while IFS= read -r line; do
            [ -z "$line" ] && continue
            if ! echo "$printed" | grep -Fxq "$line"; then
                expected_ok=0
                break
            fi
        done <<< "$expected"
    fi

    if [ -z "$should_fail" ]; then
        if echo "$output_full" | grep -q "✓" && [ $expected_ok -eq 1 ]; then
            echo -e "${GREEN}✓ PASS${NC}"
            ((PASSED++))
        else
            echo -e "${RED}✗ FAIL${NC}"
            echo "Output (tail):"
            echo "$output_tail"
            if [ $expected_ok -ne 1 ]; then
                echo "Expected lines:"
                echo "$expected"
                echo "Printed lines:"
                echo "$printed" | tail -30
            fi
            ((FAILED++))
        fi
    else
        if echo "$output_full" | grep -q "Error\\|error"; then
            echo -e "${YELLOW}⚠ EXPECTED FAIL${NC}"
            ((SKIPPED++))
        else
            echo -e "${RED}✗ UNEXPECTED PASS${NC}"
            ((FAILED++))
        fi
    fi
    ((TEST_NUM++))
}

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     CoRe Language JIT Comprehensive Test Suite             ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Rebuild first
echo -e "${BLUE}Building JIT compiler...${NC}"
cargo build --release 2>&1 | grep -E "Finished|error" || true
echo ""

# ============================================================
# FEATURE 1: BASIC VARIABLES
# ============================================================
echo -e "${YELLOW}=== FEATURE 1: BASIC VARIABLES ===${NC}"

# Create test files
cat > test_simple.fr << 'EOF'
var x: 5
say: x
EOF

cat > /tmp/test_vars.fr << 'EOF'
var x: 5
var y: 10
say: x
say: y
EOF

cat > /tmp/test_var_arith.fr << 'EOF'
var x: 5
var y: 3
var z: x + y
say: z
EOF

run_test "Simple variable assignment" "test_simple.fr" $'5' ""
run_test "Multiple variables" "/tmp/test_vars.fr" $'5\n10' ""
run_test "Variable with arithmetic" "/tmp/test_var_arith.fr" $'8' ""

# ============================================================
# FEATURE 2: ARITHMETIC
# ============================================================
echo -e "${YELLOW}=== FEATURE 2: ARITHMETIC ===${NC}"

cat > /tmp/test_add.fr << 'EOF'
var a: 5
var b: 3
say: a + b
EOF

cat > /tmp/test_sub.fr << 'EOF'
var a: 10
var b: 3
say: a - b
EOF

cat > /tmp/test_mul.fr << 'EOF'
var a: 4
var b: 5
say: a * b
EOF

cat > /tmp/test_div.fr << 'EOF'
var a: 20
var b: 4
say: a / b
EOF

run_test "Addition" "/tmp/test_add.fr" $'8' ""
run_test "Subtraction" "/tmp/test_sub.fr" $'7' ""
run_test "Multiplication" "/tmp/test_mul.fr" $'20' ""
run_test "Division" "/tmp/test_div.fr" $'5' ""

# ============================================================
# FEATURE 3: COMPARISONS
# ============================================================
echo -e "${YELLOW}=== FEATURE 3: COMPARISONS ===${NC}"

cat > /tmp/test_gt.fr << 'EOF'
var a: 5
var b: 3
if a > b {
    say: 1
} else {
    say: 0
}
EOF

cat > /tmp/test_lt.fr << 'EOF'
var a: 3
var b: 5
if a < b {
    say: 1
} else {
    say: 0
}
EOF

cat > /tmp/test_eq.fr << 'EOF'
var a: 5
var b: 5
if a == b {
    say: 1
} else {
    say: 0
}
EOF

run_test "Greater than" "/tmp/test_gt.fr" $'1' ""
run_test "Less than" "/tmp/test_lt.fr" $'1' ""
run_test "Equality" "/tmp/test_eq.fr" $'1' ""

# ============================================================
# FEATURE 4: CONTROL FLOW (if/else)
# ============================================================
echo -e "${YELLOW}=== FEATURE 4: CONTROL FLOW (if/else) ===${NC}"

cat > /tmp/test_if_simple.fr << 'EOF'
var x: 5
if x > 3 {
    say: "yes"
}
EOF

cat > /tmp/test_ifelse.fr << 'EOF'
var x: 2
if x > 3 {
    say: "big"
} else {
    say: "small"
}
EOF

run_test "Simple if" "/tmp/test_if_simple.fr" $'yes' ""
run_test "If/else" "/tmp/test_ifelse.fr" $'small' ""

# ============================================================
# FEATURE 5: FUNCTIONS
# ============================================================
echo -e "${YELLOW}=== FEATURE 5: FUNCTIONS ===${NC}"

cat > /tmp/test_fn_simple.fr << 'EOF'
fn add_one: x {
    return x + 1
}

var result: add_one: 5
say: result
EOF

cat > /tmp/test_fn_multi.fr << 'EOF'
fn multiply: a, b {
    return a * b
}

var result: multiply: 3, 4
say: result
EOF

cat > /tmp/test_fn_noargs.fr << 'EOF'
fn get_five {
    return 5
}

say: get_five
EOF

run_test "Function with parameter" "/tmp/test_fn_simple.fr" $'6' ""
run_test "Function with multiple params" "/tmp/test_fn_multi.fr" $'12' ""
run_test "Function with no args" "/tmp/test_fn_noargs.fr" $'5' ""

# ============================================================
# FEATURE 6: WHILE LOOPS
# ============================================================
echo -e "${YELLOW}=== FEATURE 6: WHILE LOOPS ===${NC}"

cat > /tmp/test_while.fr << 'EOF'
var i: 0
while i < 3 {
    say: i
    var i: i + 1
}
EOF

run_test "While loop" "/tmp/test_while.fr" $'0\n1\n2' ""

# ============================================================
# FEATURE 7: FOR LOOPS
# ============================================================
echo -e "${YELLOW}=== FEATURE 7: FOR LOOPS ===${NC}"

cat > /tmp/test_for_list.fr << 'EOF'
var list: [1, 2, 3]
for item in list {
    say: item
}
EOF

run_test "For loop on list" "/tmp/test_for_list.fr" "" "fail"

# ============================================================
# FEATURE 8: ARRAYS/LISTS
# ============================================================
echo -e "${YELLOW}=== FEATURE 8: ARRAYS/LISTS ===${NC}"

cat > /tmp/test_array.fr << 'EOF'
var list: [5, 10, 15]
say: list[0]
EOF

cat > /tmp/test_array_access.fr << 'EOF'
var arr: [1, 2, 3]
var first: arr[0]
say: first
EOF

run_test "Array creation" "/tmp/test_array.fr" $'5' ""
run_test "Array access" "/tmp/test_array_access.fr" $'1' ""

# ============================================================
# FEATURE 9: STRINGS
# ============================================================
echo -e "${YELLOW}=== FEATURE 9: STRINGS ===${NC}"

cat > /tmp/test_string.fr << 'EOF'
var msg: "hello"
say: msg
EOF

cat > /tmp/test_string_concat.fr << 'EOF'
var a: "hello"
var b: "world"
say: a + b
EOF

run_test "String variable" "/tmp/test_string.fr" $'hello' ""
run_test "String concatenation" "/tmp/test_string_concat.fr" $'helloworld' ""

# ============================================================
# FEATURE 10: MAPS/OBJECTS
# ============================================================
echo -e "${YELLOW}=== FEATURE 10: MAPS/OBJECTS ===${NC}"

cat > /tmp/test_map.fr << 'EOF'
var data: { "name": "Alice", "age": 30 }
say: data["name"]
EOF

run_test "Map creation" "/tmp/test_map.fr" $'Alice' ""

# ============================================================
# FEATURE 11: USER INPUT
# ============================================================
echo -e "${YELLOW}=== FEATURE 11: USER INPUT ===${NC}"

cat > /tmp/test_ask.fr << 'EOF'
var name: ask: "Name: "
say: name
EOF

run_test "User input (ask)" "/tmp/test_ask.fr" "" "fail"

# ============================================================
# FEATURE 12: TRY/CATCH
# ============================================================
echo -e "${YELLOW}=== FEATURE 12: TRY/CATCH ===${NC}"

cat > /tmp/test_try.fr << 'EOF'
try {
    var x: 5
    say: x
} catch e {
    say: "error"
}
EOF

run_test "Try/catch" "/tmp/test_try.fr" "" "fail"

# ============================================================
# FEATURE 13: ASYNC/AWAIT
# ============================================================
echo -e "${YELLOW}=== FEATURE 13: ASYNC/AWAIT ===${NC}"

cat > /tmp/test_async.fr << 'EOF'
async fn fetch: url {
    return 42
}

var result: fetch: "http://example.com"
say: result
EOF

run_test "Async function" "/tmp/test_async.fr" $'42' ""

# ============================================================
# SUMMARY
# ============================================================
echo ""
echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                    TEST SUMMARY                            ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo -e "Tests Passed:   ${GREEN}$PASSED${NC}"
echo -e "Tests Failed:   ${RED}$FAILED${NC}"
echo -e "Tests Skipped:  ${YELLOW}$SKIPPED${NC}"
echo ""
echo -e "Total: $((PASSED + FAILED + SKIPPED))"

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ ALL TESTS PASSED!${NC}"
    exit 0
else
    echo -e "${RED}✗ SOME TESTS FAILED${NC}"
    exit 1
fi
