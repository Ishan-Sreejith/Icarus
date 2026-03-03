#!/bin/bash
# End-to-end VM Test Script

FR_FILE=$1
if [ -z "$FR_FILE" ]; then
    echo "Usage: ./vm_test.sh <file.fr>"
    exit 1
fi

echo "→ Compiling $FR_FILE to ARM64 assembly..."
./target/release/forge -n "$FR_FILE" > /dev/null

S_FILE="${FR_FILE%.fr}.s"
if [ ! -f "$S_FILE" ]; then
    echo "✗ Failed to generate $S_FILE"
    exit 1
fi

echo "→ Building ARM64 VM..."
cd vm && cargo build --release > /dev/null 2>&1
cd ..

echo "→ Executing in VM..."
./vm/target/release/arm64vm "$S_FILE"
