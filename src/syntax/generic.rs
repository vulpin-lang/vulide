//! A single-line scanner for conventional C-family / Python / Rust syntax:
//! line + (same-line) block comments, escaped string and char literals,
//! numbers, `name(` calls, keyword words, operators and brackets.

use super::{Token, TokenKind};

pub struct LangSpec {
    pub keywords: &'static [&'static str],
    pub line_comment: &'static str,
    pub block_comment: Option<(&'static str, &'static str)>,
    /// `'x'` / `'\n'` treated leniently as a short string (skips Rust lifetimes).
    pub char_literal: bool,
    /// `'…'` is a full literal string with no escapes (shell, TOML).
    pub single_quote_string: bool,
    /// A `#word` at the first non-space column is a directive (C preprocessor).
    pub hash_directive: bool,
    /// `$name` / `${name}` / `$1` / `$?` are variables (shell).
    pub dollar_vars: bool,
    /// A line that starts (after indent) with `[` is a section header — the
    /// whole `[…]` / `[[…]]` run is highlighted (TOML).
    pub bracket_headers: bool,
}

const OP_CHARS: &str = "+-*/%<>=!&|^~?:.";

pub fn tokenize(line: &str, spec: &LangSpec) -> Vec<Token> {
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let mut tokens: Vec<Token> = Vec::new();

    let starts_with = |i: usize, pat: &str| -> bool {
        let p: Vec<char> = pat.chars().collect();
        i + p.len() <= n && chars[i..i + p.len()] == p[..]
    };

    let first_non_ws = chars.iter().position(|c| !c.is_whitespace());
    let mut i = 0usize;

    while i < n {
        let c = chars[i];

        // Line comment → rest of line.
        if !spec.line_comment.is_empty() && starts_with(i, spec.line_comment) {
            push(&mut tokens, i, n, TokenKind::Comment);
            break;
        }

        // TOML section header: a line that opens with `[` or `[[`.
        if spec.bracket_headers && first_non_ws == Some(i) && c == '[' {
            let mut j = i + 1;
            while j < n && chars[j] != ']' {
                j += 1;
            }
            while j < n && chars[j] == ']' {
                j += 1;
            }
            push(&mut tokens, i, j.min(n), TokenKind::Keyword);
            i = j.min(n);
            continue;
        }

        // Shell variable: `$name`, `${…}`, `$1`, `$?` `$@` `$#` `$*` `$$` `$!`.
        if spec.dollar_vars && c == '$' && i + 1 < n {
            let d = chars[i + 1];
            let mut j = i + 1;
            if d == '{' {
                while j < n && chars[j] != '}' {
                    j += 1;
                }
                j = (j + 1).min(n);
            } else if d.is_alphanumeric() || d == '_' {
                while j < n && (chars[j].is_alphanumeric() || chars[j] == '_') {
                    j += 1;
                }
            } else if "?@#*$!-".contains(d) {
                j += 1;
            }
            if j > i + 1 {
                push(&mut tokens, i, j, TokenKind::Variable);
                i = j;
                continue;
            }
        }

        // Block comment — only up to its close on THIS line (or EOL).
        if let Some((open, close)) = spec.block_comment
            && starts_with(i, open)
        {
            let mut j = i + open.chars().count();
            while j < n && !starts_with(j, close) {
                j += 1;
            }
            let end = if j < n { j + close.chars().count() } else { n };
            push(&mut tokens, i, end.min(n), TokenKind::Comment);
            i = end.min(n);
            continue;
        }

        // C preprocessor directive: `#word` at the first non-space column.
        if spec.hash_directive && c == '#' && first_non_ws == Some(i) {
            let mut j = i + 1;
            while j < n && (chars[j].is_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            push(&mut tokens, i, j.max(i + 1), TokenKind::Keyword);
            i = j.max(i + 1);
            continue;
        }

        // String literal with `\` escapes.
        if c == '"' {
            let mut j = i + 1;
            while j < n {
                if chars[j] == '\\' {
                    j += 2;
                    continue;
                }
                if chars[j] == '"' {
                    j += 1;
                    break;
                }
                j += 1;
            }
            push(&mut tokens, i, j.min(n), TokenKind::String);
            i = j.min(n);
            continue;
        }

        // Single-quoted string — literal, no escapes (shell, TOML).
        if c == '\'' && spec.single_quote_string {
            let mut j = i + 1;
            while j < n && chars[j] != '\'' {
                j += 1;
            }
            push(&mut tokens, i, (j + 1).min(n), TokenKind::String);
            i = (j + 1).min(n);
            continue;
        }

        // Char literal — but not a Rust lifetime (`'a`) / loop label.
        if c == '\'' && spec.char_literal {
            let mut j = i + 1;
            let mut closed = false;
            while j < n && j <= i + 4 {
                if chars[j] == '\\' {
                    j += 2;
                    continue;
                }
                if chars[j] == '\'' {
                    j += 1;
                    closed = true;
                    break;
                }
                j += 1;
            }
            if closed {
                push(&mut tokens, i, j, TokenKind::String);
                i = j;
                continue;
            }
        }

        // Number: digits, then a loose run of digits / `.` / `_` / hex / suffix.
        if c.is_ascii_digit() {
            let mut j = i + 1;
            while j < n {
                let d = chars[j];
                let exp_sign = (d == '+' || d == '-') && matches!(chars[j - 1], 'e' | 'E');
                if d.is_ascii_alphanumeric() || d == '.' || d == '_' || exp_sign {
                    j += 1;
                } else {
                    break;
                }
            }
            push(&mut tokens, i, j, TokenKind::Number);
            i = j;
            continue;
        }

        // Identifier / keyword / call.
        if c.is_alphabetic() || c == '_' {
            let mut j = i + 1;
            while j < n && (chars[j].is_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            let word: String = chars[i..j].iter().collect();
            if spec.keywords.contains(&word.as_str()) {
                push(&mut tokens, i, j, TokenKind::Keyword);
            } else {
                let mut k = j;
                while k < n && chars[k] == ' ' {
                    k += 1;
                }
                if k < n && chars[k] == '(' {
                    push(&mut tokens, i, j, TokenKind::Function);
                }
            }
            i = j;
            continue;
        }

        if "()[]{}".contains(c) {
            push(&mut tokens, i, i + 1, TokenKind::Bracket);
            i += 1;
            continue;
        }

        if OP_CHARS.contains(c) {
            let mut j = i + 1;
            while j < n && OP_CHARS.contains(chars[j]) {
                j += 1;
            }
            push(&mut tokens, i, j, TokenKind::Operator);
            i = j;
            continue;
        }

        i += 1;
    }

    tokens.sort_by_key(|t| t.start);
    tokens
}

fn push(tokens: &mut Vec<Token>, start: usize, end: usize, kind: TokenKind) {
    if end <= start {
        return;
    }
    if tokens.iter().any(|t| start < t.end && t.start < end) {
        return;
    }
    tokens.push(Token { start, end, kind });
}

/// Base spec — all flags off. `..DEFAULT` keeps the per-language specs short.
const DEFAULT: LangSpec = LangSpec {
    keywords: &[],
    line_comment: "",
    block_comment: None,
    char_literal: false,
    single_quote_string: false,
    hash_directive: false,
    dollar_vars: false,
    bracket_headers: false,
};

pub static PYTHON: LangSpec = LangSpec {
    keywords: &[
        "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del",
        "elif", "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is",
        "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while", "with",
        "yield", "match", "case", "None", "True", "False", "self", "cls",
    ],
    line_comment: "#",
    ..DEFAULT
};

pub static RUST: LangSpec = LangSpec {
    keywords: &[
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
        "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move",
        "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait",
        "true", "type", "union", "unsafe", "use", "where", "while", "String", "Vec", "Option",
        "Result", "Box", "Some", "None", "Ok", "Err",
    ],
    line_comment: "//",
    block_comment: Some(("/*", "*/")),
    char_literal: true,
    ..DEFAULT
};

pub static C: LangSpec = LangSpec {
    keywords: &[
        "auto", "break", "case", "char", "const", "continue", "default", "do", "double", "else",
        "enum", "extern", "float", "for", "goto", "if", "inline", "int", "long", "register",
        "restrict", "return", "short", "signed", "sizeof", "static", "struct", "switch", "typedef",
        "union", "unsigned", "void", "volatile", "while", "bool", "size_t", "NULL", "true",
        "false",
    ],
    line_comment: "//",
    block_comment: Some(("/*", "*/")),
    char_literal: true,
    hash_directive: true,
    ..DEFAULT
};

pub static SHELL: LangSpec = LangSpec {
    keywords: &[
        "if", "then", "elif", "else", "fi", "for", "while", "until", "do", "done", "case", "esac",
        "in", "function", "select", "return", "break", "continue", "local", "readonly", "declare",
        "export", "unset", "shift", "exit", "trap", "set", "source", "alias", "echo", "printf",
        "read", "cd", "true", "false",
    ],
    line_comment: "#",
    single_quote_string: true,
    dollar_vars: true,
    ..DEFAULT
};

pub static TOML: LangSpec = LangSpec {
    keywords: &["true", "false"],
    line_comment: "#",
    single_quote_string: true,
    bracket_headers: true,
    ..DEFAULT
};

pub static JSON: LangSpec = LangSpec {
    // strict JSON has none of these, but JSONC / JSON5 files are common
    keywords: &["true", "false", "null"],
    line_comment: "//",
    block_comment: Some(("/*", "*/")),
    ..DEFAULT
};

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(line: &str, spec: &LangSpec) -> Vec<(usize, usize, TokenKind)> {
        tokenize(line, spec)
            .into_iter()
            .map(|t| (t.start, t.end, t.kind))
            .collect()
    }

    #[test]
    fn python_def_keyword_and_call() {
        let t = kinds("def greet(name):", &PYTHON);
        assert!(t.contains(&(0, 3, TokenKind::Keyword)));
        assert!(t.contains(&(4, 9, TokenKind::Function)));
    }

    #[test]
    fn python_hash_is_a_comment_not_a_directive() {
        assert_eq!(
            kinds("x = 1  # note", &PYTHON).last().unwrap().2,
            TokenKind::Comment
        );
    }

    #[test]
    fn rust_string_escapes_dont_end_it_early() {
        let t = kinds(r#"let s = "a\"b";"#, &RUST);
        assert!(t.contains(&(0, 3, TokenKind::Keyword))); // let
        assert!(
            t.iter()
                .any(|&(s, e, k)| k == TokenKind::String && s == 8 && e == 14)
        );
    }

    #[test]
    fn rust_lifetime_is_not_a_char_literal() {
        let t = kinds("fn f<'a>(x: &'a str) {}", &RUST);
        assert!(!t.iter().any(|&(_, _, k)| k == TokenKind::String));
    }

    #[test]
    fn c_block_comment_same_line_and_preprocessor() {
        let t = kinds("#include <stdio.h>  /* x */", &C);
        assert_eq!(t[0].2, TokenKind::Keyword); // #include
        assert!(t.iter().any(|&(_, _, k)| k == TokenKind::Comment));
    }

    #[test]
    fn c_char_literal() {
        let t = kinds("char c = '\\n';", &C);
        assert!(t.iter().any(|&(_, _, k)| k == TokenKind::String));
    }

    #[test]
    fn numbers_and_operators() {
        let t = kinds("y = 3.14e-2 + 0xFF;", &RUST);
        assert!(t.iter().any(|&(_, _, k)| k == TokenKind::Number));
        assert!(t.iter().any(|&(_, _, k)| k == TokenKind::Operator));
    }

    #[test]
    fn shell_vars_keywords_and_quotes() {
        let t = kinds("if [ \"$HOME\" = '/root' ]; then echo ${x:-1}", &SHELL);
        assert!(t.contains(&(0, 2, TokenKind::Keyword))); // if
        assert!(t.iter().any(|&(_, _, k)| k == TokenKind::Variable)); // $HOME
        assert!(t.iter().any(|&(s, _, k)| k == TokenKind::String && s > 12)); // '/root'
        assert!(t.iter().any(|&(_, _, k)| k == TokenKind::Keyword)); // then
    }

    #[test]
    fn toml_header_string_and_bool() {
        let t = kinds("[package]", &TOML);
        assert_eq!(t, vec![(0, 9, TokenKind::Keyword)]);
        let t = kinds("name = \"vulide\"  # note", &TOML);
        assert!(t.iter().any(|&(_, _, k)| k == TokenKind::String));
        assert!(t.iter().any(|&(_, _, k)| k == TokenKind::Comment));
        assert!(
            kinds("debug = true", &TOML)
                .iter()
                .any(|&(_, _, k)| k == TokenKind::Keyword)
        );
    }

    #[test]
    fn json_strings_and_literals() {
        let t = kinds("  \"key\": [true, null, 42],", &JSON);
        assert!(t.iter().any(|&(_, _, k)| k == TokenKind::String));
        assert!(t.iter().any(|&(_, _, k)| k == TokenKind::Keyword)); // true / null
        assert!(t.iter().any(|&(_, _, k)| k == TokenKind::Number));
    }
}
