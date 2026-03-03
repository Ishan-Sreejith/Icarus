#!/bin/bash

# Test script for function fixes
set -e

echo "Building..."
cd "/Users/ishan/IdeaProjects/CoRe Main/CoRe Backup V1.0 copy"
cargo build 2>&1 | tail -5

echo ""
echo "=== Test 1: Simple function with return ==="
cat > /tmp/fn_simple.fr << 'EOF'
fn get_five {
    return 5
}

var x: get_five
say: x
EOF

./target/debug/fforge /tmp/fn_simple.fr 2>&1 | tail -5

echo ""
echo "=== Test 2: Function with parameters ==="
cat > /tmp/fn_params.fr << 'EOF'
fn add: a, b {
    return a + b
}

var result: add: 3, 4
say: result
EOF

./target/debug/fforge /tmp/fn_params.fr 2>&1 | tail -5

echo ""
echo "=== Test 3: Global arithmetic ==="
cat > /tmp/global.fr << 'EOF'
var a: 10
var b: 20
var c: a + b
say: c
EOF

./target/debug/fforge /tmp/global.fr 2>&1 | tail -5

echo "Tests complete!"

