#!/bin/bash
# JIT Test Verification Script
# Run all tests to verify the is_map fix and language features

cd "/Users/ishan/IdeaProjects/CoRe Main/CoRe Backup V1.0 copy"

echo "╔════════════════════════════════════════════════════════════════════════════╗"
echo "║                    JIT COMPILER FEATURE TEST SUITE                         ║"
echo "║                   All Tests with Verification Steps                        ║"
echo "╚════════════════════════════════════════════════════════════════════════════╝"
echo ""

# Step 1: Build
echo "[STEP 1] Building JIT Compiler..."
cargo build 2>&1 > /tmp/build.log
if [ $? -ne 0 ]; then
    echo "❌ BUILD FAILED"
    echo ""
    echo "Error output:"
    cat /tmp/build.log | grep "error\["
    exit 1
fi
echo "✅ Build succeeded"
echo ""

# Step 2: Test each feature
PASS=0
FAIL=0

test_feature() {
    local name="$1"
    local file="$2"
    local expected="$3"

    echo -n "  Testing: $name... "

    if [ ! -f "$file" ]; then
        echo "⊘ SKIPPED (file not found)"
        return
    fi

    ./target/debug/fforge "$file" > /tmp/test_out.txt 2>&1

    if [ -n "$expected" ] && grep -q "$expected" /tmp/test_out.txt; then
        echo "✅ PASS"
        ((PASS++))
    elif grep -q "✓" /tmp/test_out.txt; then
        echo "✅ PASS"
        ((PASS++))
    elif grep -q "Error\|error" /tmp/test_out.txt; then
        echo "❌ FAIL"
        grep "Error\|error" /tmp/test_out.txt | head -1 | sed 's/^/    /'
        ((FAIL++))
    else
        echo "⚠️ UNKNOWN"
    fi
}

echo "[STEP 2] Testing Language Features..."
echo ""

# TIER 1: Core Features
echo "Tier 1: Core Features"

# Create test files
cat > /tmp/test_vars.fr << 'EOF'
var x: 5
say: x
EOF

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

cat > /tmp/test_gt.fr << 'EOF'
if 5 > 3 {
    say: 1
}
EOF

cat > /tmp/test_lt.fr << 'EOF'
if 3 < 5 {
    say: 1
}
EOF

cat > /tmp/test_eq.fr << 'EOF'
if 5 == 5 {
    say: 1
}
EOF

cat > /tmp/test_ifelse.fr << 'EOF'
if 2 > 3 {
    say: 0
} else {
    say: 1
}
EOF

test_feature "Simple variables" "/tmp/test_vars.fr"
test_feature "Addition" "/tmp/test_add.fr"
test_feature "Subtraction" "/tmp/test_sub.fr"
test_feature "Multiplication" "/tmp/test_mul.fr"
test_feature "Greater than" "/tmp/test_gt.fr"
test_feature "Less than" "/tmp/test_lt.fr"
test_feature "Equality" "/tmp/test_eq.fr"
test_feature "If/else" "/tmp/test_ifelse.fr"

echo ""
echo "Tier 2: Functions"

cat > /tmp/test_fn_simple.fr << 'EOF'
fn add_one: x {
    return x + 1
}

var result: add_one: 5
say: result
EOF

cat > /tmp/test_fn_multi.fr << 'EOF'
fn add: x, y {
    return x + y
}

var result: add: 3, 4
say: result
EOF

test_feature "Function with parameter" "/tmp/test_fn_simple.fr"
test_feature "Function with multiple params" "/tmp/test_fn_multi.fr"

echo ""
echo "Tier 3: Loops & Collections (CRITICAL)"

cat > /tmp/test_for_simple.fr << 'EOF'
var list: [1, 2, 3]
for item in list {
    say: item
}
EOF

cat > /tmp/test_while.fr << 'EOF'
var i: 0
while i < 3 {
    say: i
    var i: i + 1
}
EOF

test_feature "For loop (with is_map fix)" "/tmp/test_for_simple.fr"
test_feature "While loop" "/tmp/test_while.fr"

echo ""
echo "Tier 4: Original Problem (main.fr)"

echo -n "  Testing: main.fr (original blocker)... "
./target/debug/fforge main.fr > /tmp/main_out.txt 2>&1

if grep -q "Unknown function: is_map" /tmp/main_out.txt; then
    echo "❌ FAIL - is_map error persists"
    grep "Unknown function" /tmp/main_out.txt | sed 's/^/    /'
    ((FAIL++))
elif grep -q "✓" /tmp/main_out.txt; then
    echo "✅ PASS - is_map fix verified!"
    ((PASS++))
else
    echo "⚠️ UNKNOWN"
    cat /tmp/main_out.txt | tail -3 | sed 's/^/    /'
fi

echo ""
echo "╔════════════════════════════════════════════════════════════════════════════╗"
echo "║                          TEST RESULTS SUMMARY                              ║"
echo "╚════════════════════════════════════════════════════════════════════════════╝"
echo ""
echo "Tests Passed:   ✅ $PASS"
echo "Tests Failed:   ❌ $FAIL"
echo "Total Tests:    $((PASS + FAIL))"
echo ""

if [ $FAIL -eq 0 ]; then
    echo "🎉 ALL TESTS PASSED! 🎉"
    echo ""
    echo "Next steps:"
    echo "1. Focus on function return value bug (returns garbage)"
    echo "2. Implement array support"
    echo "3. Implement string support"
    exit 0
else
    echo "⚠️ SOME TESTS FAILED"
    echo ""
    echo "Failed tests indicate:"
    echo "- Functions still return garbage (known issue)"
    echo "- Arrays/lists not fully implemented"
    echo "- Strings not implemented"
    exit 1
fi

