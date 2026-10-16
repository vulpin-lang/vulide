//! Vulpin single-line tokenizer, plus the block-structure rules the outline and
//! auto-indent share.
//!
//! Follows the interpreter's lexer (`Vulpin/src/parser.c`, `adv` /
//! `scanString` / `parseStmtAt`):
//!   - `#` … end of line                                  -> Comment
//!   - `"…"` `'…'` `` `…` `` and tripled `'''…'''`,
//!     optionally prefixed `r`/`R` (raw) or `$` (format)   -> String
//!     A `\` always skips the next char when finding the end, raw or not.
//!   - `$ident`                                           -> Variable
//!   - `ident(` / a builtin function name                 -> Function
//!   - any other identifier                               -> Variable
//!   - `in` `true` `false` `None`                         -> Keyword
//!   - `\d+(\.\d*)?([eE][+-]?\d+)?`                        -> Number
//!   - first non-space char in the command set            -> Command
//!   - first non-space char in the control set            -> Control
//!   - runs of `+-*/%<>=!&|`                              -> Operator
//!   - `()[]{}`                                           -> Bracket
//!
//! Any string may span lines (`scanString` reads straight through newlines),
//! so [`tokenize_from`] takes and returns the string still open at a line
//! boundary.

use super::{Token, TokenKind};

/// Statement letters — every `case` in `parseStmtAt` plus the bare closers
/// (`Q`, `N`, `Y`) and `!` (raw shell command).
const COMMAND_CHARS: &str = "GPQXEDKAFRLJWVNZTCYOUSMI!";
/// Control-flow single chars (`?` if, `:` else, `;` endif, `@` while,
/// `&` wend/endfor, `~` endfn).
const CONTROL_CHARS: &str = "?:;@&~";
/// Operator characters the lexer accepts, incl. `&&` `||`, `|` (comprehension)
/// and `<-`.
const OPERATOR_CHARS: &str = "+-*/%<>=!&|";
/// Word literals / operators.
const KEYWORDS: &[&str] = &["in", "true", "false", "None"];

/// Functions built into the interpreter (`cf_named` in `Vulpin/src/vm.c`).
pub const BUILTINS: &[(&str, &str)] = &[
    ("len", "len(x) — length of a List/Dict/Set/Str"),
    ("push", "push(list, v) — append to a List"),
    ("pop", "pop(list) — remove & return the last item"),
    ("keys", "keys(dict) — List of keys"),
    ("values", "values(dict) — List of values"),
    ("map", "map(fn, list)"),
    ("filter", "filter(fn, list)"),
    ("zip", "zip(a, b) — List of pairs"),
    ("enumerate", "enumerate(list) — [[i, v], …]"),
    ("read", "read(path) — file contents"),
    ("write", "write(path, text)"),
    ("fmt", "fmt(\"{} {name}\", args…)"),
    ("abs", "abs(n)"),
    ("pow", "pow(a, b)"),
    ("sq", "sq(n) — square root"),
    ("o", "o(cmd) — run a shell command"),
];

/// Members of the built-in pseudo-modules (`bc` in `Vulpin/src/vm.c`).
pub const MODULE_MEMBERS: &[(&str, &[(&str, &str)])] = &[
    (
        "math",
        &[
            ("sqrt", "sqrt(n)"),
            ("floor", "floor(n)"),
            ("ceil", "ceil(n)"),
            ("abs", "abs(n)"),
            ("pi", "π"),
            ("e", "Euler's number"),
        ],
    ),
    ("random", &[("randint", "randint(a, b) — inclusive")]),
];

/// A string left open at the end of a line: its quote char, and whether it's
/// tripled. `None` when the line ends outside any string.
pub type Open = Option<(char, bool)>;

/// Tokens for one line on its own (as if no string is open before it).
pub fn tokenize(line: &str) -> Vec<Token> {
    tokenize_from(line, None).0
}

