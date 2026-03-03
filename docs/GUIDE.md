# The Complete CoRe Programming Language Guide

Welcome to the comprehensive guide for **CoRe**, a modern, self-hosting programming language designed for performance, safety, and a stunning developer experience on ARM64 Apple Silicon.

---

## 🚀 Quick Start

### Installation
The CoRe compiler is built with Rust. Ensure you have `cargo` installed.

```bash
git clone <repository-url>
cd "CoRe Backup V1.0 copy"
cargo build --release
```

### Running Your First Program
Create a file named `hello.fr`:
```core
say: "Hello, CoRe!"
```

Run it immediately on the ARM64 VM (default):
```bash
forge hello.fr
forge -v hello.fr
```

Or run it with the Rust interpreter (instant feedback):
```bash
forge -r hello.fr
```

Or compile it to native ARM64:
```bash
forge --native hello.fr  # This will also auto-execute the binary
forge -n hello.fr
```

> [!TIP]
> **Forge is installed globally!** You can now run the `forge` command from any directory on your device.

---

## 📝 Language Reference

### Basic Syntax
CoRe uses a semicolon-based syntax for commands and colon-based anchors for inputs and declarations.

#### Variables
```core
var x: 42
var name: "CoRe"
```

#### I/O
- `say: <expr>`: Prints a value to stdout.
- `ask: <string>`: Prompts the user and returns input as a string.

```core
var user: ask: "What is your name? "
say: "Welcome, "
say: user
```

### Control Flow
CoRe supports standard branching and iteration.

#### If/Else
```core
if x > 10 {
    say: "Large"
} else {
    say: "Small"
}
```

#### While Loops
```core
var i: 0
while i < 10 {
    say: i
    var i: i + 1
}
```

#### For-In Loops
Iterate over lists or ranges.
```core
var list: [1, 2, 3]
for x in list {
    say: x
}

for i in 0..5 {
    say: i
}
```

### Functions
Functions are defined with `fn` and called with `:`.

```core
fn add: a, b {
    return a + b
}

var sum: add: 5, 10
say: sum
```

### Advanced Features
- **Async/Await**: `async fn` and `await` for non-blocking I/O.
- **Collections**: Python-style lists and maps.
- **Floating Point**: Support for `3.14` style literals and math.
- **Bitwise**: Support for `&`, `|`, `^`, `~`, `<<`, `>>`.
- **Structs / Classes**: Define with `struct Point { x, y }` or `class Point { x, y }` and access with `.`.
- **Traits**: `trait Name { fn method: self; }` + `impl Name for Type { fn method: self { ... } }`.
- **Error Handling**: `try { ... } catch err { ... }` plus `throw <expr>`.
- **Resource GC**: Automatic cleanup of files and memory.

> Notes:
> - `forge -r` supports cooperative async via `spawn:` / `await` / `sleep:`.
> - VM/default (`forge file.fr` / `forge -v`) and `--native` currently execute `async fn` synchronously; `await` is effectively a no-op and `sleep:` is a no-op.
> - `throw`/`try`/`catch` currently catches **explicit `throw` inside the same function** (no cross-function unwinding).
> - Method call sugar: `obj.method: a, b` desugars to calling `Type_method(obj, a, b)` when `obj` is a known struct/class variable.

### Type Conversion Builtins

```core
say: str: 123
say: num: "456"
say: bool: 0
say: type: [1, 2, 3]
```

### Syntax Changes
- **I/O Commands**: `say` and `ask` now use a colon-separated syntax for their arguments.
  ```core
  say: "Hello, CoRe!"
  var user: ask: "What is your name? "
  ```
- **Function Calls**: Functions are called using `fnname: params`.
  ```core
  var result: add: 5, 10
  ```
- **Function Definitions**: Defined using `fn name: params { ... }`.
  ```core
  fn greet: name {
      say: "Hello, "
      say: name
  }
  ```

---

## Ⓜ️ Metroman (Plugin Manager)

Metroman is a tool to manage CoRe plugins and custom syntax addons.

### Usage
- `metroman --out <filename.mtro>`: Generates a new plugin template or opens an existing one.

Example of a plugin file (`myplugin.mtro`):
```core
fn custom-greet: value {
  say: "Plugin says: "
  say: value
}
```

In your `main.fr`:
```core
import "myplugin.mtro"
custom-greet: "Hello from main!"
```

