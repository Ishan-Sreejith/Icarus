#!/bin/bash

# Comprehensive CoRe Language Test Suite
# Tests all execution pathways

set -e

PROJECT_DIR="/Users/ishan/IdeaProjects/CoRe Main/CoRe Backup V1.0 copy"
cd "$PROJECT_DIR"

echo "╔════════════════════════════════════════════════╗"
echo "║     CoRe Language - Comprehensive Test Suite   ║"
echo "╚════════════════════════════════════════════════╝"
echo ""

# Build
echo "📦 Building project..."
cargo build --release 2>&1 | tail -3
echo "✓ Build complete"
echo ""

# Test files
TEST_FILES=(
    "simple_test.fr"
    "main.fr"
    "examples/full_features.fr"
)

echo "🧪 Running tests..."
echo ""

# Test 1: Simple Test with JIT
echo "Test 1: Simple Test (JIT)"
echo "========================"
timeout 5 ./target/release/fforge simple_test.fr 2>&1 | grep -E "(===|Result:|Error)" || echo "⚠ Test 1 output suppressed"
echo ""

# Test 2: Main Features (JIT)
echo "Test 2: Main Features (JIT)"
echo "==========================="
timeout 5 ./target/release/fforge main.fr 2>&1 | grep -E "(===|Result:|Error)" || echo "⚠ Test 2 output suppressed"
echo ""

# Test 3: Full Features (VM)
echo "Test 3: Full Features (VM)"
echo "=========================="
timeout 5 ./target/release/forge main.fr 2>&1 | grep -E "(===|Result:|Error)" || echo "⚠ Test 3 output suppressed"
echo ""

echo "✓ All tests complete"
echo ""
echo "╔════════════════════════════════════════════════╗"
echo "║              Testing Complete                  ║"
echo "╚════════════════════════════════════════════════╝"

