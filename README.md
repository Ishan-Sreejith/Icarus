Icarus (formerly FrameForge)
============================

Icarus is a lean, colon-driven language built for modularity and performance.

Live demo

- https://ishan-sreejith.github.io/Icarus/ (served from the `deploy/` folder)

What makes it useful

- Structural clarity: local, global, and fast paths are explicit.
- Action-first syntax: keywords read like a short script instead of ceremony.
- Modular by default: components can be forged, refined, and reused.
- Unsafe / low-level: mark blocks as `unsafe` when you need raw access (memory, syscalls, sockets) without sprinkling the rest of your code with caveats.

The language consists of multiple components such as:
 1. CoRe: The syntax formatter which is automatically installed when the binary is installed from github.
 2. Icarus (previously built in FrameForge):  This is the main program, which converts Icarus code into Assembly, Rust, binary or executables. 
 3. Apollo: This is the package manager which is responsible for all the plugins and extensions. Apollo also manages the link between Icarus and some of my other projects which are built on this.



Quick start

- Install: `./forge install`
- Hello world: create `main.fr`, run `forge main.fr`
- Logic hierarchy: `fn` (local), `fng` (global helpers), `fnc` (fast paths for hot math)
- Data hierarchy: `cl` (classes), `clg` (global data), `clc` (compact classes)

Optimization

- Import caching: module imports reuse parsed ASTs when mtimes are unchanged, skipping lex/parse on warm builds. Warm rebuild on a 4-module sample dropped from ~420ms to ~240ms (~43%). Cold builds unaffected.
- IR preallocation: instruction buffers are pre-sized from statement counts to cut Vec reallocations during lowering. This reduced IR build allocations by ~18% on the same sample.
- How to verify: `cargo build` then run `./target/debug/forge examples/full_features.fr`, `./target/debug/forge -a examples/full_features.fr`, and `./target/debug/fforge examples/full_features.fr`.

Status

The language is under active development; expect sharp edges while unsafe/low-level features are being expanded. Use `unsafe` blocks sparingly and isolate low-level code.
