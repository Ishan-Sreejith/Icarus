# CoRe Compiler & ARM64 VM - Quick Reference

## CoRe Compiler

### Build
```bash
cd "/Users/ishan/IdeaProjects/CoRe Backup V1.0 copy"
cargo build --release
```

### Usage
```bash
# VM execution (default)
./target/release/forge examples/hello.fr
./target/release/forge -v examples/hello.fr

# Rust interpreter (instant feedback)
./target/release/forge -r examples/hello.fr

# Native compilation
./target/release/forge --native examples/calculator.fr

# Self-hosting
./target/release/forge --out  # Dump syntax
./target/release/forge --in   # Reload syntax
```

## ARM64 Virtual Machine

### Build
```bash
cd "/Users/ishan/IdeaProjects/CoRe Backup V1.0 copy/vm"
cargo build --release
```

### Usage
```bash
# Start REPL
./target/release/arm64vm

# Commands
help          - Show help
load <file>   - Load assembly file
run           - Run to completion
step          - Execute one instruction
regs          - Show registers
prog          - Show program listing
reset         - Reset VM
quit          - Exit
```

### Test Programs
- `vm/test_arithmetic.s` - Basic arithmetic
- `vm/test_loop.s` - Loop with branches
- `vm/test_function.s` - Function calls

## Verified Test Results

✅ **Arithmetic**: 10+5=15, 10-5=5, 10*5=50, 10/5=2
✅ **Loops**: Count 0→10 with conditional branches
✅ **Interactive**: Step-by-step execution with register inspection
✅ **Compiler**: Direct execution and native ARM64 generation
