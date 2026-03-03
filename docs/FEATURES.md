# CoRe Compiler - Complete Feature List

## ✅ Core Language Features
- Semicolon-based syntax (`say;`, `ask:`, `fn name: p1, p2 { }`)
- Variable declarations (`var x: value`)
- Function definitions with parameters
- Arithmetic operations (+, -, *, /)
- Comparison operators (==, !=, <, >)
- Comparison operators (<=, >=)
- **NEW: If/else conditionals**
- **NEW: While loops**
- **NEW: throw / try / catch**
- **NEW: trait / impl / class (method sugar)**
- **NEW: type conversion builtins (str/num/bool/type)**

## ✅ Execution Modes
- **VM execution** (`forge file.fr` or `forge -v file.fr`) - Generates ARM64 assembly and runs it in the VM (default)
- **Rust interpreter** (`forge -r file.fr`) - Instant feedback
- **Native compilation** (`forge --native file.fr`) - ARM64 assembly + link + run

## ✅ Advanced Features
- Python-style dynamic collections
- JavaScript-style async/await (cooperative scheduler in `forge -r`; VM/native currently run async synchronously)
- Rust-style resource GC
- Static analysis (type checking, dead code, stack alignment)
- Self-hosting (`--out`, `--in` flags)

## ✅ ARM64 Virtual Machine
- 31 general-purpose registers
- Full instruction decoder
- Interactive REPL with step debugging
- Colored output
- Program loading from files

## 🎯 Usage

### Quick Start
```bash
# Just type forge and the filename!
./forge examples/hello.fr
./forge -r examples/if_else.fr
./forge -r examples/while_loop.fr
```

### VM Usage
```bash
./vm/target/release/arm64vm
arm64> load vm/test_arithmetic.s
arm64> run
arm64> regs
```

## 📝 Example Programs

1. `examples/hello.fr` - Basic hello world
2. `examples/calculator.fr` - Function composition
3. `examples/async_demo.fr` - Async/await
4. `examples/async_concurrency.fr` - spawn/sleep/await concurrency demo
5. `examples/file_gc.fr` - Resource management
6. `examples/if_else.fr` - Conditionals
7. `examples/while_loop.fr` - Loops

## 🚀 Latest Improvements

### Bash Wrapper Script
- Created `forge` script for easy execution
- Auto-builds if binary doesn't exist
- Just run `./forge file.fr`!

### Control Flow
- **If/else statements** with proper branching
- **While loops** with condition checking
- Label generation for jumps
- Full IR support with JumpIf and Label instructions

### Testing Results
```
./forge -r examples/if_else.fr
→ Output:
x is greater than 40
y is not small
✓ Execution completed successfully
```

## 🎓 What You've Built

A complete compiler toolchain featuring:
- ✅ Lexer with logos
- ✅ Recursive descent parser
- ✅ IR generation with SSA-like temps
- ✅ Static analyzer
- ✅ ARM64 code generator
- ✅ Direct execution interpreter
- ✅ ARM64 virtual machine
- ✅ Control flow (if/else, while)
- ✅ Self-hosting capabilities
