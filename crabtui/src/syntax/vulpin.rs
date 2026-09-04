//! Vulpin single-line tokenizer.
//!
//! Mirrors `VulpinHighlighter._build_rules()` from the Python IDE:
//!   - `#` … end of line                         -> Comment
//!   - `"…"` / `'…'` with `\` escapes             -> String
//!   - `$ident`                                   -> Variable
//!   - `$ident(` (next non-space is `(`)          -> Function
//!   - `\d+(\.\d*)?`                              -> Number
//!   - first non-space char in the command set    -> Command
//!   - first non-space char in the control set     -> Control
//!   - runs of ` +-*/%=<>!&|^~ `                  -> Operator
//!   - `()[]{}`                                    -> Bracket
//!
//! `!` (raw Python) lines get generic colouring, same as the Python IDE.

use super::{Token, TokenKind};

/// Statement command letters — every `case` in Vulpin's `parseStatement`
/// (`Vulpin/src/parser.c`). Includes `O` (FOR), which the Python IDE missed.
const COMMAND_CHARS: &str = "GPQXEDKAFRLJWVNZTCYOUS";
/// Control-flow single chars (`?` if, `:` else, `;` endif, `@` while,
/// `&` wend/endfor, `~` endfn).
const CONTROL_CHARS: &str = "?:;@&~";
/// Operators the parser lexes: `+ - * / % < >` and two-char `<= >= == !=`.
const OPERATOR_CHARS: &str = "+-*/%<>=!";

pub fn tokenize(line: &str) -> Vec<Token> {
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let mut tokens: Vec<Token> = Vec::new();

    // Leading command / control char (regex anchored at `^\s*`).
    let first = chars.iter().position(|c| !c.is_whitespace());
    if let Some(i) = first {
        let c = chars[i];
        if c == '#' {
            tokens.push(Token {
                start: i,
                end: n,
                kind: TokenKind::Comment,
            });
            return tokens;
        }
        if COMMAND_CHARS.contains(c) {
            tokens.push(Token {
                start: i,
                end: i + 1,
                kind: TokenKind::Command,
            });
        } else if CONTROL_CHARS.contains(c) {
            tokens.push(Token {
                start: i,
                end: i + 1,
                kind: TokenKind::Control,
            });
        }
    }

    let mut i = 0usize;
    while i < n {
        let c = chars[i];

        // Comment wins for the rest of the line.
        if c == '#' {
            push(&mut tokens, i, n, TokenKind::Comment);
            break;
        }

        // Strings. Vulpin's lexer has NO escape handling: `"` runs to the next
        // `"` or end of line (`Vulpin/src/parser.c`). `'` is not a string
        // delimiter in Vulpin, but we still colour it for editing comfort.
        if c == '"' || c == '\'' {
            let quote = c;
            let mut j = i + 1;
            while j < n && chars[j] != quote {
                j += 1;
            }
            let end = (j + 1).min(n);
            push(&mut tokens, i, end, TokenKind::String);
            i = end;
            continue;
        }

        // `$ident` — variable, or function when followed by `(`.
        if c == '$' && i + 1 < n && (chars[i + 1].is_alphabetic() || chars[i + 1] == '_') {
            let mut j = i + 1;
            while j < n && (chars[j].is_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            let mut k = j;
            while k < n && chars[k] == ' ' {
                k += 1;
            }
            let kind = if k < n && chars[k] == '(' {
                TokenKind::Function
            } else {
                TokenKind::Variable
            };
            push(&mut tokens, i, j, kind);
            i = j;
            continue;
        }

        // Numbers: digits, optional single `.` then digits.
        if c.is_ascii_digit() {
            let mut j = i + 1;
            while j < n && chars[j].is_ascii_digit() {
                j += 1;
            }
            if j < n && chars[j] == '.' {
                j += 1;
                while j < n && chars[j].is_ascii_digit() {
                    j += 1;
                }
            }
            push(&mut tokens, i, j, TokenKind::Number);
            i = j;
            continue;
        }

        // Brackets.
        if "()[]{}".contains(c) {
            push(&mut tokens, i, i + 1, TokenKind::Bracket);
            i += 1;
            continue;
        }

        // Operator runs.
        if OPERATOR_CHARS.contains(c) {
            let mut j = i + 1;
            while j < n && OPERATOR_CHARS.contains(chars[j]) {
                j += 1;
            }
            push(&mut tokens, i, j, TokenKind::Operator);
            i = j;
            continue;
        }

        i += 1;
    }

    tokens.sort_by_key(|t| t.start);
    tokens.dedup_by_key(|t| t.start);
    tokens
}

/// Push `[start, end)` unless it collides with the token already covering that
/// span (the leading Command/Control char, added first).
fn push(tokens: &mut Vec<Token>, start: usize, end: usize, kind: TokenKind) {
    if end <= start {
        return;
    }
    if tokens.iter().any(|t| start < t.end && t.start < end) {
        return;
    }
    tokens.push(Token { start, end, kind });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(line: &str) -> Vec<(usize, usize, TokenKind)> {
        tokenize(line)
            .into_iter()
            .map(|t| (t.start, t.end, t.kind))
            .collect()
    }

    #[test]
    fn leading_command() {
        assert_eq!(kinds("G\"hello\"")[0], (0, 1, TokenKind::Command));
    }

    #[test]
    fn indented_control() {
        assert_eq!(kinds("    ? $x > 10")[0], (4, 5, TokenKind::Control));
    }

    #[test]
    fn comment_swallows_rest() {
        assert_eq!(kinds("# G\"not code\""), vec![(0, 13, TokenKind::Comment)]);
    }

    #[test]
    fn trailing_comment() {
        let t = kinds("x=10 # set x");
        assert!(t.contains(&(5, 12, TokenKind::Comment)));
    }

    #[test]
    fn string_has_no_escapes() {
        // Vulpin's lexer stops the string at the first `"` — `\` is literal.
        assert_eq!(
            kinds(r#"G"a\"b""#),
            vec![
                (0, 1, TokenKind::Command),
                (1, 5, TokenKind::String),
                (6, 7, TokenKind::String),
            ]
        );
    }

    #[test]
    fn for_command_is_a_command() {
        assert_eq!(kinds("O i 0 10")[0], (0, 1, TokenKind::Command));
    }

    #[test]
    fn variable_vs_function() {
        let t = kinds("G $name + $add(2,3)");
        assert!(t.contains(&(2, 7, TokenKind::Variable)));
        assert!(t.contains(&(10, 14, TokenKind::Function)));
    }

    #[test]
    fn numbers_and_operators() {
        let t = kinds("A\"x\"+3.5");
        assert!(t.contains(&(4, 5, TokenKind::Operator)));
        assert!(t.contains(&(5, 8, TokenKind::Number)));
    }

    #[test]
    fn brackets() {
        let t = kinds("F add(a,b)");
        assert!(t.contains(&(5, 6, TokenKind::Bracket)));
        assert!(t.contains(&(9, 10, TokenKind::Bracket)));
    }
}
