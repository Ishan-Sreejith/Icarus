# Phase 11: Speculative Optimization & Deoptimization

This phase adds scaffolding for:
- Speculative guards
- Polymorphic inline caching (PIC)
- On-stack replacement (OSR)
- Tiered compilation
- Escape analysis

## What’s Included

- `src/jit/phase11.rs`
  - `TypeTag`, `SpecGuard`, `DeoptAction`
  - `PolymorphicInlineCache` with cache resolution
  - `OsrPlanner` with loop offset tracking
  - `JitProfile` for tiering and hot counters
  - `EscapeAnalysis` stub with test coverage

## Tests

Phase 11 introduces unit tests for:
- Hot counters
- PIC resolution
- OSR loop registration
- Escape analysis detection

Run:

```bash
cargo test --lib jit::phase11
```

## Next Steps (Integration)

- Wire `JitProfile` into JIT compilation to trigger tier switches
- Emit guard instructions (CMP + conditional branch) before fast-path code
- Patch PIC stubs at call sites and recompile hot paths
- Add OSR entry points in loop headers
- Use `EscapeAnalysis` results to decide stack vs heap allocation

