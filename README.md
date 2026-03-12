# CoRe Language

CoRe is a small programming language project written in Rust. This repository includes the language runtime, a VM, a JIT path, a Rust-based interpreter, and the code for a simple online demonstration running on the V8 engine and WASM.

## What’s here

- `forge` — VM execution
- `fforge` — JIT execution
- `forger` — Rust interpreter
- `main.fr` — main sample program
- `examples/` — additional sample programs
- `src/` — compiler and runtime source

## Build

```bash
cargo build
```

## Run

```bash
./target/debug/fforge main.fr
./target/debug/forge main.fr
./target/debug/forger main.fr
```

## Test

```bash
cargo test
bash test_core_features.sh
```

## Notes

The JIT-oriented path is aimed at ARM64 systems, while the other execution modes are useful for testing and comparison. If you want a quick feature demo, start with `main.fr` or the programs in `examples/`.

## Repository guide

- `Cargo.toml` defines the Rust package and binaries.
- `docs/` contains project notes and archived documentation.
- `test_*.sh` scripts cover various language features.

If you’re trying to understand the project quickly, build it first, run `main.fr`, then open a few files in `examples/` to see how the language is used.
