# CoRe Language Installation Guide

## Quick Install (macOS/Linux)

### 1. Build Release Binaries

```bash
cd "/path/to/CoRe Backup V1.0 copy"
cargo build --release
```

### 2. Install to System PATH

```bash
./target/release/forge --install
```

This installs to `~/.local/bin` by default (no sudo needed).

### 3. Add to PATH

Add this line to your `~/.zshrc` (or `~/.bashrc` for bash):

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Then reload your shell:

```bash
source ~/.zshrc
```

## Verify Installation

```bash
which core
which forge
which fforge
```

Should output paths like `/Users/yourname/.local/bin/core`

## Available Commands

- `core main.fr` - Run with VM (default)
- `core -r main.fr` - Run with Rust interpreter
- `core -a main.fr` - Run with Assembly VM
- `fforge main.fr` - Run with JIT compiler
- `forge --native main.fr` - AOT compile to native binary
- `metroman` - Plugin manager

## Troubleshooting

### Permission Denied

If you get "Permission denied", the binaries might not be executable:

```bash
chmod +x ~/.local/bin/{core,forge,fforge,forger,metroman}
```

### Command Not Found

If commands aren't found after install:

1. Check PATH is set: `echo $PATH | grep .local`
2. Reload shell: `source ~/.zshrc`
3. Verify binaries exist: `ls -la ~/.local/bin/`

### Alternative: Install to /usr/local/bin

If you prefer system-wide install (requires sudo):

```bash
sudo ./target/release/forge --install
```

This will install to `/usr/local/bin` if `~/.local/bin` creation fails.

