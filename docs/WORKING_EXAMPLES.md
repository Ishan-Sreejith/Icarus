# Working Examples - What Actually Runs

These examples are tested and work with **forger** (interpreter) and **forge --native** (AOT compiler).

## ✅ Basic Arithmetic
```forge
var x: 10
var y: 20
var sum: x + y
var diff: y - x
var prod: x * y
say: sum    // 30
say: diff   // 10
say: prod   // 200
```

## ✅ Functions
```forge
fn add: a, b {
    var result: a + b
    return result
}

var total: add: 5, 7
say: total  // 12
```

## ✅ Conditionals
```forge
var x: 15

if x > 10 {
    say: "Big"
} else {
    say: "Small"
}
```

## ✅ Say/Ask (I/O)
```forge
say: "Hello, World!"

var name: "Alice"
say: "Name:"
say: name
```

## ✅ Lists (Basic)
```forge
var numbers: [1, 2, 3, 4, 5]
say: numbers
```

## ✅ Nested Expressions
```forge
var a: 2
var b: 3
var c: 4
var result: (a + b) * c
say: result  // 20
```

## ✅ Multiple Functions
```forge
fn greet: name {
    say: "Hello"
    say: name
}

fn calculate: x, y {
    return x * y
}

greet: "Bob"
var prod: calculate: 6, 7
say: prod  // 42
```

## ✅ Working Complete Program
```forge
// test_working.fr
fn startup {
    say: "Starting..."
}

fn process: x, y {
    var sum: x + y
    var prod: x * y
    say: "Sum:"
    say: sum
    say: "Product:"
    say: prod
    return prod
}

startup
var result: process: 5, 7
say: "Final result:"
say: result
say: "Done!"
```

**Test it**:
```bash
./target/release/forger test_working.fr
./target/release/forge --native test_working.fr
```

## ❌ Features Not Yet Working in JIT/VM

### Don't use these (they cause errors):
- `for` loops with ranges: `for i in 1..10`
- `for` loops with lists: `for item in list`
- Maps/dictionaries: `{"key": "value"}`
- Map access: `map["key"]`
- Built-in functions: `is_map()`, `is_list()`
- `while` loops (limited support)
- Classes and traits (not in JIT)

## Recommended Workflow

### For Development:
```bash
# Use interpreter - it's fast and reliable
./target/release/forger myprogram.fr
```

### For Production:
```bash
# Compile to native binary
./target/release/forge --native myprogram.fr
./myprogram
```

### Don't Use:
```bash
# JIT crashes - don't use yet
./target/release/fforge myprogram.fr  ❌

# VM has bugs - don't use yet
./target/release/core myprogram.fr  ❌
```

## Testing Your Programs

Create a simple test file:
```bash
cat > test.fr << 'EOF'
var x: 42
say: "Answer:"
say: x
EOF

# Test with interpreter (reliable)
./target/release/forger test.fr

# Test with AOT (also reliable)
./target/release/forge --native test.fr
```

## Full Working Example Programs

See these files in the project:
- `test_arithmetic.fr` - Basic math ✅
- `test_simple_add.fr` - Simple addition ✅
- `main_simple.fr` - Complete program ✅
- `examples/hello.fr` - Hello world ✅
- `examples/calculator.fr` - Calculator ✅

## Summary

**Use**: `forger` (interpreter) or `forge --native` (AOT)  
**Avoid**: `fforge` (JIT) and `core` (VM) until they're fixed

**Supported Features**:
- ✅ Variables
- ✅ Arithmetic (+, -, *, /)
- ✅ Functions
- ✅ Function calls
- ✅ Conditionals (if/else)
- ✅ Say/output
- ✅ Return values
- ✅ Nested expressions

**Not Supported (Yet)**:
- ❌ Loops (for/while)
- ❌ Lists (advanced operations)
- ❌ Maps/dictionaries
- ❌ Classes/traits
- ❌ Built-in functions

