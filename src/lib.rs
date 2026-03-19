pub mod analyzer;
pub mod ast;
pub mod codegen;
pub mod diagnostics;
pub mod ff;
pub mod ir;
pub mod jit;
pub mod training_data;
pub mod lexer;
pub mod meta;
pub mod parser;
pub mod runtime;
pub mod optimizer;

// WebAssembly-specific modules
#[cfg(feature = "wasm")]
pub mod wasm;

// Re-export commonly used types for WebAssembly
#[cfg(feature = "wasm")]
pub use wasm::*;

// Re-export main components for library use
use std::ops::Range;

pub type LexError = String;
pub type ParseError = String;
pub type Ast = ast::Program;

pub use lexer::Token;
pub use ir::{IrInstr, IrProgram};
pub use codegen::direct::{DirectExecutor, Value};
pub use optimizer::{optimize_program, OptimizationStats};

pub fn lex(source: &str) -> Result<Vec<(Token, Range<usize>)>, LexError> {
    lexer::Lexer::new(source)
        .map(|(tok, span)| tok.map(|t| (t, span)))
        .collect()
}

pub fn parse(source: &str) -> Result<Ast, ParseError> {
    let mut visited = std::collections::HashSet::new();
    let preprocessed = ff::preprocess_source(source, None, &mut visited)?;
    let tokens = lex(&preprocessed)?;
    let mut parser = parser::Parser::new(tokens);
    parser.parse()
}

pub fn build_ir(ast: &Ast) -> Result<IrProgram, String> {
    let mut builder = ir::IrBuilder::new();
    builder.build(ast, None)
}
