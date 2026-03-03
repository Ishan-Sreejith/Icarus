# CoRe Language - Quick Reference

## Installation (One-Time Setup)

```bash
cd "/Users/ishan/IdeaProjects/CoRe Main/CoRe Backup V1.0 copy"
./install.sh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

## Commands (After Installation)

| Command | Description | Use Case |
|---------|-------------|----------|
| `core file.fr` | VM execution | Default, fast startup |
| `forger file.fr` | Interpreter | Debugging |
| `fforge file.fr` | JIT compiler | Best performance |
| `forge --native file.fr` | AOT compiler | Standalone binaries |

## Common Tasks

### Run a Program
```bash
core hello.fr
```

### Compile to Native
```bash
forge --native myapp.fr
./myapp  # Run the compiled binary
```

### Verbose Output
```bash
core -v myapp.fr
forge -v --native myapp.fr
```

### Get Help
```bash
core --help
forge --help
```

## Language Basics

### Variables
```forge
var x: 42
var name: "Alice"
var list: [1, 2, 3]
```

### Functions
```forge
fn add(a, b) {
    return a + b
}
```

### Classes
```forge
class Person {
    var name: "Unknown"
    
    fn greet() {
        say: "Hello, " + self.name
    }
}

var p: Person
p.name: "Bob"
p.greet()
```

### Output
```forge
say: "Hello, World!"
say: 42
say: x
```

### Input
```forge
ask: "Enter your name"
var name: input
```

### Conditionals
```forge
if x > 10 {
    say: "Big"
} else {
    say: "Small"
}
```

### Loops
```forge
for i in range(0, 10) {
    say: i
}

while x < 100 {
    x: x + 1
}
```

## Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_jit_simple

# Run and show output
cargo test -- --nocapture
```

## Build from Source

```bash
cd "/Users/ishan/IdeaProjects/CoRe Main/CoRe Backup V1.0 copy"

# Debug build
cargo build

# Release build (recommended)
cargo build --release

# Build specific binary
cargo build --release --bin fforge
```

## Troubleshooting

### Command not found
```bash
# Make sure PATH is set
echo $PATH | grep ".local/bin"

# If not, add it
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### Re-install binaries
```bash
cd "/Users/ishan/IdeaProjects/CoRe Main/CoRe Backup V1.0 copy"
cargo build --release
./install.sh
```

## File Extensions

- `.fr` - CoRe source files
- `.mtro` - Metroman plugin files
- `.s` - Assembly output files

## Documentation

- `INSTALL_USAGE.md` - Detailed installation & usage
- `FEATURES.md` - Language features
- `PROJECT_STATUS.md` - Current status
- `JIT_PHASE11.md` - JIT optimizations
- `README.md` - Project overview

## Example Programs

Located in `examples/` directory:
- `hello.fr` - Hello World
- `calculator.fr` - Simple calculator
- `classes_traits.fr` - OOP example
- `async_await.fr` - Async example
- `foreach_map.fr` - Collections example

## Performance Tips

1. Use `fforge` for compute-intensive tasks
2. Use `core` (VM) for quick scripts
3. Use `forge --native` for production deployments
4. Use `forger` for development/debugging

## Support

Check documentation files for detailed information or run:
```bash
core --help
forge --help
fforge --help
```

