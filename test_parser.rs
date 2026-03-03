use forge::lexer::Token;
use forge::parser::Parser;
use logos::Logos;

fn main() {
    let source = r#"
var app_logic: {
    startup: fn() {
        say: "Application starting..."
    }
}

app_logic.startup
"#;

    let mut tokens = Vec::new();
    let mut lex = Token::lexer(source);

    while let Some(token) = lex.next() {
        let span = lex.span();
        match token {
            Ok(t) => tokens.push((t, span)),
            Err(_) => {
                println!("Lexer error at {:?}", span);
                return;
            }
        }
    }

    let mut parser = Parser::new(tokens);
    match parser.parse() {
        Ok(program) => {
            println!("Parsing successful! Found {} items", program.items.len());
        }
        Err(e) => {
            println!("Parsing failed: {}", e);
        }
    }
}
