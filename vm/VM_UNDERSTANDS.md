# What The VM Understands

There are two different “VM inputs” in this repo:

1) **Forge language** (`.fr`) compiled to ARM64 assembly (`.s`) and executed by `arm64vm` (this is what `forge` runs by default).
2) A simplified, text-based **CoRe VM IR** (`.cir`) that `arm64vm` can run directly.

## 1) Forge (`.fr`) → ARM64 (`.s`) → `arm64vm`

### Run commands

```bash
# VM mode (default)
forge main.fr
forge -v main.fr

# Rust interpreter mode (no VM)
forge -r main.fr

# Native ARM64 build/run (no VM)
forge --native --build main.fr
```

### Language features the VM path supports

The VM path supports whatever `forge` can compile into its ARM64 backend, including:

- Variables (`var x: ...`)
- Functions (`fn name: ... { ... }`)
- `if/else`, `while`
- `for x in list { ... }` and `for k in map { ... }` (map iterates keys)
- Lists (`[1,2,3]`) + maps (`{ "k": v }`) with indexing
- `struct` / `class` (alias) fields via `obj.field`
- `trait` + `impl Trait for Type { ... }` (methods compile to functions like `Type_method`)
- `try { ... } catch e { ... }` and `throw expr`
  - Current limitation: it only catches explicit `throw` within the same function (no cross-call unwinding yet).
- Imports: `import "relative/path"` (see below)
- Type conversion builtins: `str: x`, `num: x`, `bool: x`, `type: x`, plus `is_map/is_list/is_string`

### Async/await note (mode differences)

- `forge -r` implements **cooperative async**:
  - `async fn` calls produce a task thunk
  - `spawn: <async-call>` starts a task
  - `await <task-or-async-call>` waits while letting other tasks run
  - `sleep: ms` yields (timer-based)
- VM/default and `--native` currently run `async fn` synchronously:
  - `await` is effectively a no-op
  - `sleep:` is a no-op

### Imports / plugins

```core
import "lib.fr"
import "myplugin.mtro"
import "plugin"        // extension omitted: tries `.fr`, then `.mtro`
```

- Paths are resolved **relative to the importing file**.
- Cyclic imports are detected and rejected.

## 2) CoRe VM IR (`.cir`) for `arm64vm`

`arm64vm` can also run a simplified IR meant to be easy to write by hand.

See `vm/CORE_IR.md` for the full spec; highlights:

- Two accepted formats:
  - Section mode: `fn <name>.params:` and `fn <name>.actions:`
  - Brace mode: `fn name: { ... }`
- Values: numbers/strings/bools/null, plus lists and maps
- Control flow: labels + `jmp/jz/jnz`
- Builtins via `call`: `len`, `keys`, `values`, `range`

Run it with:

```bash
arm64vm program.cir
arm64vm --sample-ir
```

## 3) ARM64 instruction coverage (assembly mode)

`arm64vm` is an emulator focused on the subset of ARM64 + directives that `forge` emits.
It’s not intended to be a full general-purpose ARM64 system emulator.
