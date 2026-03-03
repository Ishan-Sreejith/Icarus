# CoRe Language - Installation & Usage Guide

## Installation

### Option 1: Using the install script (Recommended)

```bash
cd "/Users/ishan/IdeaProjects/CoRe Main/CoRe Backup V1.0 copy"
./install.sh
```

### Option 2: Manual installation

```bash
cd "/Users/ishan/IdeaProjects/CoRe Main/CoRe Backup V1.0 copy"

# Build all binaries
cargo build --release

# Create install directory
mkdir -p ~/.local/bin

# Copy binaries
cp target/release/forge ~/.local/bin/
cp target/release/core ~/.local/bin/
cp target/release/fforge ~/.local/bin/
cp target/release/forger ~/.local/bin/
cp target/release/metroman ~/.local/bin/

# Make them executable
chmod +x ~/.local/bin/{forge,core,fforge,forger,metroman}
```

### Option 3: Using the forge binary's --install flag

```bash
cd "/Users/ishan/IdeaProjects/CoRe Main/CoRe Backup V1.0 copy"
cargo build --release
./target/release/forge --install
```

## Adding to PATH

Add this line to your `~/.zshrc`:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Then reload your shell:

```bash
source ~/.zshrc
```

## Command Usage

Once installed, you can use the binaries without `./` prefix:

### Core (Main Binary - VM)

```bash
# Run with VM (default)
core main.fr

# Run with Rust interpreter
core -r main.fr
core --rust main.fr

# Run with Assembly VM
core -a main.fr
core --asm main.fr

# Verbose mode
core -v main.fr
core --verbose main.fr
```

### Forge (VM wrapper + Native compiler)

```bash
# Run with VM (same as core)
forge main.fr

# Native AOT compilation
forge --native main.fr
forge -n main.fr

# JIT mode (in progress)
forge --jit main.fr
```

### FForge (JIT Compiler)

```bash
# JIT compilation and execution
fforge main.fr
```

### Forger (Rust Interpreter)

```bash
# Direct interpretation in Rust
forger main.fr
```

### Metroman (Plugin Manager)

```bash
# List available plugins
metroman list

# Install a plugin
metroman install <plugin-name>

# Remove a plugin
metroman remove <plugin-name>

# Show plugin info
metroman info <plugin-name>
```

## Example Usage

```bash
# Create a simple CoRe program
cat > hello.fr << 'EOF'
say: "Hello, World!"
EOF

# Run with VM
core hello.fr

# Run with interpreter
forger hello.fr

# Compile with JIT
fforge hello.fr
```

## Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_jit_simple

# Run tests with output
cargo test -- --nocapture
```

## Execution Pipelines

CoRe supports 4 execution pipelines:

1. **VM (Virtual Machine)**: `core main.fr` - Default, bytecode execution
2. **Interpreter**: `forger main.fr` - Direct AST interpretation  
3. **JIT (Just-In-Time)**: `fforge main.fr` - Runtime compilation to machine code
4. **AOT (Ahead-Of-Time)**: `forge --native main.fr` - Compile to native assembly

## Troubleshooting

### "command not found: core"

Make sure ~/.local/bin is in your PATH:

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### Binaries not executable

```bash
chmod +x ~/.local/bin/{forge,core,fforge,forger,metroman}
```

### Binaries don't exist

Build them first:

```bash
cd "/Users/ishan/IdeaProjects/CoRe Main/CoRe Backup V1.0 copy"
cargo build --release
```

## Development

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Build specific binary
cargo build --release --bin core
cargo build --release --bin fforge
```

### Running without installation

```bash
# From project root
./target/release/core main.fr
./target/release/fforge main.fr
./target/release/forger main.fr
```

## Features

- ✅ VM execution
- ✅ Direct interpretation
- ✅ Native AOT compilation
- ✅ JIT compilation (Phases 1-10 complete, Phase 11 in progress)
- ✅ Garbage collection
- ✅ Async/await support
- ✅ Classes and traits
- ✅ Plugin system (Metroman)
- ✅ Import system
- ✅ Pattern matching
- ✅ Type inference

## Next Steps

See `FEATURES.md` for detailed language features and `JIT_PHASE11.md` for advanced JIT optimizations.