/// Tokens for one line that starts inside `open` (the string carried over from
/// the line above), plus the string still open at its end.
pub fn tokenize_from(line: &str, open: Open) -> (Vec<Token>, Open) {
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let mut tokens: Vec<Token> = Vec::new();
    let mut open_out: Open = None;

    let mut i = 0usize;
    if let Some((quote, triple)) = open {
        // Finish the carried-over string; it's mid-statement, so no leading
        // command char after it.
        let (end, still) = string_body_end(&chars, 0, quote, triple);
        push(&mut tokens, 0, end, TokenKind::String);
        if still.is_some() {
            return (tokens, still);
        }
        i = end;
    } else if let Some(first) = chars.iter().position(|c| !c.is_whitespace()) {
        // Leading command / control char (regex anchored at `^\s*`). The main
        // scan starts after it, so compact forms like `Gname` or `Knum"…"`
        // still get their identifier coloured.
        let c = chars[first];
        if c == '#' {
            tokens.push(Token {
                start: first,
                end: n,
                kind: TokenKind::Comment,
            });
            return (tokens, None);
        }
        let kind = if COMMAND_CHARS.contains(c) {
            Some(TokenKind::Command)
        } else if CONTROL_CHARS.contains(c) {
            Some(TokenKind::Control)
        } else {
            None
        };
        if let Some(kind) = kind {
            tokens.push(Token {
                start: first,
                end: first + 1,
                kind,
            });
            i = first + 1;
        }
    }

    while i < n {
        let c = chars[i];

        // Comment wins for the rest of the line.
        if c == '#' {
            push(&mut tokens, i, n, TokenKind::Comment);
            break;
        }

        // Strings, with an optional `r`/`R`/`$` prefix.
        // (`$` can't be part of a word, so `P$"{x}"` counts; `r`/`R` must
        // start a word, or `bar"…"` would swallow the string's prefix.)
        let prefixed = chars.get(i + 1).is_some_and(|q| is_quote(*q))
            && (c == '$' || (matches!(c, 'r' | 'R') && (i == 0 || !is_word_char(chars[i - 1]))));
        if is_quote(c) || prefixed {
            let q = if prefixed { i + 1 } else { i };
            let (end, still) = string_end(&chars, q);
            open_out = still;
            push(&mut tokens, i, end, TokenKind::String);
            i = end;
            continue;
        }

        // `$ident` — a variable reference.
        if c == '$' && chars.get(i + 1).is_some_and(|c| is_ident_start(*c)) {
            let j = word_end(&chars, i + 1);
            push(&mut tokens, i, j, TokenKind::Variable);
            i = j;
            continue;
        }

        // Numbers: digits, optional `.digits`, optional exponent.
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
            if j < n && (chars[j] == 'e' || chars[j] == 'E') {
                let mut k = j + 1;
                if k < n && (chars[k] == '+' || chars[k] == '-') {
                    k += 1;
                }
                if k < n && chars[k].is_ascii_digit() {
                    while k < n && chars[k].is_ascii_digit() {
                        k += 1;
                    }
                    j = k;
                }
            }
            push(&mut tokens, i, j, TokenKind::Number);
            i = j;
            continue;
        }

        // Identifiers: keyword, function call / builtin, or variable. A name
        // right after `.` is a method or module member, coloured as a function.
        if is_ident_start(c) {
            let j = word_end(&chars, i);
            let word: String = chars[i..j].iter().collect();
            let mut k = j;
            while k < n && chars[k] == ' ' {
                k += 1;
            }
            let after_dot = i > 0 && chars[i - 1] == '.';
            let kind = if KEYWORDS.contains(&word.as_str()) {
                TokenKind::Keyword
            } else if after_dot
                || (k < n && chars[k] == '(')
                || BUILTINS.iter().any(|(b, _)| *b == word)
            {
                TokenKind::Function
            } else {
                TokenKind::Variable
            };
            push(&mut tokens, i, j, kind);
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
    (tokens, open_out)
}

/// End (exclusive) of the string whose opening quote is at `q`, and the string
/// left open if it runs off the end of the line. Mirrors `scanString`: triple
/// quotes when the quote repeats three times, and a `\` always skips the
/// following char.
fn string_end(chars: &[char], q: usize) -> (usize, Open) {
    let n = chars.len();
    let quote = chars[q];
    let triple = q + 2 < n && chars[q + 1] == quote && chars[q + 2] == quote;
    string_body_end(chars, if triple { q + 3 } else { q + 1 }, quote, triple)
}

/// Scan a string body from `j` to just past its closing quote(s).
fn string_body_end(chars: &[char], mut j: usize, quote: char, triple: bool) -> (usize, Open) {
    let n = chars.len();
    while j < n {
        if chars[j] == '\\' && j + 1 < n {
            j += 2;
            continue;
        }
        if triple {
            if j + 2 < n && chars[j] == quote && chars[j + 1] == quote && chars[j + 2] == quote {
                return (j + 3, None);
            }
        } else if chars[j] == quote {
            return (j + 1, None);
        }
        j += 1;
    }
    (n, Some((quote, triple)))
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

fn is_quote(c: char) -> bool {
    c == '"' || c == '\'' || c == '`'
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn word_end(chars: &[char], mut i: usize) -> usize {
    while i < chars.len() && is_word_char(chars[i]) {
        i += 1;
    }
    i
}

// ---- block structure ----

/// What a line does to the block nesting, judged from its leading statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lead {
    /// Opens a block: `F ? @ O W T`, and `E i, v list` (enumerate).
    Open(char),
    /// Closes a block: `~ ; & Z Y`.
    Close(char),
    /// Sits inside its opener and starts a new section: `: V N C`, and `:?`
    /// (elif, reported as `'e'`).
    Middle(char),
    /// Anything else.
    Other,
}

/// Classify a line. The same leading letter can mean different statements:
/// `T` alone is try but `T fn list` is filter, `W file …` writes a file rather
/// than opening a switch, `Z` alone ends a switch while `Z a b` is zip, and
/// `E i, v list` loops (closed by `&`) while `E name value` / `E"msg"` don't.
pub fn lead(line: &str) -> Lead {
    let trimmed = line.trim_start();
    let mut cs = trimmed.chars();
    let Some(cmd) = cs.next() else {
        return Lead::Other;
    };
    let rest = cs.as_str();
    let bare = is_bare(rest);
    match cmd {
        'F' | '@' | 'O' => Lead::Open(cmd),
        'E' if enumerate_vars(rest).is_some() => Lead::Open('E'),
        '?' if closes_inline(rest) => Lead::Other,
        '?' => Lead::Open('?'),
        'T' if bare => Lead::Open('T'),
        'W' if !starts_with_word(rest, "file") => Lead::Open('W'),
        '~' | ';' | '&' | 'Y' => Lead::Close(cmd),
        'Z' if bare => Lead::Close('Z'),
        ':' if rest.trim_start().starts_with('?') => Lead::Middle('e'),
        ':' | 'V' | 'N' | 'C' => Lead::Middle(cmd),
        _ => Lead::Other,
    }
}

/// Whether a newline after this line should indent one more level.
pub fn opens_indent(line: &str) -> bool {
    matches!(lead(line), Lead::Open(_) | Lead::Middle(_))
}

/// `i, v …` after `E` — the enumerate loop's index and value names.
pub fn enumerate_vars(rest: &str) -> Option<(String, String)> {
    let chars: Vec<char> = rest.trim_start().chars().collect();
    let ident = |at: usize| -> Option<usize> {
        (at < chars.len() && is_ident_start(chars[at])).then(|| word_end(&chars, at))
    };
    let skip_ws = |mut at: usize| {
        while at < chars.len() && chars[at].is_whitespace() {
            at += 1;
        }
        at
    };
    let a_end = ident(0)?;
    let comma = skip_ws(a_end);
    if chars.get(comma) != Some(&',') {
        return None;
    }
    let b_start = skip_ws(comma + 1);
    let b_end = ident(b_start)?;
    Some((
        chars[..a_end].iter().collect(),
        chars[b_start..b_end].iter().collect(),
    ))
}

/// Nothing but whitespace, a statement separator or a comment follows.
fn is_bare(rest: &str) -> bool {
    let r = rest.trim_start();
    r.is_empty() || r.starts_with(';') || r.starts_with(':') || r.starts_with('#')
}

fn starts_with_word(rest: &str, word: &str) -> bool {
    let r = rest.trim_start();
    r.starts_with(word) && !r[word.len()..].starts_with(is_word_char)
}

/// An inline `?cond;stmt;` — the if gets a body and its closing `;` on the
/// same line. (`?cond;` alone is just the condition; the body follows.)
fn closes_inline(rest: &str) -> bool {
    let chars: Vec<char> = rest.chars().collect();
    let mut seen_cond = false;
    let mut body = false; // current segment has non-space text
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '#' {
            break;
        }
        if is_quote(c) {
            i = string_end(&chars, i).0;
            body = true;
            continue;
        }
        if c == ';' {
            if seen_cond && body {
                return true;
            }
            seen_cond = true;
            body = false;
        } else if !c.is_whitespace() {
            body = true;
        }
        i += 1;
    }
    false
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
    fn escaped_quote_stays_inside_the_string() {
        assert_eq!(
            kinds(r#"G"a\"b""#),
            vec![(0, 1, TokenKind::Command), (1, 7, TokenKind::String)]
        );
    }

    #[test]
    fn all_quote_styles_and_prefixes() {
        assert!(kinds("G 'hi'").contains(&(2, 6, TokenKind::String)));
        assert!(kinds("G `hi`").contains(&(2, 6, TokenKind::String)));
        assert!(kinds("G '''a ' b'''").contains(&(2, 13, TokenKind::String)));
        assert!(kinds(r#"G r"a\n""#).contains(&(2, 8, TokenKind::String)));
        assert!(kinds(r#"G $"hi {x}""#).contains(&(2, 11, TokenKind::String)));
    }

    #[test]
    fn strings_carry_across_lines() {
        let (t, open) = tokenize_from("G```hello", None);
        assert_eq!(
            t[1],
            Token {
                start: 1,
                end: 9,
                kind: TokenKind::String
            }
        );
        assert_eq!(open, Some(('`', true)));
        // a middle line is all string, still open
        let (t, open) = tokenize_from("hello \"there\" G", open);
        assert_eq!(
            t,
            vec![Token {
                start: 0,
                end: 15,
                kind: TokenKind::String
            }]
        );
        // the closing line: string up to the quotes, then normal tokens
        let (t, open) = tokenize_from("``` + 1", open);
        assert_eq!(
            t[0],
            Token {
                start: 0,
                end: 3,
                kind: TokenKind::String
            }
        );
        assert!(t.contains(&Token {
            start: 6,
            end: 7,
            kind: TokenKind::Number
        }));
        assert_eq!(open, None);
        // a plain single-quoted string spans lines too
        let (_, open) = tokenize_from("G\"a", None);
        assert_eq!(open, Some(('"', false)));
        assert_eq!(tokenize_from("b\"", open).1, None);
        // an escaped quote at the end doesn't close it
        assert_eq!(tokenize_from(r#"G"a\""#, None).1, Some(('"', false)));
    }

    #[test]
    fn for_command_is_a_command() {
        assert_eq!(kinds("O i 0 10")[0], (0, 1, TokenKind::Command));
    }

    #[test]
    fn variables_and_functions() {
        let t = kinds("G $name + add(2,3) + total");
        assert!(t.contains(&(2, 7, TokenKind::Variable)));
        assert!(t.contains(&(10, 13, TokenKind::Function)));
        assert!(t.contains(&(21, 26, TokenKind::Variable)));
    }

    #[test]
    fn compact_form_colours_the_identifier() {
        let t = kinds("Gname");
        assert_eq!(t[0], (0, 1, TokenKind::Command));
        assert!(t.contains(&(1, 5, TokenKind::Variable)));
    }

    #[test]
    fn builtins_methods_and_keywords() {
        let t = kinds("G len(l) + math.pi");
        assert!(t.contains(&(2, 5, TokenKind::Function)));
        assert!(t.contains(&(16, 18, TokenKind::Function)));
        let t = kinds("? 2 in s");
        assert!(t.contains(&(4, 6, TokenKind::Keyword)));
    }

    #[test]
    fn numbers_and_operators() {
        let t = kinds("A\"x\"+3.5");
        assert!(t.contains(&(4, 5, TokenKind::Operator)));
        assert!(t.contains(&(5, 8, TokenKind::Number)));
        assert!(kinds("G 1e3").contains(&(2, 5, TokenKind::Number)));
    }

    #[test]
    fn brackets() {
        let t = kinds("F add(a,b)");
        assert!(t.contains(&(5, 6, TokenKind::Bracket)));
        assert!(t.contains(&(9, 10, TokenKind::Bracket)));
    }

    #[test]
    fn lead_distinguishes_same_letter_statements() {
        assert_eq!(lead("T"), Lead::Open('T'));
        assert_eq!(lead("T odd [1,2,3]"), Lead::Other);
        assert_eq!(lead("W fruit"), Lead::Open('W'));
        assert_eq!(lead("W file \"o.txt\" \"hi\""), Lead::Other);
        assert_eq!(lead("Z"), Lead::Close('Z'));
        assert_eq!(lead("Z a b"), Lead::Other);
        assert_eq!(lead(":? x > 1"), Lead::Middle('e'));
        assert_eq!(lead("C\"err\""), Lead::Middle('C'));
        assert_eq!(lead("E i, v [\"a\"]"), Lead::Open('E'));
        assert_eq!(lead("E x 5"), Lead::Other);
        assert_eq!(lead("E\"boom\""), Lead::Other);
    }

    #[test]
    fn format_string_after_a_compact_command() {
        assert!(kinds(r#"P$"{x}""#).contains(&(1, 7, TokenKind::String)));
    }

    #[test]
    fn inline_if_opens_nothing() {
        assert_eq!(lead("?a==5;G\"yes\";"), Lead::Other);
        assert_eq!(lead("?a==5;G\"a;b\";"), Lead::Other);
        assert_eq!(lead("?a==5;"), Lead::Open('?'));
        assert_eq!(lead("?a==5;G\"yes\""), Lead::Open('?'));
        assert_eq!(lead("? a == 5"), Lead::Open('?'));
    }
}
