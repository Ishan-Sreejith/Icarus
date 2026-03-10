use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandDef {
    pub params: Vec<String>,
    pub body: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyntaxProfile {
    pub commands: HashMap<String, CommandDef>,
}

impl SyntaxProfile {
    pub fn parse(source: &str) -> Result<Self, String> {
        let mut profile = SyntaxProfile::default();
        let lines: Vec<&str> = source.lines().collect();
        let mut i = 0usize;

        while i < lines.len() {
            let line = lines[i].trim();
            i += 1;

            if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
                continue;
            }
            if line == "fforge-main {" || line == "}" {
                continue;
            }

            let Some(header) = line.strip_prefix("command ") else {
                return Err(format!("Invalid syntax.ff line: {}", line));
            };
            let Some(open_paren) = header.find('(') else {
                return Err(format!("Expected '(' in command header: {}", line));
            };
            let Some(close_paren) = header[open_paren + 1..].find(')') else {
                return Err(format!("Expected ')' in command header: {}", line));
            };
            let close_paren = open_paren + 1 + close_paren;
            let name = header[..open_paren].trim();
            let params_str = header[open_paren + 1..close_paren].trim();
            let after = header[close_paren + 1..].trim();
            if after != "{" {
                return Err(format!("Expected '{{' in command header: {}", line));
            }

            let params = if params_str.is_empty() {
                Vec::new()
            } else {
                params_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect::<Vec<_>>()
            };

            let mut body = Vec::new();
            while i < lines.len() {
                let body_line = lines[i].trim();
                i += 1;
                if body_line == "}" {
                    break;
                }
                if !body_line.is_empty() {
                    body.push(body_line.to_string());
                }
            }

            profile
                .commands
                .insert(name.to_string(), CommandDef { params, body });
        }

        Ok(profile)
    }
}

#[allow(dead_code)]
pub fn default_syntax_ff() -> String {
    r#"fforge-main {
command say(value) {
ll.print "hello "
ll.print value
}
command ask(prompt) {
ll.expr __ff_builtin_ask__: prompt
}
command print(value) {
ll.print value
}
command println(value) {
ll.print value
ll.print "\n"
}
command define(name, value) {
ll.var name, value
}
command set(name, value) {
ll.mov name, value
}
command call(name, args) {
ll.call name, args
}
command len(value) {
ll.expr __ff_builtin_len__: value
}
command str(value) {
ll.expr __ff_builtin_str__: value
}
command num(value) {
ll.expr __ff_builtin_num__: value
}
command keys(value) {
ll.expr __ff_builtin_keys__: value
}
command values(value) {
ll.expr __ff_builtin_values__: value
}
command range(start, end) {
ll.expr __ff_builtin_range__: start, end
}
command abs(value) {
ll.expr __ff_builtin_abs__: value
}
command sqrt(value) {
ll.expr __ff_builtin_sqrt__: value
}
command min(left, right) {
ll.expr __ff_builtin_min__: left, right
}
command max(left, right) {
ll.expr __ff_builtin_max__: left, right
}
command pow(left, right) {
ll.expr __ff_builtin_pow__: left, right
}
command contains(left, right) {
ll.expr __ff_builtin_contains__: left, right
}
command emit(line) {
ll.raw line
}
}"#
        .to_string()
}

#[allow(dead_code)]
pub fn write_default_syntax_ff(path: &Path) -> Result<(), String> {
    fs::write(path, default_syntax_ff())
        .map_err(|e| format!("Failed to write '{}': {}", path.display(), e))
}

pub fn preprocess_file(path: &Path) -> Result<String, String> {
    let mut visited = HashSet::new();
    preprocess_file_inner(path, &mut visited)
}

fn preprocess_file_inner(path: &Path, visited: &mut HashSet<PathBuf>) -> Result<String, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|e| format!("Failed to read '{}': {}", path.display(), e))?;
    if !visited.insert(canonical.clone()) {
        return Err(format!(
            "Cyclic syntax/use chain detected at '{}'",
            canonical.display()
        ));
    }

    let source = fs::read_to_string(&canonical)
        .map_err(|e| format!("Failed to read '{}': {}", canonical.display(), e))?;
    let processed = preprocess_source(&source, Some(&canonical), visited)?;
    visited.remove(&canonical);
    Ok(processed)
}

