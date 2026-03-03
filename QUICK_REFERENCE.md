# CoRe Language - Quick Reference Card

## Running Programs

```bash
# JIT Compilation (Fastest - 2-5x speedup)
fforge main.fr

# VM Execution (Baseline)
forge main.fr

# Rust Interpreter (Compatibility)
forger main.fr

# Assembly VM
forge -a main.fr
```

## Syntax at a Glance

### Variables
```core
var x: 10
var name: "Alice"
var list: [1, 2, 3]
var dict: {"a": 1, "b": 2}
```

### Output
```core
say: "Hello"
say: x
say: list
```

### Arithmetic
```core
var sum: 5 + 3      // 8
var diff: 10 - 4    // 6
var prod: 6 * 7     // 42
var quot: 20 / 4    // 5
```

### Strings
```core
var greeting: "Hello " + "World"
say: greeting       // "Hello World"
```

### Conditionals
```core
if x > 10 {
    say: "Greater"
} else {
    say: "Not greater"
}
```

### Loops
```core
var i: 0
while i < 5 {
    say: i
    i = i + 1
}
```

### Functions
```core
fn add: x y {
    var result: x + y
    result
}

var answer: add: 5 3  // 8
```

## Common Operations

| Operation | Syntax | Example |
|-----------|--------|---------|
| Addition | `a + b` | `5 + 3` |
| Subtraction | `a - b` | `10 - 4` |
| Multiplication | `a * b` | `6 * 7` |
| Division | `a / b` | `20 / 4` |
| Less than | `a < b` | `5 < 10` |
| Greater than | `a > b` | `10 > 5` |
| Less/equal | `a <= b` | `5 <= 5` |
| Greater/equal | `a >= b` | `10 >= 10` |
| Equal | `a == b` | `5 == 5` |
| Not equal | `a != b` | `5 != 3` |
| And | `a and b` | `true and false` |
| Or | `a or b` | `true or false` |

## File Structure

```
.
├── main.fr                  ← Your program here
├── src/                     ← Source code
├── examples/                ← Example programs
└── docs/                    ← Documentation
```

## Testing Your Code

```bash
# Build once
cargo build --release

# Test with all modes
./target/release/fforge main.fr     # JIT
./target/release/forge main.fr      # VM
./target/release/forger main.fr     # Interpreter
./target/release/forge -a main.fr   # Assembly

# Run test suite
bash test_all_features.sh
```

## Performance Tips

1. **Use JIT for speed**: `fforge main.fr` is 2-5x faster
2. **Use VM for safety**: `forge main.fr` is stable and reliable
3. **Use Interpreter for debugging**: `forger main.fr` shows detailed info
4. **Minimize allocations**: Less list/map creation = faster code
5. **Keep functions small**: Helps with optimization

## Error Handling

```core
try {
    var x: some_operation
} catch {
    say: "Error occurred"
}
```

## Lists & Maps

```core
// Lists
var list: [1, 2, 3]
say: list

// Maps
var map: {"key": "value", "x": 10}
say: map
```

## Debugging

```bash
# Check syntax
forge main.fr   # Shows any parsing errors

# See detailed output
forger main.fr  # Rust interpreter with debug info

# Check optimizations
fforge --verbose main.fr  # Shows JIT compilation info
```

## Installation

```bash
cd "/Users/ishan/IdeaProjects/CoRe Main/CoRe Backup V1.0 copy"
cargo build --release
```

## Documentation

For more help:
- See `README.md` for overview
- Check `docs/` folder for detailed docs
- Look at `examples/` for sample programs

---

**CoRe Language v1.0** | Production Ready ✅

