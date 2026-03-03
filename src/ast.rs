use serde::{Deserialize, Serialize};

/// Abstract Syntax Tree node definitions for CoRe language
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    // Literals
    Number(f64),
    Float(f64),
    String(String),
    Identifier(String),

    // Binary operations
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),

    // Bitwise
    BitAnd(Box<Expr>, Box<Expr>),
    BitOr(Box<Expr>, Box<Expr>),
    BitXor(Box<Expr>, Box<Expr>),
    BitNot(Box<Expr>),
    Shl(Box<Expr>, Box<Expr>),
    Shr(Box<Expr>, Box<Expr>),

    // Comparisons
    Eq(Box<Expr>, Box<Expr>),
    Ne(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
    Le(Box<Expr>, Box<Expr>),
    Ge(Box<Expr>, Box<Expr>),

    // Logical
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),

    // Unary
    Neg(Box<Expr>),
    Bool(bool),

    // Async
    Await(Box<Expr>),

    // Function call
    Call(String, Vec<Expr>),

    // Input
    Ask(Box<Expr>),

    // List literal
    List(Vec<Expr>),

    // Range: start..end
    Range(Box<Expr>, Box<Expr>),

    // Indexing: expr[index]
    Index(Box<Expr>, Box<Expr>),

    // Member access: expr.member
    Member(Box<Expr>, String),

    // Method call: expr.method: args...
    MethodCall(Box<Expr>, String, Vec<Expr>),

    // Map literal: { key: value, ... }
    Map(Vec<(Expr, Expr)>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraitDef {
    pub name: String,
    pub methods: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImplBlock {
    pub trait_name: String,
    pub type_name: String,
    pub methods: Vec<FnDef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Stmt {
    // Variable declaration: var x: expr
    VarDecl(String, Expr),

    // Assignment: target = expr
    Assign(Expr, Expr),

    // Print statement: say: expr
    Say(Expr),

    // Return statement
    Return(Expr),

    // Expression statement
    Expr(Expr),

    // Block
    Block(Vec<Stmt>),

    // If statement: if condition { then_block } else { else_block }
    If(Expr, Vec<Stmt>, Option<Vec<Stmt>>),

    // While loop: while condition { body }
    While(Expr, Vec<Stmt>),

    // For loop: for var in start..end { body }
    For(String, Expr, Vec<Stmt>),

    // Import statement: import "filename"
    Import(String),

    // Try/Catch
    TryCatch(Vec<Stmt>, String, Vec<Stmt>),

    // Throw
    Throw(Expr),

    // Struct definition (within statement context or top level)
    Struct(StructDef),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FnType {
    Normal,
    Global,      // fng
    Precompiled, // fnc
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub is_async: bool,
    pub fn_type: FnType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TopLevel {
    Function(FnDef),
    Struct(StructDef),
    Trait(TraitDef),
    Impl(ImplBlock),
    Statement(Stmt),
    Import(String),
    Use(String),      // use filename.fr
    Hardwire(String), // upd filename.frx
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Program {
    pub items: Vec<TopLevel>,
}

impl Program {
    pub fn new() -> Self {
        Program { items: Vec::new() }
    }
}
