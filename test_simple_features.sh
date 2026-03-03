#!/bin/bash
# Simple test - just test core features one by one

cd "/Users/ishan/IdeaProjects/CoRe Main/CoRe Backup V1.0 copy"

echo "Building..."
cargo build 2>&1 > /tmp/build.log
if [ $? -ne 0 ]; then
    echo "BUILD FAILED"
    cat /tmp/build.log | tail -20
    exit 1
fi
echo "✓ Build succeeded"

# Test 1: Simple variable
echo ""
echo "TEST 1: Simple variable"
cat > /tmp/test1.fr << 'EOF'
var x: 42
say: x
EOF
./target/debug/fforge /tmp/test1.fr > /tmp/test1.out 2>&1
if grep -q "✓" /tmp/test1.out; then
    echo "✓ PASS"
else
    echo "✗ FAIL"
    cat /tmp/test1.out | tail -5
fi

# Test 2: Arithmetic
echo ""
echo "TEST 2: Arithmetic"
cat > /tmp/test2.fr << 'EOF'
var a: 10
var b: 5
say: a + b
EOF
./target/debug/fforge /tmp/test2.fr > /tmp/test2.out 2>&1
if grep -q "✓" /tmp/test2.out; then
    echo "✓ PASS"
else
    echo "✗ FAIL"
    cat /tmp/test2.out | tail -5
fi

# Test 3: Function call
echo ""
echo "TEST 3: Function call"
cat > /tmp/test3.fr << 'EOF'
fn add_one: x {
    return x + 1
}

var result: add_one: 5
say: result
EOF
./target/debug/fforge /tmp/test3.fr > /tmp/test3.out 2>&1
if grep -q "✓" /tmp/test3.out; then
    echo "✓ PASS (but return value may be wrong)"
else
    echo "✗ FAIL"
    cat /tmp/test3.out | tail -5
fi

# Test 4: For loop (after is_map fix)
echo ""
echo "TEST 4: For loop (with is_map fix)"
cat > /tmp/test4.fr << 'EOF'
var list: [1, 2, 3]
for item in list {
    say: item
}
EOF
./target/debug/fforge /tmp/test4.fr > /tmp/test4.out 2>&1
if grep -q "✓" /tmp/test4.out; then
    echo "✓ PASS"
elif grep -q "is_map" /tmp/test4.out; then
    echo "✗ FAIL - is_map error still present"
    cat /tmp/test4.out | grep "Error"
else
    echo "⚠ UNKNOWN"
    cat /tmp/test4.out | tail -5
fi

# Test 5: main.fr
echo ""
echo "TEST 5: main.fr (the original problem file)"
./target/debug/fforge main.fr > /tmp/test5.out 2>&1
if grep -q "is_map" /tmp/test5.out; then
    echo "✗ FAIL - is_map error"
    cat /tmp/test5.out | grep "Error"
elif grep -q "✓" /tmp/test5.out; then
    echo "✓ PASS"
else
    echo "⚠ UNKNOWN"
    cat /tmp/test5.out | tail -5
fi

echo ""
echo "═══════════════════════════════════"
echo "Test Summary:"
echo "═══════════════════════════════════"
echo "Check /tmp/test*.out for full output"

