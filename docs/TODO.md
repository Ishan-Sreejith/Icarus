# CoRe Language Project Status & TODO

## 🚀 Features Implemented

### Core Language
- [x] **Variables**: `var x: 10`
- [x] **Types**: Number (f64), String, Boolean, Float
- [x] **Control Flow**: `if`, `else`, `while`, `for` loops
- [x] **Functions**: `fn name: params { ... }`
- [x] **Data Structures**:
    - Lists: `[1, 2, 3]`
    - Maps: `{ "key": "value" }`
    - Structs: `struct Point { x, y }`
- [x] **Operators**:
    - Arithmetic: `+`, `-`, `*`, `/`
    - Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`
    - Logical: `and`, `or`, `not`
    - Bitwise: `&`, `|`, `^`, `~`, `<<`, `>>`
- [x] **I/O**: `say: "hello"`, `ask: "prompt"`
- [x] **Error Handling**: `try { ... } catch err { ... }` (Parser & IR support)
- [x] **Imports**: `import "filename.fr"` (Recursive parsing implemented)

### Toolchain
- [x] **Compiler (`forge`)**:
    - Lexer (Logos based)
    - Parser (Recursive Descent)
    - IR Generator (Linear 3-address code)
    - Static Analyzer (Basic type checking & dead code detection)
    - Code Generator (ARM64 Assembly)
- [x] **Interpreter**: Direct execution of IR (`forge main.fr`)
- [x] **Virtual Machine (`arm64vm`)**:
    - Executes generated ARM64 assembly
    - Supports heap allocation (`malloc`)
    - Supports system calls (`write`, `read`)
    - Supports floating point & bitwise operations
- [x] **Plugin Manager (`metroman`)**:
    - Generates syntax plugin templates (`metroman --out plugin.fr`)
- [x] **Syntax Customization**:
    - Export syntax: `forge --out`
    - Import syntax & rebuild: `forge --in`

### Execution Modes
- [x] **VM (Default)**: `forge main.fr` / `forge -v main.fr`
- [x] **Rust Interpreter**: `forge -r main.fr`
- [x] **Native Compilation**: `forge --native main.fr` -> `./main`
- [x] **Assembly VM (Legacy Flag)**: `forge --asm main.fr`

---

## 🚧 Pipelines & Architecture

### Compilation Pipeline
1.  **Source Code** (`.fr`) -> **Lexer** -> **Tokens**
2.  **Tokens** -> **Parser** -> **AST** (Abstract Syntax Tree)
3.  **AST** -> **IR Builder** -> **IR** (Intermediate Representation)
4.  **IR** -> **Analyzer** -> **Optimized IR** (Basic checks)
5.  **Optimized IR** -> **Codegen** -> **ARM64 Assembly** (`.s`)
6.  **Assembly** -> **Assembler (`as`)** -> **Object File** (`.o`)
7.  **Object File** -> **Linker (`ld`)** -> **Executable**

### Execution Pipeline (Interpreter)
1.  **Source Code** -> ... -> **IR**
2.  **IR** -> **DirectExecutor** -> **Result**

### Execution Pipeline (VM)
1.  **Source Code** -> ... -> **Assembly**
2.  **Assembly** -> **VM Loader** -> **VM Memory/Program**
3.  **VM** -> **Step-by-step Execution** -> **Result**

---

## 📝 TODO / Missing Features

### High Priority
- [ ] **Async/Await Runtime**:
    - The parser and IR support `async`/`await`, but the backend generates empty stubs.
    - Needs a simple task scheduler or coroutine implementation in assembly/runtime.
- [ ] **File I/O Implementation**:
    - `AllocFile` and `CloseFile` IR instructions are stubbed.
    - Needs `open`, `read`, `write`, `close` syscall implementations in codegen.
- [ ] **Exception Handling Runtime**:
    - `try/catch` is parsed but not fully wired in codegen.
    - Needs stack unwinding or a setjmp/longjmp equivalent mechanism.

### Medium Priority
- [ ] **Standard Library**:
    - More built-in functions for string manipulation, math, etc.
- [ ] **Type System Improvements**:
    - Stricter type checking in the Analyzer.
    - Struct field type validation.
- [ ] **Metroman Integration**:
    - Make `import` automatically apply syntax changes defined in plugins (currently requires manual `forge --in`).

### Low Priority / Future
- [ ] **x86_64 Support**: Port codegen to Intel/AMD architecture.
- [ ] **Optimization Pass**: Implement constant folding, dead code elimination in IR.
- [ ] **Debugger**: Source-level debugging support.

---

## 🐛 Known Issues
- **Float/Int Mixing**: The type system is permissive; mixing floats and ints might cause runtime weirdness in native mode if not carefully cast.
- **Memory Management**: Basic reference counting is implemented, but cycle detection is missing (potential leaks for circular references).
