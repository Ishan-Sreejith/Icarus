#!/bin/bash


echo "Setting up CoRe Language for WebAssembly deployment..."

if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: Please run this script from the CoRe language root directory"
    exit 1
fi

if ! command -v rustup &> /dev/null; then
    echo "❌ Rust is not installed. Please install Rust first:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

if ! command -v wasm-pack &> /dev/null; then
    echo "Installing wasm-pack..."
    cargo install wasm-pack
fi

echo "Adding WebAssembly target..."
rustup target add wasm32-unknown-unknown

echo "Building WebAssembly package..."
wasm-pack build --target web --features wasm --no-default-features

if [ $? -eq 0 ]; then
    echo "✅ WebAssembly build successful!"
    echo ""
    echo "📁 Generated files:"
    ls -la pkg/
    echo ""
    echo "To test locally, run:"
    echo "   python3 -m http.server 8000"
    echo "   # Then open http://localhost:8000 in your browser"
    echo ""
    echo "To deploy to GitHub Pages:"
    echo "   1. Create a new repository on GitHub"
    echo "   2. Upload index.html and the pkg/ folder"
    echo "   3. Enable GitHub Pages in repository settings"
    echo ""
    echo "Your CoRe Language WebAssembly edition is ready!"
else
    echo "❌ WebAssembly build failed. Check the error messages above."
    exit 1
fi
