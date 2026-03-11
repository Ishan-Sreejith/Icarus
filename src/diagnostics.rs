use colored::Colorize;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Suggestion,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub suggestion: Option<String>,
}

impl Diagnostic {
    pub fn error(msg: &str) -> Self {
        Diagnostic {
            level: DiagnosticLevel::Error,
            message: msg.to_string(),
            line: None,
            column: None,
            suggestion: None,
        }
    }

    #[allow(dead_code)]
    pub fn warning(msg: &str) -> Self {
        Diagnostic {
            level: DiagnosticLevel::Warning,
            message: msg.to_string(),
            line: None,
            column: None,
            suggestion: None,
        }
    }

    #[allow(dead_code)]
    pub fn suggestion(msg: &str, suggestion: &str) -> Self {
        Diagnostic {
            level: DiagnosticLevel::Suggestion,
            message: msg.to_string(),
            line: None,
            column: None,
            suggestion: Some(suggestion.to_string()),
        }
    }

    pub fn at(mut self, line: usize, col: usize) -> Self {
        self.line = Some(line);
        self.column = Some(col);
        self
    }

    pub fn render(&self, source: Option<&str>) {
        let prefix = match self.level {
            DiagnosticLevel::Error => "✗ Error".red().bold(),
            DiagnosticLevel::Warning => "⚠ Warning".yellow().bold(),
            DiagnosticLevel::Suggestion => "💡 Suggestion".cyan().bold(),
        };

        let loc = if let (Some(l), Some(c)) = (self.line, self.column) {
            format!(" at line {}, column {}", l, c)
        } else {
            "".to_string()
        };

        eprintln!("{}{} : {}", prefix, loc, self.message);

        if let (Some(l), Some(src)) = (self.line, source) {
            let lines: Vec<&str> = src.lines().collect();
            if l > 0 && l <= lines.len() {
                let content = lines[l - 1];
                eprintln!("  {}", content);
                if let Some(c) = self.column {
                    if c > 0 {
                        eprintln!("  {}^", " ".repeat(c - 1));
                    }
                }
            }
        }

        if let Some(sug) = &self.suggestion {
            eprintln!("  {} {}", "help:".bold(), sug);
        }
    }
}
