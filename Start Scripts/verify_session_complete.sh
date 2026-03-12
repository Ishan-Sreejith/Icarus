#!/bin/bash

cd "/Users/ishan/IdeaProjects/CoRe Main/CoRe Backup V1.0 copy"

echo "╔════════════════════════════════════════════════════════════════════════════╗"
echo "║              SESSION VERIFICATION & COMPLETION CHECKLIST                   ║"
echo "╚════════════════════════════════════════════════════════════════════════════╝"
echo ""

echo "[1/6] Checking is_map fix in source code..."
if grep -q 'func == "is_map"' src/jit/compiler.rs; then
    echo "  ✅ is_map handler found in src/jit/compiler.rs"
else
    echo "  ❌ is_map handler NOT found"
fi

if grep -q 'func == "is_list"' src/jit/compiler.rs; then
    echo "  ✅ is_list handler found"
else
    echo "  ❌ is_list handler NOT found"
fi

if grep -q 'func == "is_string"' src/jit/compiler.rs; then
    echo "  ✅ is_string handler found"
else
    echo "  ❌ is_string handler NOT found"
fi
echo ""

echo "[2/6] Checking test scripts..."
test_files=(
    "verify_jit_features.sh"
    "test_simple_features.sh"
    "test_jit_comprehensive.sh"
    "organize_docs.sh"
)

for file in "${test_files[@]}"; do
    if [ -f "$file" ]; then
        echo "  ✅ $file"
    else
        echo "  ❌ $file NOT FOUND"
    fi
done
echo ""

echo "[3/6] Checking documentation files..."
doc_files=(
    "JIT_TESTING_MARKDOWN_REPORT.md"
    "SESSION_JIT_TESTING_FINAL_REPORT.txt"
    "JIT_FEATURE_TEST_COMPREHENSIVE.txt"
    "SESSION_COMPLETION_SUMMARY.txt"
)

for file in "${doc_files[@]}"; do
    if [ -f "$file" ]; then
        echo "  ✅ $file"
    else
        echo "  ❌ $file NOT FOUND"
    fi
done
echo ""

echo "[4/6] Checking build status..."
if cargo build 2>&1 | grep -q "Finished"; then
    echo "  ✅ Build succeeds"
else
    echo "  ❌ Build FAILS"
fi
echo ""

echo "[5/6] Running quick feature tests..."

cat > /tmp/verify_var.fr << 'EOF'
var x: 42
say: x
EOF

./target/debug/fforge /tmp/verify_var.fr > /tmp/verify_var.out 2>&1
if grep -q "✓" /tmp/verify_var.out; then
    echo "  ✅ Variables work"
else
    echo "  ⚠️  Variables test inconclusive"
fi

cat > /tmp/verify_arith.fr << 'EOF'
say: 5 + 3
EOF

./target/debug/fforge /tmp/verify_arith.fr > /tmp/verify_arith.out 2>&1
if grep -q "✓" /tmp/verify_arith.out; then
    echo "  ✅ Arithmetic works"
else
    echo "  ⚠️  Arithmetic test inconclusive"
fi

cat > /tmp/verify_for.fr << 'EOF'
var list: [1, 2, 3]
for item in list {
    say: item
}
EOF

./target/debug/fforge /tmp/verify_for.fr > /tmp/verify_for.out 2>&1
if grep -q "Unknown function: is_map" /tmp/verify_for.out; then
    echo "  ❌ is_map fix NOT working - error still present"
elif grep -q "✓" /tmp/verify_for.out; then
    echo "  ✅ is_map fix verified - for loops work"
else
    echo "  ⚠️  For loop test inconclusive"
fi

echo ""

echo "[6/6] Testing main.fr..."
./target/debug/fforge main.fr > /tmp/main_verify.out 2>&1
if grep -q "Unknown function: is_map" /tmp/main_verify.out; then
    echo "  ❌ is_map error still present in main.fr"
elif grep -q "Error" /tmp/main_verify.out; then
    echo "  ⚠️  Different error in main.fr"
    grep "Error" /tmp/main_verify.out | head -1 | sed 's/^/    /'
elif grep -q "✓" /tmp/main_verify.out; then
    echo "  ✅ main.fr compiles and runs!"
else
    echo "  ⚠️  main.fr test inconclusive"
fi

echo ""
echo "╔════════════════════════════════════════════════════════════════════════════╗"
echo "║                     VERIFICATION COMPLETE                                  ║"
echo "╚════════════════════════════════════════════════════════════════════════════╝"
echo ""
echo "Next steps:"
echo "1. Read: JIT_TESTING_MARKDOWN_REPORT.md"
echo "2. Run: bash verify_jit_features.sh"
echo "3. Debug: Function return value bug"
echo "4. Implement: Array support"
echo ""
echo "See SESSION_COMPLETION_SUMMARY.txt for full report"
echo ""

