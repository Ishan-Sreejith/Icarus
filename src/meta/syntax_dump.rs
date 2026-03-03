use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

#[derive(Debug, Serialize, Deserialize)]
pub struct SyntaxMapping {
    pub keywords: HashMap<String, String>,
    pub operators: HashMap<String, String>,
    pub token_descriptions: HashMap<String, String>,
}

impl SyntaxMapping {
    pub fn from_compiler() -> Self {
        let mut keywords = HashMap::new();
        keywords.insert("fn".to_string(), "fn".to_string());
        keywords.insert("var".to_string(), "var".to_string());
        keywords.insert("return".to_string(), "return".to_string());
        keywords.insert("async".to_string(), "async".to_string());
        keywords.insert("mod".to_string(), "mod".to_string());
        keywords.insert("if".to_string(), "if".to_string());
        keywords.insert("else".to_string(), "else".to_string());
        keywords.insert("while".to_string(), "while".to_string());
        keywords.insert("for".to_string(), "for".to_string());
        keywords.insert("struct".to_string(), "struct".to_string());
        keywords.insert("try".to_string(), "try".to_string());
        keywords.insert("catch".to_string(), "catch".to_string());
        keywords.insert("import".to_string(), "import".to_string());
        keywords.insert("in".to_string(), "in".to_string());
        keywords.insert("true".to_string(), "true".to_string());
        keywords.insert("false".to_string(), "false".to_string());
        keywords.insert("await".to_string(), "await".to_string());
        keywords.insert("and".to_string(), "and".to_string());
        keywords.insert("or".to_string(), "or".to_string());
        keywords.insert("not".to_string(), "not".to_string());
        keywords.insert("say".to_string(), "say".to_string());
        keywords.insert("ask".to_string(), "ask".to_string());
        // Extra keywords used by metroman / extensions
        keywords.insert("fng".to_string(), "fng".to_string());
        keywords.insert("use".to_string(), "use".to_string());
        keywords.insert("upd".to_string(), "upd".to_string());
        keywords.insert("init".to_string(), "init".to_string());

        let mut operators = HashMap::new();
        // Compatibility: older syntax.fr used say:/ask: (the lexer token is `say`/`ask`, colon is separate)
        operators.insert("say:".to_string(), "say:".to_string());
        operators.insert("ask:".to_string(), "ask:".to_string());
        operators.insert("+".to_string(), "+".to_string());
        operators.insert("-".to_string(), "-".to_string());
        operators.insert("*".to_string(), "*".to_string());
        operators.insert("/".to_string(), "/".to_string());
        operators.insert("==".to_string(), "==".to_string());
        operators.insert("!=".to_string(), "!=".to_string());
        operators.insert("<".to_string(), "<".to_string());
        operators.insert(">".to_string(), ">".to_string());
        operators.insert("=".to_string(), "=".to_string());
        operators.insert("&".to_string(), "&".to_string());
        operators.insert("|".to_string(), "|".to_string());
        operators.insert("^".to_string(), "^".to_string());
        operators.insert("~".to_string(), "~".to_string());
        operators.insert("<<".to_string(), "<<".to_string());
        operators.insert(">>".to_string(), ">>".to_string());
        operators.insert(":".to_string(), ":".to_string());
        operators.insert("..".to_string(), "..".to_string());
        operators.insert(".".to_string(), ".".to_string());
        operators.insert(",".to_string(), ",".to_string());
        operators.insert(";".to_string(), ";".to_string());
        operators.insert("{".to_string(), "{".to_string());
        operators.insert("}".to_string(), "}".to_string());
        operators.insert("(".to_string(), "(".to_string());
        operators.insert(")".to_string(), ")".to_string());
        operators.insert("[".to_string(), "[".to_string());
        operators.insert("]".to_string(), "]".to_string());

        let mut token_descriptions = HashMap::new();
        token_descriptions.insert("fn".to_string(), "Function definition keyword".to_string());
        token_descriptions.insert(
            "var".to_string(),
            "Variable declaration keyword".to_string(),
        );
        token_descriptions.insert(
            "say".to_string(),
            "Print statement anchor (used as `say:`)".to_string(),
        );
        token_descriptions.insert(
            "ask".to_string(),
            "Input statement anchor (used as `ask:`)".to_string(),
        );
        token_descriptions.insert("if".to_string(), "Conditional branching".to_string());
        token_descriptions.insert("while".to_string(), "Looping construct".to_string());

        SyntaxMapping {
            keywords,
            operators,
            token_descriptions,
        }
    }

    pub fn dump_to_file(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        let mut file = File::create(path)?;

        writeln!(file, "# CoRe Language Syntax Definition")?;
        writeln!(
            file,
            "# This file defines the token mappings for the CoRe compiler"
        )?;
        writeln!(
            file,
            "# You can modify this file and reload it with: forge --in"
        )?;
        writeln!(
            file,
            "# Note: `say:` and `ask:` are tokenized as `say`/`ask` plus a separate `:` token"
        )?;
        writeln!(file)?;
        writeln!(file, "{}", json)?;

        Ok(())
    }

    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;

        // Skip comment lines
        let json_start = content
            .lines()
            .position(|line| line.trim().starts_with('{'))
            .unwrap_or(0);

        let json_content = content
            .lines()
            .skip(json_start)
            .collect::<Vec<_>>()
            .join("\n");

        let mapping: SyntaxMapping = serde_json::from_str(&json_content)?;
        Ok(mapping)
    }
}
