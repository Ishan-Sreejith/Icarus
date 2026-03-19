# CoRe Language WebAssembly Setup Guide

## 🌟 Overview

This guide walks you through setting up the CoRe Language compiler for WebAssembly, enabling you to run CoRe code directly in web browsers with a complete IDE experience.

## 📋 Prerequisites

### Required Tools

1. **Rust** (with wasm32 target)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup target add wasm32-unknown-unknown
   ```

2. **wasm-pack** (will be auto-installed by build script)
   ```bash
   curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
   ```

3. **Web Server** (for serving files)
   - Python 3: `python3 -m http.server`
   - Node.js: `npx serve`
   - Or any local web server

## 🏗️ Project Structure

```
wasm/
├── Cargo.toml          # WebAssembly-specific dependencies
├── src/
│   └── lib.rs          # Main WASM bindings and interpreter
├── index.html          # Web IDE interface
├── build.sh           # Build script
└── README.md          # This guide
```

## 🚀 Quick Start

### 1. Build the WebAssembly Module

```bash
cd wasm/
./build.sh
```

This will:
- Install `wasm-pack` if needed
- Build the WebAssembly module
- Generate the `pkg/` directory with WASM bindings

### 2. Serve the Web IDE

```bash
# Option 1: Python
python3 -m http.server 8000

# Option 2: Node.js
npx serve .

# Option 3: Any web server pointing to the wasm/ directory
```

### 3. Open in Browser

Navigate to `http://localhost:8000` and start coding!

## 🎮 Using the Web IDE

### Interface Overview

- **Left Panel:**
  - **File Manager:** Create, edit, and delete `.fr` files
  - **Terminal:** Execute CoRe commands (`core`, `fforge`, `metroman`)
  - **Examples:** Quick-start code snippets

- **Right Panel:**
  - **Editor:** Code editor with syntax highlighting
  - **Output:** Program execution results

### Available Commands

#### Core Compiler Commands
```bash
core <file.fr>          # Execute CoRe file
core -r <file.fr>       # Execute with interpreter
core -j <file.fr>       # Execute with JIT (same as interpreter in WASM)
core --out              # Dump syntax mapping to syntax.fr
core --in               # Load custom syntax from syntax.fr
```

#### JIT Compiler
```bash
fforge <file.fr>        # Execute with JIT compiler
```

#### Plugin Manager
```bash
metroman --out          # Create plugin template
metroman --init         # Initialize plugin project
metroman --build        # Build plugin
```

#### File Operations
```bash
ls                      # List files
cat <filename>          # Show file contents
help                    # Show all commands
clear                   # Clear terminal
```

### Example Workflow

1. **Create a new file:**
   - Type filename in "File Manager" input (e.g., `hello.fr`)
   - Click "Create File"

2. **Write CoRe code:**
   ```core
   say: "Hello, WebAssembly!"
   var name: "World"
   say: "Hello, " + name + "!"
   ```

3. **Run the code:**
   - Click "Run" button, or
   - Use terminal: `core hello.fr`

4. **Experiment with syntax:**
   - Terminal: `core --out` (generates `syntax.fr`)
   - Modify `syntax.fr` to customize language syntax
   - Terminal: `core --in` (loads custom syntax)

## 🔧 Advanced Features

### Custom Syntax Support

The Web IDE supports CoRe's syntax customization feature:

```bash
# Generate syntax mapping
core --out

# Edit syntax.fr file to customize keywords, operators, etc.

# Load custom syntax
core --in
```

### Plugin Development

Create and test plugins directly in the browser:

```bash
# Generate plugin template
metroman --out

# Edit the generated plugin_template.fr
# Test plugin functionality
```

### File Persistence

- Files are stored in browser memory during the session
- Download/upload functionality can be added for persistence
- Local Storage integration available for auto-save

## 🏗️ Architecture

### WebAssembly Module (`src/lib.rs`)

The WASM module provides:
- **CoreCompilerWasm:** Main compiler interface
- **WebInterpreter:** Browser-compatible interpreter (no JIT)
- **Command simulation:** Terminal command handling
- **File system emulation:** In-memory file management

### Key Components

