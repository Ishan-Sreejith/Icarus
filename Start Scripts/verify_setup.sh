#!/bin/bash

echo "╔════════════════════════════════════════════════╗"
echo "║      CoRe Language - Status Verification       ║"
echo "╚════════════════════════════════════════════════╝"
echo ""

PROJECT_DIR="/Users/ishan/IdeaProjects/CoRe Main/CoRe Backup V1.0 copy"
cd "$PROJECT_DIR"

echo "📂 Project Files:"
echo "  ✓ main.fr ($(wc -l < main.fr) lines)"
echo "  ✓ simple_test.fr ($(wc -l < simple_test.fr) lines)"
echo "  ✓ examples/full_features.fr ($(wc -l < examples/full_features.fr) lines)"
echo ""

echo "📦 Build Status:"
if [ -f "target/release/fforge" ]; then
    echo "  ✓ fforge (JIT) - $(ls -lh target/release/fforge | awk '{print $5}')"
else
    echo "  ✗ fforge not built"
fi

if [ -f "target/release/forge" ]; then
    echo "  ✓ forge (VM) - $(ls -lh target/release/forge | awk '{print $5}')"
else
    echo "  ✗ forge not built"
fi

if [ -f "target/release/forger" ]; then
    echo "  ✓ forger (Interpreter) - $(ls -lh target/release/forger | awk '{print $5}')"
else
    echo "  ✗ forger not built"
fi
echo ""

echo "📚 Documentation:"
if [ -d "docs" ]; then
    DOC_COUNT=$(find docs -type f | wc -l)
    echo "  ✓ docs/ folder ($DOC_COUNT files)"
else
    echo "  ✗ docs/ folder not found"
fi
echo ""

echo "💡 Examples:"
if [ -d "examples" ]; then
    EX_COUNT=$(find examples -type f | wc -l)
    echo "  ✓ examples/ folder ($EX_COUNT files)"
else
    echo "  ✗ examples/ folder not found"
fi
echo ""

echo "🧪 Test Files:"
if [ -f "test_all_features.sh" ]; then
    echo "  ✓ test_all_features.sh"
else
    echo "  ✗ test_all_features.sh not found"
fi

if [ -f "test_core_features.sh" ]; then
    echo "  ✓ test_core_features.sh"
else
    echo "  ✗ test_core_features.sh not found"
fi
echo ""

echo "🔍 Code Statistics:"
RUST_LINES=$(find src -name "*.rs" -type f -exec wc -l {} + | tail -1 | awk '{print $1}')
echo "  ✓ Rust source: $RUST_LINES lines"
FR_LINES=$(find . -maxdepth 1 -name "*.fr" -type f -exec wc -l {} + | tail -1 | awk '{print $1}')
echo "  ✓ CoRe programs: $FR_LINES lines"
echo ""

echo "╔════════════════════════════════════════════════╗"
echo "║        Verification Complete ✅                ║"
echo "╚════════════════════════════════════════════════╝"