---

## 🏗️ Compiler Architecture

The CoRe compiler (`forge`) follows a traditional multi-stage pipeline:

1.  **Lexer (`lexer.rs`)**: Uses the `logos` crate to tokenize source code.
2.  **Parser (`parser.rs`)**: A recursive descent parser that produces an Abstract Syntax Tree (AST).
3.  **IR Generation (`ir.rs`)**: Translates the AST into a Three-Address Code Intermediate Representation. It performs basic type tracking (`Number`, `String`, `Bool`).
4.  **Static Analyzer (`analyzer.rs`)**: Checks for type errors, dead code, and ensures stack alignment.
5.  **Codegen**:
    - **Native ARM64 (`codegen/arm64.rs`)**: Produces optimized assembly for macOS Apple Silicon. Activated with the `-n` flag.
    - **Direct Executor (`codegen/direct.rs`)**: A Rust interpreter that executes IR directly for fast development cycles. Activated with `-r` (or legacy `-d`).
    - **Assembly VM (`vm/`)**: A custom ARM64 emulator for low-level testing. Default execution mode (or `-v`; legacy `-a`).

---

## 💻 ARM64 Backend Implementation

The ARM64 backend is specifically tuned for **Apple Silicon (A-series/M-series)**:

### Stack Management
- **Frame Size**: Fixed at 1024 bytes per function for simplicity and 16-byte alignment.
- **Registers**: Standard preservation of `x29` (frame pointer) and `x30` (link register).
- **Parameters**: The first 8 parameters (x0-x7) are immediately saved to the stack to prevent clobbering by nested function calls.

### Numeric Printing & Variadic Calls
MacOS ARM64 has a specific calling convention for variadic functions like `printf`:
- **Variadic Arguments**: Must be passed on the stack, not in registers.
- **_print_num**: A custom helper in `arm64.rs` handles this by placing the numeric value at `[sp]` before calling `_printf`.
- **Buffering**: Automated `fflush(0)` calls ensure that output is visible in the terminal even if the program terminates via a raw syscall.

---

## 🛠️ ARM64 Virtual Machine

For low-level debugging, CoRe includes a dedicated ARM64 VM.

### Usage
```bash
cd vm
cargo run --release
```

### REPL Commands
- `load <file.s>`: Load an assembly file.
- `step`: Execute one instruction.
- `regs`: Inspect all 31 general-purpose registers.
- `run`: Execute until a `svc #0x80` exit call.

---

## 🔄 Self-Hosting

CoRe supports modifying its own syntax via a mapping file.

1.  **Dump Syntax**: `./forge --out` creates `syntax.fr`.
2.  **Modify**: Change keywords in `syntax.fr` (e.g., change `say;` to `print;`).
3.  **Apply**: `./forge --in` re-parses the compiler's own logic with the new syntax.

---

## 🗺️ Roadmap & Missing Features

While CoRe 1.0 is a powerful toolchain, there are several areas for future expansion:

### 1. ARM64 Backend Enhancements
- **True Async/Await (native/VM)**: `forge -r` supports cooperative async, but native/VM still treat `async fn` synchronously.
- **Native Collections**: Assembly implementation for `AllocList`, `GetIndex`, and `SetIndex` to support lists and maps natively.
- **String Manipulation**: Native support for concatenation, slicing, and length checks.

### 2. Language Features
- **For Loops**: Traditional `for i: 0 to 10` style loops (currently only `while` is supported).
- **Cross-function errors**: `throw`/`try` currently does not unwind across calls.
- **Richer stdlib**: More builtins (string ops, math, IO).

### 3. Virtual Machine (SVM)
The integrated ARM64 VM is currently a **simulation tool** for low-level logic. It has the following limitations:
- **Coverage**: Instruction/support is focused on what `forge` emits; unrelated ARM64 instructions may be unimplemented.
- **C Library Interop**: Only a small emulated subset is provided (enough for `forge` output), not a full libc.
- **Assembler Directives**: Supports the directives needed for `forge` output, not a full assembler.

---

## 🛠️ Performance Tips
- Use **VM Mode** (default, `forge file.fr` or `-v`) for a portable, debuggable run.
- Use **Rust Interpreter** (`-r`) for the fastest compile/run loop while developing.
- Use **Native Mode** (`--native` / `-n`) when you want a real ARM64 binary.