1. **Lexer/Parser:** Full CoRe language parsing
2. **IR Generation:** Intermediate representation
3. **Web Interpreter:** Executes IR without native code generation
4. **Command Interface:** Simulates terminal commands
5. **File Management:** In-browser file operations

### Differences from Native

- **No JIT Compilation:** Uses interpreter instead of ARM64 code generation
- **No Native VM:** WebAssembly interpreter replaces native VM
- **Browser File System:** In-memory instead of actual file I/O
- **Command Simulation:** Terminal interface simulated in JavaScript

## 🐛 Troubleshooting

### Build Issues

**Error: `wasm-pack` not found**
```bash
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

**Error: Can't find forge library**
- Ensure you're building from the `wasm/` directory
- Check that the parent directory contains the main Forge project

**Rust compilation errors:**
- Ensure wasm32 target is installed: `rustup target add wasm32-unknown-unknown`
- Check Rust version: `rustc --version` (should be 1.70+)

### Runtime Issues

**WebAssembly module fails to load:**
- Check browser console for errors
- Ensure files are served via HTTP (not file://)
- Verify MIME types are configured for .wasm files

**Commands not working:**
- Check browser console for JavaScript errors
- Ensure WebAssembly module loaded successfully
- Verify command syntax matches help output

**File operations failing:**
- Files are stored in memory only during session
- Check for JavaScript errors in console
- Ensure valid `.fr` filenames

## 🚢 Deployment

### GitHub Pages

1. **Build the project:**
   ```bash
   cd wasm/
   ./build.sh
   ```

2. **Commit to repository:**
   ```bash
   git add .
   git commit -m "Add WebAssembly build"
   git push origin main
   ```

3. **Configure GitHub Pages:**
   - Go to repository Settings
   - Enable GitHub Pages from `/wasm` directory
   - Access at `https://username.github.io/repository-name`

### Custom Hosting

1. Upload the entire `wasm/` directory to your web server
2. Ensure proper MIME types for `.wasm` files
3. Configure CORS headers if needed

### CDN Deployment

For faster loading, consider hosting WASM files on a CDN:
```javascript
import init from 'https://cdn.example.com/core_wasm.js';
```

## 🎯 Features Implemented

### ✅ Core Language Features
- [x] Variables and basic data types
- [x] Functions and function calls
- [x] Classes and object instantiation
- [x] Collections (lists, maps)
- [x] Control flow (if/else, loops)
- [x] String manipulation
- [x] Arithmetic operations

### ✅ Development Tools
- [x] Interactive web IDE
- [x] File management
- [x] Terminal simulation
- [x] Syntax customization (`core --out`, `core --in`)
- [x] Plugin templates (`metroman --out`)
- [x] Example code library
- [x] Real-time execution

### ✅ WebAssembly Integration
- [x] Full compiler pipeline in WASM
- [x] Browser-compatible interpreter
- [x] JavaScript bindings
- [x] Memory-efficient operation
- [x] Error handling and reporting

## 🎨 Customization

### Styling

Modify `index.html` CSS to customize the IDE appearance:
- Color schemes
- Font families
- Layout adjustments
- Additional UI elements

### Functionality

Extend `src/lib.rs` to add:
- Additional commands
- New language features
- Enhanced file operations
- Integration with external APIs

### Examples

Add more example code snippets by modifying the `examples` object in `index.html`.

## 📚 Learning Resources

### CoRe Language Documentation
- See main project documentation for language syntax
- Review examples in the Web IDE
- Experiment with syntax customization

### WebAssembly Resources
- [WebAssembly.org](https://webassembly.org/)
- [wasm-bindgen Guide](https://rustwasm.github.io/wasm-bindgen/)
- [Rust and WebAssembly](https://rustwasm.github.io/docs/book/)

## 🤝 Contributing

1. Fork the repository
2. Create your feature branch
3. Test in both native and WebAssembly environments
4. Submit a pull request

## 📞 Support

- **Issues:** GitHub repository issues section
- **Documentation:** Main project README
- **Examples:** Web IDE example library

---

**Ready to start coding in CoRe Language on the web? Run `./build.sh` and open your browser!** 🚀
