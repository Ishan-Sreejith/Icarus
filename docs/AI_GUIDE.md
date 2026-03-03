# CoRe Language & Toolchain Guide for AI Agents

This document provides a comprehensive reference for AI agents to understand, write, compile, and execute code in the CoRe language, as well as manage the toolchain.

---

## 1. Toolchain Commands

The toolchain consists of two main binaries: `forge` (the compiler/interpreter) and `metroman` (the plugin manager).

### `forge` - The Compiler & Runner

| Command | Description | Example |
| :--- | :--- | :--- |
| `forge <file.fr>` | **VM Mode (Default)**. Generates ARM64 assembly and runs it on the internal ARM64 emulator. | `forge main.fr` |
| `forge -v <file.fr>` | **VM Mode (Explicit)**. Same as default. | `forge -v main.fr` |
| `forge -r <file.fr>` | **Rust Interpreter Mode**. Runs the IR directly (fastest iteration). | `forge -r main.fr` |
| `forge --native <file.fr>` | **Native Compilation**. Compiles to ARM64 assembly, links, and produces a binary. | `forge --native main.fr` -> `./main` |
| `forge --clean` | **Cleanup**. Removes generated artifacts (`.s`, `.o`, binaries) from the current directory. | `forge --clean` |
| `forge --out` | **Export Syntax**. Dumps the current language syntax mapping to `syntax.fr`. | `forge --out` |
| `forge --in` | **Import Syntax**. Reads `syntax.fr`, updates the lexer, and rebuilds the compiler. | `forge --in` |
| `forge --install` | **Install**. Copies `forge` and `metroman` to `/usr/local/bin/`. | `sudo forge --install` |

### `metroman` - The Plugin Manager

| Command | Description | Example |
| :--- | :--- | :--- |
| `metroman --out <file.fr>` | **Create Plugin**. Generates a template syntax file for creating plugins. | `metroman --out myplugin.fr` |

---

## 2. CoRe Language Syntax

CoRe is a statically-typed (inferred), imperative language with a focus on simplicity.

### Basic Structure
```core
# Comments start with hash
fn main: {
    say: "Hello World"
}
```

### Variables & Types
```core
var x: 10          # Number (f64)
var y: 3.14        # Float
var s: "String"    # String
var b: 1           # Boolean (1=true, 0=false)
```

### I/O
```core
say: "Output to stdout"
var name: ask: "What is your name? "
```

### Control Flow
```core
# If / Else
if x > 5 {
    say: "Big"
} else {
    say: "Small"
}

# While Loop
while x > 0 {
    x = x - 1
}

# For Loop (Range)
for i in 0..10 {
    say: i
}

# For Loop (List)
for item in list {
    say: item
}
```

### Functions
Defined with `fn`, called with `:`.
```core
fn add: a, b {
    return a + b
}

var result: add: 5, 10
```

### Data Structures

**Lists**
```core
var list: [1, 2, 3]
var item: list[0]
list[1] = 5
```

**Maps**
```core
var map: { "key": "value", "a": 1 }
var val: map["key"]
map["b"] = 2
```

**Structs**
```core
struct Point { x, y }

var p: Point
p.x = 10
p.y = 20
say: p.x
```

### Operators
*   **Math**: `+`, `-`, `*`, `/`
*   **Comparison**: `==`, `!=`, `<`, `>`, `<=`, `>=`
*   **Logic**: `and`, `or`, `not`
*   **Bitwise**: `&` (AND), `|` (OR), `^` (XOR), `~` (NOT), `<<` (SHL), `>>` (SHR)

### Error Handling
```core
try {
    # risky code
} catch err {
    say: "An error occurred"
}
```

### Imports
```core
import "other_file.fr"
```

Imports are resolved **relative to the importing file's directory** (like most languages), and are loaded at compile time. You can also import plugin-style files (e.g. `import "myplugin.mtro"`) as long as they use valid CoRe syntax.

---

## 3. Workflow for AI Agents

1.  **Write Code**: Generate a `.fr` file (e.g., `script.fr`) using the syntax above.
2.  **Verify**: Run `forge -r script.fr` for the fastest iterate/debug loop (Rust interpreter).
3.  **Run on VM**: Run `forge script.fr` (default) to execute via the ARM64 VM.
3.  **Compile (Optional)**: If a binary is needed, run `forge --native script.fr`.
4.  **Cleanup**: Run `forge --clean` to remove temporary files (`script.s`, `script.o`, `script`).

## 4. Customizing Syntax (Advanced)

To change the language keywords (e.g., rename `fn` to `func`):
1.  `forge --out` -> generates `syntax.fr`.
2.  Edit `syntax.fr` JSON content.
3.  `forge --in` -> Rebuilds the compiler.
4.  `sudo forge --install` -> Updates the system binary.
