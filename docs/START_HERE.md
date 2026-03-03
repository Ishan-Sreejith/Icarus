t# Quick Start - What Actually Works

## TL;DR

**Use these commands** (they work):
```bash
# Interpreter - fast, reliable
./target/release/forger myprogram.fr

# Native compiler - for production
./target/release/forge --native myprogram.fr
./myprogram
```

**Don't use these** (they're broken):
```bash
./target/release/fforge myprogram.fr  # JIT crashes ❌
./target/release/core myprogram.fr    # VM broken ❌
```

## Simple Working Example

Create `hello.fr`:
```forge
var message: "Hello from CoRe!"
say: message

var x: 10
var y: 32
var result: x + y
say: "Result:"
say: result
```

Run it:
```bash
./target/release/forger hello.fr
```

Output:
```
Hello from CoRe!
Result:
42
```

## What Works

- ✅ Variables: `var x: 42`
- ✅ Arithmetic: `+`, `-`, `*`, `/`
- ✅ Functions: `fn add: a, b { return a + b }`
- ✅ Conditionals: `if x > 10 { ... } else { ... }`
- ✅ Output: `say: "text"` or `say: variable`

## What Doesn't Work (Yet)

- ❌ `for` loops
- ❌ `while` loops  
- ❌ Lists/arrays (advanced operations)
- ❌ Maps/dictionaries
- ❌ `is_map()`, `is_list()` functions
- ❌ Range syntax `1..10`

## If You Get Errors

### "Unknown function: is_map"
Your code uses unsupported features. Remove:
- `is_map()`, `is_list()`, `is_string()`
- For loops with lists/ranges
- Map operations

### "Unknown instruction: var"
You're using `core` (VM) - it's broken. Use `forger` instead:
```bash
./target/release/forger myprogram.fr
```

### Segmentation fault
You're using `fforge` (JIT) - it crashes. Use `forger` instead:
```bash
./target/release/forger myprogram.fr
```

## Example: main.fr (Fixed Version)

**Original** (doesn't work):
```forge
var range_data: 1..5  // ❌ Not supported
for item in data {     // ❌ Not supported
    if is_map(x) {     // ❌ Not supported
```

**Fixed** (works):
```forge
var x: 5
var y: 7
if x > 3 {
    say: "Big"
}
```

## Complete Working Program

```forge
// Save as: my_program.fr

fn greet: name {
    say: "Hello"
    say: name
}

fn calculate: a, b {
    var sum: a + b
    var product: a * b
    say: "Sum:"
    say: sum
    say: "Product:"
    say: product
    return product
}

greet: "Ishan"
var result: calculate: 6, 7
say: "Final:"
say: result
```

Run:
```bash
./target/release/forger my_program.fr
```

Output:
```
Hello
Ishan
Sum:
13
Product:
42
Final:
42
```

## Installation

If commands don't work without `./target/release/`, install to PATH:
```bash
./install.sh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc

# Now you can use:
forger myprogram.fr
forge --native myprogram.fr
```

## Summary

**Working**:
- Interpreter (`forger`) ✅
- AOT Compiler (`forge --native`) ✅

**Broken**:
- JIT (`fforge`) ❌
- VM (`core`) ❌

**Recommendation**: Use `forger` for everything until JIT is fixed.

---

See `WORKING_EXAMPLES.md` for more examples.
See `HONEST_REPORT.md` for what's broken and why.

