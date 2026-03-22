use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    Number(f64),
    Float(f64),
    String(String),
    Identifier(String),

    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),

    BitAnd(Box<Expr>, Box<Expr>),
    BitOr(Box<Expr>, Box<Expr>),
    BitXor(Box<Expr>, Box<Expr>),
    BitNot(Box<Expr>),
    Shl(Box<Expr>, Box<Expr>),
    Shr(Box<Expr>, Box<Expr>),

    Eq(Box<Expr>, Box<Expr>),
    Ne(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
    Le(Box<Expr>, Box<Expr>),
    Ge(Box<Expr>, Box<Expr>),

    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),

    Neg(Box<Expr>),
    Bool(bool),

    Await(Box<Expr>),

    Call(String, Vec<Expr>),

    Ask(Box<Expr>),

    List(Vec<Expr>),

    Range(Box<Expr>, Box<Expr>),

    Index(Box<Expr>, Box<Expr>),

    Member(Box<Expr>, String),

    MethodCall(Box<Expr>, String, Vec<Expr>),

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
    VarDecl(String, Expr),

    Assign(Expr, Expr),

    Say(Expr),

    Return(Expr),

    Expr(Expr),

    Block(Vec<Stmt>),
    Unsafe(Vec<Stmt>),

    If(Expr, Vec<Stmt>, Option<Vec<Stmt>>),

    While(Expr, Vec<Stmt>),

    For(String, Expr, Vec<Stmt>),

    Import(String),

    TryCatch(Vec<Stmt>, String, Vec<Stmt>),

    Throw(Expr),

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
    pub is_unsafe: bool,
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