pub fn preprocess_source(
    source: &str,
    current_path: Option<&Path>,
    visited: &mut HashSet<PathBuf>,
) -> Result<String, String> {
    let mut profile = SyntaxProfile::default();
    let mut content_lines = Vec::new();

    for raw_line in source.lines() {
        let trimmed = raw_line.trim();
        if let Some(use_target) = parse_use_ff_line(trimmed) {
            let base = current_path
                .and_then(Path::parent)
                .unwrap_or_else(|| Path::new("."));
            let resolved = base.join(use_target);
            let profile_src = preprocess_file_inner(&resolved, visited)?;
            let parsed = SyntaxProfile::parse(&profile_src)?;
            for (name, def) in parsed.commands {
                profile.commands.insert(name, def);
            }
            continue;
        }
        content_lines.push(raw_line.to_string());
    }

    let expanded = expand_commands(&content_lines, &profile);
    let expression_expanded = expanded
        .iter()
        .map(|line| expand_expression_commands_in_line(line, &profile))
        .collect::<Vec<_>>();
    Ok(lower_low_level(&expression_expanded.join("\n")))
}

fn parse_use_ff_line(line: &str) -> Option<String> {
    let rest = line.strip_prefix("use ")?;
    let trimmed = rest.trim();
    if let Some(stripped) = trimmed.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        if stripped.ends_with(".ff") {
            return Some(stripped.to_string());
        }
    }
    if trimmed.ends_with(".ff") {
        return Some(trimmed.to_string());
    }
    None
}

fn expand_commands(lines: &[String], profile: &SyntaxProfile) -> Vec<String> {
    let mut out = Vec::new();
    for raw_line in lines {
        let trimmed = raw_line.trim();
        if let Some((name, args)) = parse_command_invocation(trimmed) {
            if let Some(def) = profile.commands.get(name) {
                if !is_expression_command(def) {
                    out.extend(expand_command(def, &args));
                    continue;
                }
            }
        }
        out.push(raw_line.clone());
    }
    out
}

fn expand_command(def: &CommandDef, args: &[String]) -> Vec<String> {
    let mut expanded = Vec::new();
    for body_line in &def.body {
        let mut line = body_line.clone();
        for (param, arg) in def.params.iter().zip(args.iter()) {
            line = replace_word(&line, param, arg);
        }
        expanded.push(line);
    }
    expanded
}

