#!/usr/bin/env bash
# Test script to diagnose JIT compiler hanging

set -e

cd "/Users/ishan/IdeaProjects/CoRe Main/CoRe Backup V1.0 copy"

echo "[1/5] Checking if binaries exist..."
ls -la target/debug/forge target/debug/forger target/debug/fforge 2>&1 | tail -5

echo ""
echo "[2/5] Running cargo test to verify compilation..."
cargo test --lib jit::trampoline::tests::test_trampoline_returns_value --release 2>&1 | tail -10

echo ""
echo "[3/5] Attempting to create minimal test file..."
cat > /tmp/minimal_test.fr << 'EOF'
say: 5
EOF

echo ""
echo "[4/5] Attempting forger (Rust interpreter)..."
timeout 5 ./target/debug/forger /tmp/minimal_test.fr || echo "forger timed out or failed"

echo ""
echo "[5/5] Attempting fforge (JIT)..."
timeout 5 ./target/debug/fforge /tmp/minimal_test.fr 2>&1 || echo "fforge timed out or failed"

echo ""
echo "Diagnostic complete."

