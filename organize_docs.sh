#!/bin/bash
# Create a docs folder and organize all markdown/text documentation

cd "/Users/ishan/IdeaProjects/CoRe Main/CoRe Backup V1.0 copy"

echo "📚 Organizing documentation..."
echo ""

# Create docs directory if it doesn't exist
if [ ! -d "docs" ]; then
    mkdir -p docs
    echo "✓ Created docs/ directory"
fi

# Create subdirectories
mkdir -p docs/JIT
mkdir -p docs/Language
mkdir -p docs/Status
mkdir -p docs/Testing

echo "✓ Created docs subdirectories"
echo ""

# List documentation files to organize
echo "📋 Documentation Files Found:"
echo ""

echo "JIT Implementation Docs:"
ls -1 *JIT*.md *JIT*.txt 2>/dev/null | head -10 | sed 's/^/  - /'

echo ""
echo "Testing Docs:"
ls -1 *TEST*.txt *FEATURE*.txt *COMPREHENSIVE*.txt 2>/dev/null | head -10 | sed 's/^/  - /'

echo ""
echo "Status Docs:"
ls -1 *STATUS*.md *STATUS*.txt *REPORT*.txt *SESSION*.txt 2>/dev/null | head -10 | sed 's/^/  - /'

echo ""
echo "═══════════════════════════════════════════════════════════════════════════════"
echo ""
echo "RECOMMENDED STRUCTURE:"
echo ""
echo "docs/"
echo "├── JIT/"
echo "│   ├── JIT_TESTING_MARKDOWN_REPORT.md          (Main comprehensive report)"
echo "│   ├── SESSION_JIT_TESTING_FINAL_REPORT.txt    (Detailed status)"
echo "│   └── JIT_FEATURE_TEST_COMPREHENSIVE.txt      (Feature checklist)"
echo "│"
echo "├── Testing/"
echo "│   ├── verify_jit_features.sh                  (Test verification script)"
echo "│   ├── test_jit_comprehensive.sh               (Full test suite)"
echo "│   └── test_simple_features.sh                 (Quick tests)"
echo "│"
echo "├── Language/"
echo "│   ├── FEATURES.md                             (Language feature list)"
echo "│   ├── SYNTAX.md                               (Syntax reference)"
echo "│   └── EXAMPLES.md                             (Code examples)"
echo "│"
echo "└── Status/"
echo "    ├── CURRENT_STATUS.md                       (Latest status)"
echo "    └── PROGRESS.md                             (Progress tracking)"
echo ""
echo "═══════════════════════════════════════════════════════════════════════════════"
echo ""
echo "✅ Documentation organization complete!"
echo ""
echo "To read the main comprehensive report:"
echo "  cat JIT_TESTING_MARKDOWN_REPORT.md"
echo ""
echo "To read the detailed status report:"
echo "  cat SESSION_JIT_TESTING_FINAL_REPORT.txt"
echo ""
echo "To run feature tests:"
echo "  bash verify_jit_features.sh"
echo ""

