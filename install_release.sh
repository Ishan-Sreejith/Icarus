#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$SCRIPT_DIR"

echo "Building release binaries..."
cargo build --release

echo "Running forge installer..."
"$SCRIPT_DIR/target/release/forge" --install

ZSHRC="${HOME}/.zshrc"
PATH_LINE='export PATH="$HOME/.local/bin:$PATH"'

if [ ! -f "$ZSHRC" ]; then
    touch "$ZSHRC"
fi

if ! grep -Fqs "$PATH_LINE" "$ZSHRC"; then
    {
        printf '\n'
        printf '# CoRe / forge local binaries\n'
        printf '%s\n' "$PATH_LINE"
    } >> "$ZSHRC"
    echo "Added ~/.local/bin to $ZSHRC"
else
    echo "~/.local/bin is already in $ZSHRC"
fi

echo "Reload your shell with:"
echo "  source \"$ZSHRC\""