fn replace_word(line: &str, word: &str, replacement: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = line.chars().collect();
    let word_chars: Vec<char> = word.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let matches = i + word_chars.len() <= chars.len()
            && chars[i..i + word_chars.len()] == word_chars[..]
            && (i == 0 || !is_ident_char(chars[i - 1]))
            && (i + word_chars.len() == chars.len() || !is_ident_char(chars[i + word_chars.len()]));
        if matches {
            out.push_str(replacement);
            i += word_chars.len();
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn is_expression_command(def: &CommandDef) -> bool {
    def.body.len() == 1
        && def.body[0]
            .trim()
            .strip_prefix("ll.expr ")
            .or_else(|| def.body[0].trim().strip_prefix("ll.expr:"))
            .is_some()
}

fn expression_template(def: &CommandDef, args: &[String]) -> Option<String> {
    let mut line = def
        .body
        .first()?
        .trim()
        .strip_prefix("ll.expr ")
        .or_else(|| def.body.first()?.trim().strip_prefix("ll.expr:"))?
        .trim()
        .to_string();
    for (param, arg) in def.params.iter().zip(args.iter()) {
        line = replace_word(&line, param, arg);
    }
    Some(line)
}

fn parse_command_invocation(line: &str) -> Option<(&str, Vec<String>)> {
    let colon = line.find(':')?;
    let name = line[..colon].trim();
    if name.is_empty() || !name.chars().all(is_ident_char) {
        return None;
    }
    let args_src = line[colon + 1..].trim();
    Some((name, split_top_level_args(args_src)))
}

fn split_top_level_args(source: &str) -> Vec<String> {
    if source.is_empty() {
        return Vec::new();
    }

    let mut args = Vec::new();
    let mut current = String::new();
    let mut depth_paren = 0usize;
    let mut depth_brace = 0usize;
    let mut depth_bracket = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for ch in source.chars() {
        if in_string {
            current.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                current.push(ch);
            }
            '(' => {
                depth_paren += 1;
                current.push(ch);
            }
            ')' => {
                depth_paren = depth_paren.saturating_sub(1);
                current.push(ch);
            }
            '{' => {
                depth_brace += 1;
                current.push(ch);
            }
            '}' => {
                depth_brace = depth_brace.saturating_sub(1);
                current.push(ch);
            }
            '[' => {
                depth_bracket += 1;
                current.push(ch);
            }
            ']' => {
                depth_bracket = depth_bracket.saturating_sub(1);
                current.push(ch);
            }
            ','
                if depth_paren == 0
                    && depth_brace == 0
                    && depth_bracket == 0 =>
            {
                let arg = current.trim();
                if !arg.is_empty() {
                    args.push(arg.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    let arg = current.trim();
    if !arg.is_empty() {
        args.push(arg.to_string());
    }
    args
}

fn expand_expression_commands_in_line(line: &str, profile: &SyntaxProfile) -> String {
    let mut current = line.to_string();
    for _ in 0..32 {
        let Some(next) = expand_expression_commands_once(&current, profile) else {
            break;
        };
        if next == current {
            break;
        }
        current = next;
    }
    current
}

fn expand_expression_commands_once(line: &str, profile: &SyntaxProfile) -> Option<String> {
    let mut best: Option<(usize, usize, String)> = None;

    for (name, def) in &profile.commands {
        if !is_expression_command(def) {
            continue;
        }
        let Some((start, end, args)) = find_expression_invocation(line, name, def.params.len()) else {
            continue;
        };
        let Some(expr) = expression_template(def, &args) else {
            continue;
        };
        let replacement = format!("({})", expr);
        match &best {
            Some((best_start, _, _)) if *best_start <= start => {}
            _ => best = Some((start, end, replacement)),
        }
    }

    let (start, end, replacement) = best?;
    let mut out = String::new();
    out.push_str(&line[..start]);
    out.push_str(&replacement);
    out.push_str(&line[end..]);
    Some(out)
}

fn find_expression_invocation(
    line: &str,
    name: &str,
    arity: usize,
) -> Option<(usize, usize, Vec<String>)> {
    let bytes = line.as_bytes();
    let name_bytes = name.as_bytes();
    let mut i = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    while i < bytes.len() {
        let ch = bytes[i] as char;
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if ch == '"' {
            in_string = true;
            i += 1;
            continue;
        }

        if i + name_bytes.len() <= bytes.len()
            && &bytes[i..i + name_bytes.len()] == name_bytes
            && (i == 0 || !is_ident_char(bytes[i - 1] as char))
            && (i + name_bytes.len() == bytes.len()
                || !is_ident_char(bytes[i + name_bytes.len()] as char))
        {
            let mut colon = i + name_bytes.len();
            while colon < bytes.len() && (bytes[colon] as char).is_ascii_whitespace() {
                colon += 1;
            }
            if colon >= bytes.len() || bytes[colon] as char != ':' {
                i += 1;
                continue;
            }
            let (end, args) = parse_expression_invocation_args(line, colon + 1, arity)?;
            return Some((i, end, args));
        }

        i += 1;
    }

    None
}

fn parse_expression_invocation_args(
    line: &str,
    mut idx: usize,
    arity: usize,
) -> Option<(usize, Vec<String>)> {
    let chars: Vec<char> = line.chars().collect();
    let mut args = Vec::new();

    while idx < chars.len() && chars[idx].is_ascii_whitespace() {
        idx += 1;
    }

    if arity == 0 {
        return Some((idx, args));
    }

    for arg_index in 0..arity {
        let start = idx;
        let mut depth_paren = 0usize;
        let mut depth_brace = 0usize;
        let mut depth_bracket = 0usize;
        let mut in_string = false;
        let mut escaped = false;

        while idx < chars.len() {
            let ch = chars[idx];
            if in_string {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    in_string = false;
                }
                idx += 1;
                continue;
            }

            match ch {
                '"' => {
                    in_string = true;
                    idx += 1;
                }
                '(' => {
                    depth_paren += 1;
                    idx += 1;
                }
                ')' => {
                    if depth_paren == 0 && arg_index + 1 == arity {
                        break;
                    }
                    depth_paren = depth_paren.saturating_sub(1);
                    idx += 1;
                }
                '{' => {
                    depth_brace += 1;
                    idx += 1;
                }
                '}' => {
                    if depth_brace == 0 && arg_index + 1 == arity {
                        break;
                    }
                    depth_brace = depth_brace.saturating_sub(1);
                    idx += 1;
                }
                '[' => {
                    depth_bracket += 1;
                    idx += 1;
                }
                ']' => {
                    if depth_bracket == 0 && arg_index + 1 == arity {
                        break;
                    }
                    depth_bracket = depth_bracket.saturating_sub(1);
                    idx += 1;
                }
                ','
                    if depth_paren == 0
                        && depth_brace == 0
                        && depth_bracket == 0 =>
                {
                    if arg_index + 1 == arity {
                        break;
                    }
                    let arg = chars[start..idx]
                        .iter()
                        .collect::<String>()
                        .trim()
                        .to_string();
                    if arg.is_empty() {
                        return None;
                    }
                    args.push(arg);
                    idx += 1;
                    while idx < chars.len() && chars[idx].is_ascii_whitespace() {
                        idx += 1;
                    }
                    break;
                }
                _ => idx += 1,
            }
        }

        if args.len() == arg_index {
            let arg = chars[start..idx]
                .iter()
                .collect::<String>()
                .trim()
                .to_string();
            if arg.is_empty() {
                return None;
            }
            args.push(arg);
        }
    }

    Some((idx, args))
}

fn lower_low_level(source: &str) -> String {
    let mut out = Vec::new();
    for raw_line in source.lines() {
        let trimmed = raw_line.trim();
        if let Some(expr) = trimmed
            .strip_prefix("ll.print ")
            .or_else(|| trimmed.strip_prefix("ll.print:"))
        {
            out.push(format!("say: {}", expr.trim()));
        } else if let Some(rest) = trimmed
            .strip_prefix("ll.mov ")
            .or_else(|| trimmed.strip_prefix("ll.mov:"))
        {
            if let Some((name, expr)) = rest.split_once(',') {
                out.push(format!("{}: {}", name.trim(), expr.trim()));
            } else {
                out.push(raw_line.to_string());
            }
        } else if let Some(rest) = trimmed
            .strip_prefix("ll.var ")
            .or_else(|| trimmed.strip_prefix("ll.var:"))
        {
            if let Some((name, expr)) = rest.split_once(',') {
                out.push(format!("var {}: {}", name.trim(), expr.trim()));
            } else {
                out.push(raw_line.to_string());
            }
        } else if let Some(rest) = trimmed
            .strip_prefix("ll.call ")
            .or_else(|| trimmed.strip_prefix("ll.call:"))
        {
            if let Some((name, args)) = rest.split_once(',') {
                out.push(format!("{}: {}", name.trim(), args.trim()));
            } else {
                out.push(raw_line.to_string());
            }
        } else if let Some(rest) = trimmed
            .strip_prefix("ll.raw ")
            .or_else(|| trimmed.strip_prefix("ll.raw:"))
        {
            out.push(rest.trim().to_string());
        } else {
            out.push(raw_line.to_string());
        }
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn expands_say_command_from_profile() {
        let profile = SyntaxProfile::parse(
            r#"fforge-main {
command say(value) {
ll.print "hello "
ll.print value
}
}"#,
        )
        .unwrap();
        let source = "say: name";
        let expected = r#"say: "hello "
say: name"#;
        assert_eq!(expand_commands(&[source.to_string()], &profile).join("\n"), r#"ll.print "hello "
ll.print name"#);
        assert_eq!(lower_low_level(r#"ll.print "hello "
ll.print name"#), expected);
    }

    #[test]
    fn expands_generic_commands_with_nested_arguments() {
        let profile = SyntaxProfile::parse(
            r#"fforge-main {
command pack(left, right) {
ll.print left
ll.print right
}
}"#,
        )
        .unwrap();
        let expanded = expand_commands(
            &[r#"pack: [1, 2], { "x": "a,b" }"#.to_string()],
            &profile,
        );
        assert_eq!(
            expanded.join("\n"),
            "ll.print [1, 2]\nll.print { \"x\": \"a,b\" }"
        );
    }

    #[test]
    fn expands_expression_commands_inside_expressions() {
        let profile = SyntaxProfile::parse(
            r#"fforge-main {
command double(value) {
ll.expr value + value
}
}"#,
        )
        .unwrap();
        let expanded =
            expand_expression_commands_in_line(r#"var total: 1 + double: values[0]"#, &profile);
        assert_eq!(expanded, r#"var total: 1 + (values[0] + values[0])"#);
    }

    #[test]
    fn expands_ask_command_to_internal_builtin() {
        let profile = SyntaxProfile::parse(&default_syntax_ff()).unwrap();
        let expanded =
            expand_expression_commands_in_line(r#"var name: ask: "Name? ""#, &profile);
        assert_eq!(expanded, r#"var name: (__ff_builtin_ask__: "Name? ")"#);
    }

    #[test]
    fn preprocesses_use_syntax_ff() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ff_syntax_{}", now));
        fs::create_dir_all(&dir).unwrap();
        let syntax_path = dir.join("syntax.ff");
        fs::write(&syntax_path, default_syntax_ff()).unwrap();
        let source_path = dir.join("main.fr");
        fs::write(&source_path, "use syntax.ff\nsay: \"world\"").unwrap();
        let processed = preprocess_file(&source_path).unwrap();
        assert!(processed.contains(r#"say: "hello ""#));
        assert!(processed.contains(r#"say: "world""#));
    }
}
