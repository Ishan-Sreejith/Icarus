use crate::ast::*;
use crate::lexer::Token;

pub struct Parser {
    tokens: Vec<(Token, std::ops::Range<usize>)>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<(Token, std::ops::Range<usize>)>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|(token, _)| token)
    }

    fn advance(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let (token, _) = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(token)
        } else {
            None
        }
    }

    fn error<T>(&self, msg: String) -> Result<T, String> {
        if self.pos < self.tokens.len() {
            let range = &self.tokens[self.pos].1;
            Err(format!("{} at byte {}", msg, range.start))
        } else {
            Err(format!("{} at EOF", msg))
        }
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        match self.current() {
            Some(token) if std::mem::discriminant(token) == std::mem::discriminant(&expected) => {
                self.advance();
                Ok(())
            }
            Some(token) => self.error(format!("Expected {:?}, found {:?}", expected, token)),
            None => Err(format!("Expected {:?}, found EOF", expected)),
        }
    }

    pub fn parse(&mut self) -> Result<Program, String> {
        let mut program = Program::new();

        while self.current().is_some() {
            program.items.push(self.parse_top_level()?);
        }

        Ok(program)
    }

    fn parse_top_level(&mut self) -> Result<TopLevel, String> {
        match self.current() {
            Some(Token::Fn) | Some(Token::Async) => {
                Ok(TopLevel::Function(self.parse_function(FnType::Normal)?))
            }
            Some(Token::Fng) => Ok(TopLevel::Function(self.parse_function(FnType::Global)?)),
            Some(Token::Fnc) => Ok(TopLevel::Function(
                self.parse_function(FnType::Precompiled)?,
            )),
            Some(Token::Struct)
            | Some(Token::Class)
            | Some(Token::Cl)
            | Some(Token::Clg)
            | Some(Token::Clc) => {
                Ok(TopLevel::Struct(self.parse_struct_def()?))
            }
            Some(Token::Trait) => Ok(TopLevel::Trait(self.parse_trait_def()?)),
            Some(Token::Impl) => Ok(TopLevel::Impl(self.parse_impl_block()?)),
            Some(Token::Import) => {
                self.advance();
                match self.advance() {
                    Some(Token::StringLit(s)) => Ok(TopLevel::Import(s)),
                    _ => self.error("Expected string literal after import".to_string()),
                }
            }
            Some(Token::Use) => {
                self.advance();
                match self.advance() {
                    Some(Token::Identifier(s)) => Ok(TopLevel::Use(format!("{}.fr", s))),
                    _ => self.error("Expected filename after use".to_string()),
                }
            }
            Some(Token::Upd) => {
                self.advance();
                match self.advance() {
                    Some(Token::Identifier(s)) => Ok(TopLevel::Hardwire(format!("{}.frx", s))),
                    _ => self.error("Expected filename after upd".to_string()),
                }
            }
            _ => Ok(TopLevel::Statement(self.parse_statement()?)),
        }
    }

    fn parse_function(&mut self, fn_type: FnType) -> Result<FnDef, String> {
        let is_async = if matches!(self.current(), Some(Token::Async)) {
            self.advance();
            true
        } else {
            false
        };

        match fn_type {
            FnType::Global => self.expect(Token::Fng)?,
            FnType::Precompiled => self.expect(Token::Fnc)?,
            FnType::Normal => self.expect(Token::Fn)?,
        }

        let name = match self.advance() {
            Some(Token::Identifier(s)) => s,
            _ => return Err("Expected function name".to_string()),
        };

        // Parse parameters - colon is optional if no parameters
        let mut params = Vec::new();

        // Check if next token is LBrace (no params)
        if matches!(self.current(), Some(Token::LBrace)) {
            // No parameters, no colon needed
        } else {
            // Parameters present, expect colon
            self.expect(Token::Colon)?;

            loop {
                match self.current() {
                    Some(Token::Identifier(s)) => {
                        params.push(s.clone());
                        self.advance();

                        if matches!(self.current(), Some(Token::Comma)) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    _ => break,
                }
            }
        }

        self.expect(Token::LBrace)?;

        let mut body = Vec::new();
        while !matches!(self.current(), Some(Token::RBrace)) {
            body.push(self.parse_statement()?);
        }

        self.expect(Token::RBrace)?;

        Ok(FnDef {
            name,
            params,
            body,
            is_async,
            fn_type,
        })
    }

    fn parse_statement(&mut self) -> Result<Stmt, String> {
        match self.current() {
            Some(Token::Var) => self.parse_var_decl(),
            Some(Token::Say) => self.parse_say(),
            Some(Token::Return) => self.parse_return(),
            Some(Token::LBrace) => self.parse_block(),
            Some(Token::If) => self.parse_if(),
            Some(Token::While) => self.parse_while(),
            Some(Token::For) => self.parse_for(),
            Some(Token::Struct)
            | Some(Token::Class)
            | Some(Token::Cl)
            | Some(Token::Clg)
            | Some(Token::Clc) => Ok(Stmt::Struct(self.parse_struct_def()?)),
            Some(Token::Try) => self.parse_try_catch(),
            Some(Token::Throw) => self.parse_throw(),
            Some(Token::Import) => {
                self.advance();
                match self.advance() {
                    Some(Token::StringLit(s)) => Ok(Stmt::Import(s)),
                    _ => self.error("Expected string literal after import".to_string()),
                }
            }
            Some(Token::Fn) | Some(Token::Async) => {
                // Nested function definition
                let func = self.parse_function(FnType::Normal)?;
                Ok(Stmt::Expr(Expr::Identifier(func.name)))
            }
            _ => {
                let expr = self.parse_expression()?;
                if matches!(self.current(), Some(Token::Eq)) {
                    self.advance();
                    let value = self.parse_expression()?;
                    Ok(Stmt::Assign(expr, value))
                } else {
                    Ok(Stmt::Expr(expr))
                }
            }
        }
    }

    fn parse_var_decl(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Var)?;

        let name = match self.advance() {
            Some(Token::Identifier(s)) => s,
            _ => return Err("Expected variable name".to_string()),
        };

        // Parse expression - colon is optional for arrays
        let expr = if matches!(self.current(), Some(Token::LBrace)) {
            // Array declaration without colon
            self.parse_expression()?
        } else {
            // Regular declaration with colon
            self.expect(Token::Colon)?;
            self.parse_expression()?
        };

        Ok(Stmt::VarDecl(name, expr))
    }

    fn parse_say(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Say)?;
        // say: "hello"
        // The colon is separate from the Say token in lexer
        // So we need to expect the colon after Say
        self.expect(Token::Colon)?;

        let expr = self.parse_expression()?;
        Ok(Stmt::Say(expr))
    }

    fn parse_return(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Return)?;
        let expr = self.parse_expression()?;
        Ok(Stmt::Return(expr))
    }

    fn parse_block(&mut self) -> Result<Stmt, String> {
        self.expect(Token::LBrace)?;

        let mut stmts = Vec::new();
        while !matches!(self.current(), Some(Token::RBrace)) {
            stmts.push(self.parse_statement()?);
        }

        self.expect(Token::RBrace)?;
        Ok(Stmt::Block(stmts))
    }

    fn parse_if(&mut self) -> Result<Stmt, String> {
        self.expect(Token::If)?;

        let condition = self.parse_expression()?;

        self.expect(Token::LBrace)?;
        let mut then_block = Vec::new();
        while !matches!(self.current(), Some(Token::RBrace)) {
            then_block.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;

        let else_block = if matches!(self.current(), Some(Token::Else)) {
            self.advance();
            self.expect(Token::LBrace)?;
            let mut stmts = Vec::new();
            while !matches!(self.current(), Some(Token::RBrace)) {
                stmts.push(self.parse_statement()?);
            }
            self.expect(Token::RBrace)?;
            Some(stmts)
        } else {
            None
        };

        Ok(Stmt::If(condition, then_block, else_block))
    }

    fn parse_while(&mut self) -> Result<Stmt, String> {
        self.expect(Token::While)?;

        let condition = self.parse_expression()?;

        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while !matches!(self.current(), Some(Token::RBrace)) {
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;

        Ok(Stmt::While(condition, body))
    }

    fn parse_for(&mut self) -> Result<Stmt, String> {
        self.expect(Token::For)?;

        let var_name = match self.advance() {
            Some(Token::Identifier(s)) => s,
            _ => return Err("Expected variable name after for".to_string()),
        };

        self.expect(Token::In)?;

        let iterable = self.parse_expression()?;

        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while !matches!(self.current(), Some(Token::RBrace)) {
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;

        Ok(Stmt::For(var_name, iterable, body))
    }

    fn parse_struct_def(&mut self) -> Result<StructDef, String> {
        match self.current() {
            Some(Token::Class)
            | Some(Token::Cl)
            | Some(Token::Clg)
            | Some(Token::Clc) => {
                self.advance();
            }
            _ => self.expect(Token::Struct)?,
        }

        let name = match self.advance() {
            Some(Token::Identifier(s)) => s,
            _ => return Err("Expected struct name".to_string()),
        };

        self.expect(Token::LBrace)?;

        let mut fields = Vec::new();
        while !matches!(self.current(), Some(Token::RBrace)) {
            match self.advance() {
                Some(Token::Identifier(f)) => {
                    fields.push(f);
                    if matches!(self.current(), Some(Token::Comma)) {
                        self.advance();
                    }
                }
                _ => break,
            }
        }

        self.expect(Token::RBrace)?;
        Ok(StructDef { name, fields })
    }

    fn parse_try_catch(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Try)?;
        self.expect(Token::LBrace)?;

        let mut try_body = Vec::new();
        while !matches!(self.current(), Some(Token::RBrace)) {
            try_body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;

        self.expect(Token::Catch)?;

        let error_var = if matches!(self.current(), Some(Token::Identifier(_))) {
            match self.advance() {
                Some(Token::Identifier(s)) => s,
                _ => "err".to_string(),
            }
        } else {
            "err".to_string()
        };

        self.expect(Token::LBrace)?;
        let mut catch_body = Vec::new();
        while !matches!(self.current(), Some(Token::RBrace)) {
            catch_body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;

        Ok(Stmt::TryCatch(try_body, error_var, catch_body))
    }

    fn parse_throw(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Throw)?;
        let expr = self.parse_expression()?;
        Ok(Stmt::Throw(expr))
    }

    fn parse_trait_def(&mut self) -> Result<TraitDef, String> {
        self.expect(Token::Trait)?;

        let name = match self.advance() {
            Some(Token::Identifier(s)) => s,
            _ => return Err("Expected trait name".to_string()),
        };

        self.expect(Token::LBrace)?;

        let mut methods = Vec::new();
        while !matches!(self.current(), Some(Token::RBrace)) {
            self.expect(Token::Fn)?;
            let method_name = match self.advance() {
                Some(Token::Identifier(s)) => s,
                _ => return Err("Expected method name in trait".to_string()),
            };

            // Optional params (ignored for now): ": a, b"
            if matches!(self.current(), Some(Token::Colon)) {
                self.advance();
                while matches!(self.current(), Some(Token::Identifier(_))) {
                    self.advance();
                    if matches!(self.current(), Some(Token::Comma)) {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }

            // Require semicolon between method signatures.
            self.expect(Token::Semicolon)?;
            methods.push(method_name);
        }

        self.expect(Token::RBrace)?;
        Ok(TraitDef { name, methods })
    }

    fn parse_impl_block(&mut self) -> Result<ImplBlock, String> {
        self.expect(Token::Impl)?;

        let trait_name = match self.advance() {
            Some(Token::Identifier(s)) => s,
            _ => return Err("Expected trait name after impl".to_string()),
        };

        // Syntax: impl Trait for Type { ... }
        self.expect(Token::For)?;
        let type_name = match self.advance() {
            Some(Token::Identifier(s)) => s,
            _ => return Err("Expected type name after 'for'".to_string()),
        };

        self.expect(Token::LBrace)?;
        let mut methods = Vec::new();
        while !matches!(self.current(), Some(Token::RBrace)) {
            match self.current() {
                Some(Token::Fn) | Some(Token::Async) => {
                    methods.push(self.parse_function(FnType::Normal)?);
                }
                _ => {
                    return Err(
                        "Only function definitions are allowed inside impl blocks".to_string()
                    )
                }
            }
        }
        self.expect(Token::RBrace)?;

        Ok(ImplBlock {
            trait_name,
            type_name,
            methods,
        })
    }

    fn parse_expression(&mut self) -> Result<Expr, String> {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_logical_and()?;

        while matches!(self.current(), Some(Token::Or)) {
            self.advance();
            let right = self.parse_logical_and()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_or()?;

        while matches!(self.current(), Some(Token::And)) {
            self.advance();
            let right = self.parse_bitwise_or()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    fn parse_bitwise_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_xor()?;
        while matches!(self.current(), Some(Token::BitOr)) {
            self.advance();
            let right = self.parse_bitwise_xor()?;
            left = Expr::BitOr(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_bitwise_xor(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_and()?;
        while matches!(self.current(), Some(Token::BitXor)) {
            self.advance();
            let right = self.parse_bitwise_and()?;
            left = Expr::BitXor(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_bitwise_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_range()?;
        while matches!(self.current(), Some(Token::BitAnd)) {
            self.advance();
            let right = self.parse_range()?;
            left = Expr::BitAnd(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_range(&mut self) -> Result<Expr, String> {
        let left = self.parse_comparison()?;

        if matches!(self.current(), Some(Token::DotDot)) {
            self.advance();
            let right = self.parse_comparison()?;
            Ok(Expr::Range(Box::new(left), Box::new(right)))
        } else {
            Ok(left)
        }
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        if matches!(self.current(), Some(Token::Not)) {
            self.advance();
            let expr = self.parse_comparison()?;
            return Ok(Expr::Not(Box::new(expr)));
        }

        let mut left = self.parse_additive()?;

        loop {
            match self.current() {
                Some(Token::EqEq) => {
                    self.advance();
                    let right = self.parse_additive()?;
                    left = Expr::Eq(Box::new(left), Box::new(right));
                }
                Some(Token::Ne) => {
                    self.advance();
                    let right = self.parse_additive()?;
                    left = Expr::Ne(Box::new(left), Box::new(right));
                }
                Some(Token::Lt) => {
                    self.advance();
                    let right = self.parse_additive()?;
                    left = Expr::Lt(Box::new(left), Box::new(right));
                }
                Some(Token::Gt) => {
                    self.advance();
                    let right = self.parse_additive()?;
                    left = Expr::Gt(Box::new(left), Box::new(right));
                }
                Some(Token::Le) => {
                    self.advance();
                    let right = self.parse_additive()?;
                    left = Expr::Le(Box::new(left), Box::new(right));
                }
                Some(Token::Ge) => {
                    self.advance();
                    let right = self.parse_additive()?;
                    left = Expr::Ge(Box::new(left), Box::new(right));
                }
                _ => break,
            }
        }

        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_shift()?;

        loop {
            match self.current() {
                Some(Token::Plus) => {
                    self.advance();
                    let right = self.parse_shift()?;
                    left = Expr::Add(Box::new(left), Box::new(right));
                }
                Some(Token::Minus) => {
                    self.advance();
                    let right = self.parse_shift()?;
                    left = Expr::Sub(Box::new(left), Box::new(right));
                }
                _ => break,
            }
        }

        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative()?;
        loop {
            match self.current() {
                Some(Token::Shl) => {
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    left = Expr::Shl(Box::new(left), Box::new(right));
                }
                Some(Token::Shr) => {
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    left = Expr::Shr(Box::new(left), Box::new(right));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_postfix()?;

        loop {
            match self.current() {
                Some(Token::Star) => {
                    self.advance();
                    let right = self.parse_postfix()?;
                    left = Expr::Mul(Box::new(left), Box::new(right));
                }
                Some(Token::Slash) => {
                    self.advance();
                    let right = self.parse_postfix()?;
                    left = Expr::Div(Box::new(left), Box::new(right));
                }
                _ => break,
            }
        }

        Ok(left)
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.current() {
                Some(Token::LBracket) => {
                    self.advance();
                    let index = self.parse_expression()?;
                    self.expect(Token::RBracket)?;
                    expr = Expr::Index(Box::new(expr), Box::new(index));
                }
                Some(Token::Dot) => {
                    self.advance();
                    match self.advance() {
                        Some(Token::Identifier(member)) => {
                            // Check if this is a function call (next token is colon)
                            if matches!(self.current(), Some(Token::Colon)) {
                                self.advance();
                                let mut args = Vec::new();

                                loop {
                                    if matches!(
                                        self.current(),
                                        None | Some(Token::RBrace)
                                            | Some(Token::Var)
                                            | Some(Token::Say)
                                            | Some(Token::Return)
                                            | Some(Token::Fn)
                                    ) {
                                        break;
                                    }

                                    args.push(self.parse_expression()?);

                                    if matches!(self.current(), Some(Token::Comma)) {
                                        self.advance();
                                    } else {
                                        break;
                                    }
                                }

                                expr = Expr::MethodCall(Box::new(expr), member, args);
                            } else {
                                // For now, treat as member access
                                // We'll need to handle function calls without parameters differently
                                expr = Expr::Member(Box::new(expr), member);
                            }
                        }
                        _ => return self.error("Expected member name after .".to_string()),
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.current() {
            Some(Token::Number(n)) => {
                let num = *n;
                self.advance();
                Ok(Expr::Number(num))
            }
            Some(Token::StringLit(s)) => {
                let string = s.clone();
                self.advance();
                Ok(Expr::String(string))
            }
            Some(Token::True) => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            Some(Token::False) => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            Some(Token::Identifier(name)) => {
                let id = name.clone();
                self.advance();

                // Check if this is a function call
                if matches!(self.current(), Some(Token::Colon)) {
                    self.advance();
                    let mut args = Vec::new();

                    loop {
                        if matches!(
                            self.current(),
                            None | Some(Token::RBrace)
                                | Some(Token::Var)
                                | Some(Token::Say)
                                | Some(Token::Return)
                                | Some(Token::Fn)
                        ) {
                            break;
                        }

                        args.push(self.parse_expression()?);

                        if matches!(self.current(), Some(Token::Comma)) {
                            self.advance();
                        } else {
                            break;
                        }
                    }

                    Ok(Expr::Call(id, args))
                } else {
                    Ok(Expr::Identifier(id))
                }
            }
            Some(Token::LParen) => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            Some(Token::LBracket) => {
                self.advance();
                let mut elements = Vec::new();

                while !matches!(self.current(), Some(Token::RBracket)) {
                    elements.push(self.parse_expression()?);

                    if matches!(self.current(), Some(Token::Comma)) {
                        self.advance();
                    } else {
                        break;
                    }
                }

                self.expect(Token::RBracket)?;
                Ok(Expr::List(elements))
            }
            Some(Token::LBrace) => {
                self.advance();
                let mut pairs = Vec::new();

                while !matches!(self.current(), Some(Token::RBrace)) {
                    let key = self.parse_expression()?;
                    self.expect(Token::Colon)?;
                    let value = self.parse_expression()?;
                    pairs.push((key, value));

                    if matches!(self.current(), Some(Token::Comma)) {
                        self.advance();
                    } else {
                        break;
                    }
                }

                self.expect(Token::RBrace)?;
                Ok(Expr::Map(pairs))
            }
            Some(Token::Await) => {
                self.advance();
                let expr = self.parse_primary()?;
                Ok(Expr::Await(Box::new(expr)))
            }
            Some(Token::BitNot) => {
                self.advance();
                let expr = self.parse_primary()?;
                Ok(Expr::BitNot(Box::new(expr)))
            }
            Some(Token::Minus) => {
                self.advance();
                let expr = self.parse_primary()?;
                Ok(Expr::Neg(Box::new(expr)))
            }
            Some(Token::Ask) => {
                self.advance();
                self.expect(Token::Colon)?;
                let prompt = self.parse_expression()?;
                Ok(Expr::Ask(Box::new(prompt)))
            }
            _ => self.error(format!(
                "Unexpected token in expression: {:?}",
                self.current()
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_source(source: &str) -> Result<Program, String> {
        use logos::Logos; // Import trait to access span()

        let mut tokens = Vec::new();
        let mut lex = Token::lexer(source);

        while let Some(token) = lex.next() {
            let span = lex.span();
            match token {
                Ok(t) => tokens.push((t, span)),
                Err(_) => return Err(format!("Lexer error at {:?}", span)),
            }
        }

        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    #[test]
    fn test_parse_var() {
        let source = "var x: 10 + 2";
        let program = parse_source(source).unwrap();
        assert_eq!(program.items.len(), 1);
    }

    #[test]
    fn test_parse_function() {
        let source = r#"fn area: w, h { return w * h }"#;
        let program = parse_source(source).unwrap();
        assert_eq!(program.items.len(), 1);
    }

    #[test]
    fn test_parse_le_ge() {
        let source = "var a: 1 <= 2\nvar b: 3 >= 4";
        let program = parse_source(source).unwrap();
        assert_eq!(program.items.len(), 2);

        match &program.items[0] {
            TopLevel::Statement(Stmt::VarDecl(_, Expr::Le(_, _))) => {}
            other => panic!("unexpected AST for <=: {:?}", other),
        }
        match &program.items[1] {
            TopLevel::Statement(Stmt::VarDecl(_, Expr::Ge(_, _))) => {}
            other => panic!("unexpected AST for >=: {:?}", other),
        }
    }

    #[test]
    fn test_parse_try_catch_without_var() {
        let source = r#"
try { say: "x" } catch { say: "y" }
"#;
        let program = parse_source(source).unwrap();
        assert_eq!(program.items.len(), 1);
    }

    #[test]
    fn test_parse_throw() {
        let source = r#"
try { throw 123 } catch err { say: err }
"#;
        let program = parse_source(source).unwrap();
        assert_eq!(program.items.len(), 1);
    }

    #[test]
    fn test_parse_trait_impl_and_method_call() {
        let source = r#"
trait Printable { fn print: self; }
class Point { x, y }
impl Printable for Point {
  fn print: self { say: self.x }
}
var p: Point
p.print:
"#;
        let program = parse_source(source).unwrap();
        assert!(program.items.len() >= 4);
    }
}
