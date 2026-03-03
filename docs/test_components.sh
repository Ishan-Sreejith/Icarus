#!/bin/bash
# Test script to verify what actually works

echo "═══════════════════════════════════════════════════════════"
echo "CoRe Language - Component Test Suite"
echo "═══════════════════════════════════════════════════════════"
echo ""

# Test file
TEST_FILE="test_arithmetic.fr"

# Test 1: Interpreter
echo "1. Testing Interpreter (forger)..."
if ./target/release/forger "$TEST_FILE" > /tmp/forger_out.txt 2>&1; then
    if grep -q "12" /tmp/forger_out.txt && grep -q "35" /tmp/forger_out.txt; then
        echo "   ✅ PASS - Interpreter works correctly"
    else
        echo "   ❌ FAIL - Wrong output"
        cat /tmp/forger_out.txt
    fi
else
    echo "   ❌ FAIL - Crashed or errored"
fi
echo ""

# Test 2: Native Compiler
echo "2. Testing AOT Compiler (forge --native)..."
if ./target/release/forge --native "$TEST_FILE" > /tmp/aot_out.txt 2>&1; then
    if grep -q "12" /tmp/aot_out.txt && grep -q "35" /tmp/aot_out.txt; then
        echo "   ✅ PASS - AOT compiler works correctly"
    else
        echo "   ❌ FAIL - Wrong output"
        cat /tmp/aot_out.txt
    fi
else
    echo "   ❌ FAIL - Crashed or errored"
fi
echo ""

# Test 3: JIT Compiler
echo "3. Testing JIT Compiler (fforge)..."
if ./target/release/fforge "$TEST_FILE" > /tmp/jit_out.txt 2>&1; then
    if grep -q "12" /tmp/jit_out.txt && grep -q "35" /tmp/jit_out.txt; then
        echo "   ✅ PASS - JIT works correctly"
    else
        echo "   ⚠️  PARTIAL - Runs but wrong output"
        cat /tmp/jit_out.txt | head -10
    fi
else
    echo "   ❌ FAIL - Crashed (expected)"
    echo "   Exit code: $?"
fi
echo ""

# Test 4: VM
echo "4. Testing VM (core)..."
if ./target/release/core "$TEST_FILE" > /tmp/vm_out.txt 2>&1; then
    if grep -q "12" /tmp/vm_out.txt && grep -q "35" /tmp/vm_out.txt; then
        echo "   ✅ PASS - VM works correctly"
    else
        echo "   ❌ FAIL - Wrong output"
        cat /tmp/vm_out.txt | head -10
    fi
else
    echo "   ❌ FAIL - Crashed or errored"
    cat /tmp/vm_out.txt | head -5
fi
echo ""

# Test 5: Unit Tests
echo "5. Testing Unit Tests..."
if cargo test --lib --quiet 2>&1 | grep -q "test result: ok"; then
    echo "   ✅ PASS - All unit tests pass"
else
    echo "   ❌ FAIL - Tests crash or fail"
    echo "   (Run 'cargo test --lib' to see details)"
fi
echo ""

echo "═══════════════════════════════════════════════════════════"
echo "Summary:"
echo "  - Interpreter: Check above"
echo "  - AOT Compiler: Check above"
echo "  - JIT Compiler: Check above"
echo "  - VM: Check above"
echo "  - Unit Tests: Check above"
echo "═══════════════════════════════════════════════════════════"

