#!/bin/bash

echo "Starting CoRe Language WebAssembly IDE..."

if [ ! -d "pkg" ]; then
    echo "Building WebAssembly module..."
    wasm-pack build --target web --out-dir pkg
fi

echo "Starting HTTP server on http://localhost:8080"
echo "Testing: http://localhost:8080/test.html"
echo "Main IDE: http://localhost:8080"

python3 -m http.server 8080
