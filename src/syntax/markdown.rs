//! A single-line Markdown scanner: headings, code spans + fences, blockquotes,
//! list markers, links, and bold.
//!
//! No cross-line state — a line inside a fenced code block is still scanned as
//! Markdown, and a `**` that opens on one line and closes on the next is not
//! matched. Good enough for a status-glance at a README.

use super::{Token, TokenKind};

pub fn tokenize(line: &str) -> Vec<Token> {
    let ch: Vec<char> = line.chars().collect();
    let n = ch.len();
    let mut out: Vec<Token> = Vec::new();
    let first = ch.iter().position(|c| !c.is_whitespace()).unwrap_or(n);

    // Whole-line: fenced code marker, thematic break.
    if run_at(&ch, first, '`') >= 3 || run_at(&ch, first, '~') >= 3 || is_thematic_break(&ch, first)
    {
        push(&mut out, 0, n, TokenKind::Comment);
        return out;
    }

    // ATX heading: 1–6 `#` then a space or end of line.
    if first < n && ch[first] == '#' {
        let hashes = run_at(&ch, first, '#');
        if hashes <= 6 && (first + hashes >= n || ch[first + hashes] == ' ') {
            push(&mut out, 0, n, TokenKind::Heading);
            return out;
        }
    }

    // Line prefix: blockquote marker or list bullet.
    let mut body = 0usize;
    if first < n && ch[first] == '>' {
        let mut q = first + 1;
        if q < n && ch[q] == ' ' {
            q += 1;
        }
        push(&mut out, 0, q, TokenKind::Comment);
        body = q;
    } else if let Some(end) = list_marker(&ch, first) {
        push(&mut out, first, end, TokenKind::Operator);
        body = end;
    }

    // Inline scan.
    let mut i = body;
    while i < n {
        let c = ch[i];

        // `code` / ``code`` — matching backtick run of the same length.
        if c == '`' {
            let len = run_at(&ch, i, '`');
            if let Some(close) = find_run(&ch, i + len, '`', len) {
                push(&mut out, i, close + len, TokenKind::String);
                i = close + len;
                continue;
            }
        }

        // [text](url) or ![alt](url)
        if c == '[' || (c == '!' && ch.get(i + 1) == Some(&'[')) {
            let open = if c == '!' { i + 1 } else { i };
            if let Some((rb, lp, rp)) = link_parts(&ch, open) {
                push(&mut out, open + 1, rb, TokenKind::Variable); // link text
                push(&mut out, lp, rp + 1, TokenKind::String); // (url)
                i = rp + 1;
                continue;
            }
        }

        // **bold** / __bold__
        if (c == '*' || c == '_')
            && ch.get(i + 1) == Some(&c)
            && let Some(close) = find_run(&ch, i + 2, c, 2)
        {
            push(&mut out, i, close + 2, TokenKind::Emphasis);
            i = close + 2;
            continue;
        }

        i += 1;
    }

    out.sort_by_key(|t| t.start);
    out
}

/// How many `c` in a row starting at `i`.
fn run_at(ch: &[char], i: usize, c: char) -> usize {
    let mut k = i;
    while k < ch.len() && ch[k] == c {
        k += 1;
    }
    k - i
}

/// Index of the start of a run of exactly `len` copies of `c` at or after `from`.
fn find_run(ch: &[char], from: usize, c: char, len: usize) -> Option<usize> {
    let mut i = from;
    while i < ch.len() {
        if ch[i] == c {
            let r = run_at(ch, i, c);
            if r == len {
                return Some(i);
            }
            i += r;
        } else {
            i += 1;
        }
    }
    None
}

fn is_thematic_break(ch: &[char], first: usize) -> bool {
    if first >= ch.len() {
        return false;
    }
    let marker = ch[first];
    if !matches!(marker, '-' | '*' | '_') {
        return false;
    }
    let mut count = 0;
    for &c in &ch[first..] {
        if c == marker {
            count += 1;
        } else if c != ' ' {
            return false;
        }
    }
    count >= 3
}

/// `(text_close_bracket, open_paren, close_paren)` for `[text](url)` at `lb`.
fn link_parts(ch: &[char], lb: usize) -> Option<(usize, usize, usize)> {
    let rb = (lb + 1..ch.len()).find(|&k| ch[k] == ']')?;
    if ch.get(rb + 1) != Some(&'(') {
        return None;
    }
    let lp = rb + 1;
    let rp = (lp + 1..ch.len()).find(|&k| ch[k] == ')')?;
    Some((rb, lp, rp))
}

/// End index (exclusive) of a `-`/`*`/`+` or `1.`/`1)` bullet + its space.
fn list_marker(ch: &[char], first: usize) -> Option<usize> {
    let n = ch.len();
    if first >= n {
        return None;
    }
    if matches!(ch[first], '-' | '*' | '+') && ch.get(first + 1) == Some(&' ') {
        return Some(first + 2);
    }
    let mut k = first;
    while k < n && ch[k].is_ascii_digit() {
        k += 1;
    }
    if k > first && k < n && matches!(ch[k], '.' | ')') && ch.get(k + 1) == Some(&' ') {
        return Some(k + 2);
    }
    None
}

fn push(out: &mut Vec<Token>, start: usize, end: usize, kind: TokenKind) {
    if end <= start {
        return;
    }
    if out.iter().any(|t| start < t.end && t.start < end) {
        return;
    }
    out.push(Token { start, end, kind });
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
    fn heading_is_the_whole_line() {
        assert_eq!(kinds("## Title here"), vec![(0, 13, TokenKind::Heading)]);
        // seven hashes is not a heading
        assert!(kinds("####### nope").is_empty());
    }

    #[test]
    fn fence_and_thematic_break() {
        assert_eq!(kinds("```rust").first().unwrap().2, TokenKind::Comment);
        assert_eq!(kinds("---").first().unwrap().2, TokenKind::Comment);
    }

    #[test]
    fn inline_code_and_link() {
        let t = kinds("see `foo()` and [docs](http://x.y)");
        assert!(t.contains(&(4, 11, TokenKind::String))); // `foo()`
        assert!(t.iter().any(|&(_, _, k)| k == TokenKind::Variable)); // link text
        assert!(t.iter().any(|&(s, _, k)| k == TokenKind::String && s > 11)); // (url)
    }

    #[test]
    fn list_marker_and_bold() {
        let t = kinds("- some **bold** text");
        assert_eq!(t[0], (0, 2, TokenKind::Operator)); // "- "
        assert!(t.iter().any(|&(_, _, k)| k == TokenKind::Emphasis));
    }

    #[test]
    fn blockquote_prefix_only() {
        let t = kinds("> quoted `code`");
        assert_eq!(t[0], (0, 2, TokenKind::Comment)); // "> "
        assert!(t.iter().any(|&(_, _, k)| k == TokenKind::String));
    }
}
