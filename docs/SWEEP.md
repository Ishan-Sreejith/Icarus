# CoRe Compiler - SWEEP Reference

## Build & Test Commands

```bash
# Build (dev)
cargo build

# Build (release)
cargo build --release

# Run all tests
cargo test

# Run via ARM64 VM (default)
./target/debug/forge <file.fr>
./target/debug/forge -v <file.fr>

# Run the Rust interpreter on a .fr file
./target/debug/forge -r <file.fr>

# Run with native ARM64 compilation
./target/debug/forge --native <file.fr>

# Run via ARM64 VM (legacy flag)
./target/debug/forge --asm <file.fr>

# Dump syntax mapping
./target/debug/forge --out

# Reload syntax mapping and rebuild
./target/debug/forge --in

# Install to /usr/local/bin (requires sudo)
sudo ./target/release/forge --install

# Clean generated artifacts
./target/debug/forge --clean
```

## Project Structure

```
src/
  main.rs          - CLI entry point (clap-based)
  lib.rs           - Library re-exports
  lexer.rs         - Logos-based tokenizer
  parser.rs        - Recursive descent parser
  ast.rs           - AST node definitions
  ir.rs            - IR builder (3-address code)
  analyzer.rs      - Static analysis (dead code, types, resource leaks)
  diagnostics.rs   - Colored error/warning rendering
  metroman.rs      - Plugin manager binary
  codegen/
    mod.rs
    arm64.rs       - ARM64 native code generator
    direct.rs      - IR interpreter (activated with -r)
  runtime/
    mod.rs
    collections.rs - DynamicList (heap-allocated)
    gc.rs          - ResourceGC (file handle tracking)
    async_loop.rs  - Tokio-based EventLoop
  meta/
    mod.rs
    syntax_dump.rs - Dump syntax mapping to syntax.fr
    syntax_load.rs - Reload syntax.fr and rebuild compiler
```

## Code Style

- Rust 2021 edition
- Use `#[allow(dead_code)]` for intentionally unused public API items
- Prefix unused variables with `_` (e.g. `_err_var`)
- All IR instructions use named struct fields (e.g. `{ dest, left, right }`)
- ARM64 codegen emits macOS Apple Silicon assembly (`.s` files)

## Key AST / IR Notes

- `Expr::Ask(Box<Expr>)` — prompt is an expression, not a raw string
- `Expr::Bool(bool)` — boolean literals (`true`/`false`)
- `Expr::Neg(Box<Expr>)` — unary negation
- `Expr::Await(Box<Expr>)` — async await expression
- `IrInstr::Input { dest, prompt }` — `prompt` is a temp variable name (String)
- Numbers are `f64` throughout (both `Number` and `Float` AST variants map to `IrValue::Number`)

## Known TODO (from TODO.md)

- [ ] Async/Await native ARM64 runtime (currently stubbed)
- [ ] File I/O syscall implementation in ARM64 codegen
- [ ] try/catch stack unwinding in codegen
- [ ] Standard library (string manipulation, math builtins)
- [ ] x86_64 codegen port
- [ ] Optimization pass (constant folding, DCE)
