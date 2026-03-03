#!/bin/bash
#  Installation script for CoRe language binaries

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RELEASE_DIR="$SCRIPT_DIR/target/release"
INSTALL_DIR="${HOME}/.local/bin"

echo "→ Installing CoRe language binaries to $INSTALL_DIR..."
echo ""

# Create install directory
mkdir -p "$INSTALL_DIR"

# List of binaries to install
BINARIES=("forge" "core" "fforge" "forger" "metroman")

# Check if binaries exist
for bin in "${BINARIES[@]}"; do
    if [ ! -f "$RELEASE_DIR/$bin" ]; then
        echo "⚠ Binary '$bin' not found in $RELEASE_DIR"
        echo "  Run 'cargo build --release' first"
        exit 1
    fi
done

# Copy and make executable
for bin in "${BINARIES[@]}"; do
    cp "$RELEASE_DIR/$bin" "$INSTALL_DIR/$bin"
    chmod +x "$INSTALL_DIR/$bin"
    echo "✓ Installed $bin"
done

echo ""
echo "✓ Installation complete!"
echo ""
echo "Add this to your ~/.zshrc if not already present:"
echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
echo ""
echo "Then run: source ~/.zshrc"
echo ""
echo "Available commands:"
echo "  core main.fr        # VM (default)"
echo "  core -r main.fr     # Rust interpreter"
echo "  core -a main.fr     # Assembly VM"
echo "  fforge main.fr      # JIT compiler"
echo "  forge main.fr       # Alias for core (VM)"
echo "  forge --native main.fr  # AOT compiler"
echo "  forger main.fr      # Rust interpreter"
echo "  metroman <command>  # Plugin manager"
echo ""

