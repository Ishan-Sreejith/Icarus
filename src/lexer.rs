use logos::Logos;

/// Tokens for the CoRe language
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\n\f]+")]
pub enum Token {
    // Comments
    #[regex(r"//[^\n]*", logos::skip)]
    #[regex(r"(?s)/\*(?:[^*]|\*[^/])*\*/", logos::skip)]
    #[regex(r"#[^\n]*", logos::skip)]
    // Keywords
    #[token("fn")]
    Fn,

    #[token("var")]
    Var,

    #[token("return")]
    Return,

    #[token("async")]
    Async,

    #[token("mod")]
    Mod,

    #[token("if")]
    If,

    #[token("else")]
    Else,

    #[token("while")]
    While,

    #[token("for")]
    For,

    #[token("trait")]
    Trait,

    #[token("impl")]
    Impl,

    #[token("class")]
    Class,

    #[token("cl")]
    Cl,

    #[token("clg")]
    Clg,

    #[token("clc")]
    Clc,

    #[token("struct")]
    Struct,

    #[token("try")]
    Try,

    #[token("catch")]
    Catch,

    #[token("throw")]
    Throw,

    #[token("in")]
    In,

    #[token("true")]
    True,

    #[token("false")]
    False,

    #[token("await")]
    Await,

    #[token("&")]
    BitAnd,

    #[token("|")]
    BitOr,

    #[token("^")]
    BitXor,

    #[token("~")]
    BitNot,

    #[token("<<")]
    Shl,

    #[token(">>")]
    Shr,

    #[token("and")]
    And,

    #[token("or")]
    Or,

    #[token("not")]
    Not,

    #[token("import")]
    Import,

    #[token("say")]
    Say,

    #[token("ask")]
    Ask,

    #[token("fng")]
    Fng,

    #[token("fnc")]
    Fnc,

    #[token("use")]
    Use,

    #[token("upd")]
    Upd,

    #[token("init")]
    Init,

    // Operators
    #[token("+")]
    Plus,

    #[token("-")]
    Minus,

    #[token("*")]
    Star,

    #[token("/")]
    Slash,

    #[token("==")]
    EqEq,

    #[token("!=")]
    Ne,

    #[token("<=")]
    Le,

    #[token(">=")]
    Ge,

    #[token("<")]
    Lt,

    #[token(">")]
    Gt,

    #[token("=")]
    Eq,

    // Delimiters
    #[token(":")]
    Colon,

    #[token("..")]
    DotDot,

    #[token(".")]
    Dot,

    #[token(";")]
    Semicolon,

    #[token(",")]
    Comma,

    #[token("{")]
    LBrace,

    #[token("}")]
    RBrace,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[token("[")]
    LBracket,

    #[token("]")]
    RBracket,

    // Literals
    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        s[1..s.len()-1].to_string()
    })]
    StringLit(String),

    #[regex(r"[0-9]+\.[0-9]+", |lex| lex.slice().parse::<f64>().ok())]
    Float(f64),

    #[regex(r"[0-9]+", |lex| lex.slice().parse::<f64>().ok())]
    Number(f64),

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_-]*", |lex| lex.slice().to_string())]
    Identifier(String),
}

pub struct Lexer<'source> {
    inner: logos::Lexer<'source, Token>,
}

impl<'source> Lexer<'source> {
    pub fn new(source: &'source str) -> Self {
        Lexer {
            inner: Token::lexer(source),
        }
    }
}

impl<'source> Iterator for Lexer<'source> {
    type Item = (Result<Token, String>, std::ops::Range<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            Some(result) => {
                let span = self.inner.span();
                let token =
                    result.map_err(|_| format!("Unexpected token at position {}", span.start));
                Some((token, span))
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_basic() {
        // var x : 10 + 2 => Var, Identifier, Colon, Number, Plus, Number = 6 tokens
        let source = r#"var x: 10 + 2"#;
        let tokens: Vec<_> = Lexer::new(source).collect();
        assert_eq!(tokens.len(), 6);
    }

    #[test]
    fn test_lexer_say() {
        let source = r#"say: "Hello""#;
        let tokens: Vec<_> = Lexer::new(source).collect();
        // tokens is a Vec<(Result<Token, String>, Range<usize>)>
        // We need to match against the first element of the tuple
        assert!(matches!(tokens[0].0, Ok(Token::Say)));
    }

    #[test]
    fn test_lexer_le_ge() {
        let source = "var x: 1 <= 2\nvar y: 3 >= 4";
        let tokens: Vec<_> = Lexer::new(source).map(|(t, _)| t.unwrap()).collect();
        assert!(tokens.contains(&Token::Le));
        assert!(tokens.contains(&Token::Ge));
    }
}
