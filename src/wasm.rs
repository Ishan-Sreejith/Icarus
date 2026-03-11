use wasm_bindgen::prelude::*;

// Import the console.log function from the `console` module
#[cfg(feature = "wasm")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// Define a macro for easier console logging
#[cfg(feature = "wasm")]
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

// Import necessary CoRe language components
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::codegen::direct::DirectExecutor;
use crate::ir::IrBuilder;

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub struct CoReEngine {
    executor: DirectExecutor,
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl CoReEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> CoReEngine {
        console_error_panic_hook::set_once();
        console_log!("CoRe Language Engine initialized for WebAssembly");
        
        CoReEngine {
            executor: DirectExecutor::new(),
        }
    }

    #[wasm_bindgen]
    pub fn execute(&mut self, source: &str) -> String {
        match self.execute_internal(source) {
            Ok(result) => result,
            Err(error) => format!("❌ Error: {}", error),
        }
    }

    fn execute_internal(&mut self, source: &str) -> Result<String, String> {
        console_log!("Executing CoRe code: {}", source);

        // Lexical analysis - collect tokens with ranges
        let lexer = Lexer::new(source);
        let mut tokens = Vec::new();
        for (token_result, span) in lexer {
            match token_result {
                Ok(token) => tokens.push((token, span)),
                Err(e) => return Err(format!("Lexer error: {}", e)),
            }
        }
        
        console_log!("Tokens generated: {}", tokens.len());

        // Parsing
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().map_err(|e| format!("Parser error: {:?}", e))?;
        
        console_log!("AST nodes: {}", ast.items.len());

        // IR Generation
        let mut ir_builder = IrBuilder::new();
        let program = ir_builder.build(&ast, None).map_err(|e| format!("IR error: {}", e))?;

        console_log!("Generated IR with {} functions", program.functions.len());

        // Execute with DirectExecutor
        self.executor.execute(&program).map_err(|e| format!("Runtime error: {}", e))?;

        Ok("Program executed successfully".to_string())
    }

    #[wasm_bindgen]
    pub fn get_version(&self) -> String {
        "CoRe Language v1.0 (WebAssembly Edition)".to_string()
    }

    #[wasm_bindgen] 
    pub fn get_features(&self) -> String {
        serde_json::to_string(&vec![
            "Enhanced Collection Display",
            "25+ Built-in Functions", 
            "WebAssembly Support",
            "Browser Execution",
            "Real-time Code Execution"
        ]).unwrap_or_else(|_| "[]".to_string())
    }

    #[wasm_bindgen]
    pub fn reset(&mut self) {
        self.executor = DirectExecutor::new();
        console_log!("CoRe Engine reset");
    }
}

// Utility functions for the web interface
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn format_error(error: &str) -> String {
    format!("🚨 CoRe Error: {}", error)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn get_sample_code() -> String {
    r#"// Welcome to CoRe Language in your browser!

var greeting: "Hello from WebAssembly!"
var numbers: [1, 4, 9, 16, 25]
var info: {
    "language": "CoRe",
    "version": "1.0",
    "platform": "WebAssembly"
}

say: greeting
say: numbers  
say: info

var sum: 0
var i: 0
while i < 3 {
    if i < len(numbers) {
        sum: sum + numbers[i]
        say: "Added: " + str(numbers[i])
        i: i + 1
    }
}

say: "Sum of first 3 numbers: " + str(sum)"#.to_string()
}

#[cfg(feature = "wasm")]
#[wasm_bindgen(start)]
pub fn main() {
    console_log!("🚀 CoRe Language WebAssembly module loaded!");
}
