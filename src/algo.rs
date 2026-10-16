//! Structure outline — the "algorithm viewer".
//!
//! A single pass over the buffer turns the leading command char of each line
//! into a flat list of [`Item`]s with a nesting `depth`, which the sidebar
//! (`src/ui/algo.rs`) renders as an indented tree and the app uses for
//! jump-to-line.
//!
//! Depth comes from an explicit stack of open blocks, not a running counter:
//! a half-typed file (the normal editing state) has unbalanced closers, and a
//! saturating counter would flatten the rest of the outline after the first
//! stray `~`. A closer that matches nothing on the stack is simply ignored.
//!
//! Block grammar (`Vulpin/src/parser.c` `parseStmtAt`), classified by
//! [`vulpin::lead`] so the outline and auto-indent agree:
//!   openers  F(func)  ?(if)  @(while)  O(for)  E i, v(enumerate)  W(switch)  T(try)
//!   closers  ~(endfn) ;(endif) &(endwhile/endfor/enumerate) Z(endsw) Y(endtry)
//!   in-block :(else)  :?(elif)  V(case)  N(default)  C(catch) — shown one level in
//! `T fn list` (filter), `W file …` (write), `Z a b` (zip) and an inline
//! `?cond;stmt;` are not blocks.

use crate::buffer::Buffer;
use crate::syntax::vulpin::{self, Lead};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Function,
    While,
    For,
    If,
    Else,
    Switch,
    Case,
    Try,
    Catch,
    Label,
    Jump,
    Return,
}

impl Kind {
    /// Short tag shown before the label in the sidebar.
    pub fn tag(self) -> &'static str {
        match self {
            Kind::Function => "ƒn",
            Kind::While => "@",
            Kind::For => "O",
            Kind::If => "?",
            Kind::Else => ":",
            Kind::Switch => "W",
            Kind::Case => "V",
            Kind::Try => "T",
            Kind::Catch => "C",
            Kind::Label => "L",
            Kind::Jump => "J",
            Kind::Return => "R",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// 0-based buffer line.
    pub line: usize,
    /// Indent level in the tree.
    pub depth: usize,
    pub kind: Kind,
    /// Text after the tag (function signature, condition, label name, …).
    pub label: String,
}

/// Walk the buffer and produce the outline.
pub fn outline(buf: &Buffer) -> Vec<Item> {
    let mut items = Vec::new();
    let mut stack: Vec<Kind> = Vec::new();

    for line in 0..buf.line_count() {
        let raw = buf.line_text(line);
        let trimmed = raw.trim_start();
        let mut cs = trimmed.chars();
        let Some(cmd) = cs.next() else { continue };
        let rest = cs.as_str().trim();

        match (vulpin::lead(trimmed), cmd) {
            // ---- openers ----
            (Lead::Open(_), 'F') => {
                items.push(mk(line, stack.len(), Kind::Function, function_label(rest)));
                stack.push(Kind::Function);
            }
            (Lead::Open(_), '?') => {
                items.push(mk(line, stack.len(), Kind::If, prefixed("if", rest)));
                stack.push(Kind::If);
            }
            (Lead::Open(_), '@') => {
                items.push(mk(line, stack.len(), Kind::While, prefixed("while", rest)));
                stack.push(Kind::While);
            }
            (Lead::Open(_), 'O') => {
                items.push(mk(line, stack.len(), Kind::For, prefixed("for", rest)));
                stack.push(Kind::For);
            }
            (Lead::Open(_), 'E') => {
                items.push(mk(
                    line,
                    stack.len(),
                    Kind::For,
                    prefixed("enumerate", rest),
                ));
                stack.push(Kind::For);
            }
            (Lead::Open(_), 'W') => {
                items.push(mk(
                    line,
                    stack.len(),
                    Kind::Switch,
                    prefixed("switch", rest),
                ));
                stack.push(Kind::Switch);
            }
            (Lead::Open(_), 'T') => {
                items.push(mk(line, stack.len(), Kind::Try, "try".to_string()));
                stack.push(Kind::Try);
            }

            // ---- closers (pop the nearest matching opener, drop danglers) ----
            (Lead::Close(_), '~') => pop_to(&mut stack, &[Kind::Function]),
            (Lead::Close(_), ';') => pop_to(&mut stack, &[Kind::If]),
            (Lead::Close(_), '&') => pop_to(&mut stack, &[Kind::While, Kind::For]),
            (Lead::Close(_), 'Z') => pop_to(&mut stack, &[Kind::Switch]),
            (Lead::Close(_), 'Y') => pop_to(&mut stack, &[Kind::Try]),

            // ---- in-block markers (one level inside their opener) ----
            (Lead::Middle('e'), _) => items.push(mk(
                line,
                depth_inside(&stack),
                Kind::Else,
                prefixed("else if", rest.trim_start_matches('?').trim()),
            )),
            (Lead::Middle(_), ':') => items.push(mk(
                line,
                depth_inside(&stack),
                Kind::Else,
                "else".to_string(),
            )),
            (Lead::Middle(_), 'V') => items.push(mk(
                line,
                depth_inside(&stack),
                Kind::Case,
                prefixed("case", rest),
            )),
            (Lead::Middle(_), 'N') => items.push(mk(
                line,
                depth_inside(&stack),
                Kind::Case,
                "default".to_string(),
            )),
            (Lead::Middle(_), 'C') => items.push(mk(
                line,
                depth_inside(&stack),
                Kind::Catch,
                if rest.is_empty() {
                    "catch".to_string()
                } else {
                    format!("catch {}", clip(rest, 24))
                },
            )),

            // ---- leaves ----
            (_, 'L') if !rest.is_empty() => {
                items.push(mk(line, stack.len(), Kind::Label, rest.to_string()))
            }
            (_, 'J') if !rest.is_empty() => items.push(mk(
                line,
                stack.len(),
                Kind::Jump,
                format!("→ {}", clip(rest, 24)),
            )),
            (_, 'R') if !is_file_form(rest) => items.push(mk(
                line,
                stack.len(),
                Kind::Return,
                if rest.is_empty() {
                    "return".to_string()
                } else {
                    format!("return {}", clip(rest, 20))
                },
            )),
            _ => {}
        }
    }
    items
}

/// `R file "path"` reads a file — not a return.
fn is_file_form(rest: &str) -> bool {
    rest.strip_prefix("file")
        .is_some_and(|r| !r.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_'))
}

fn mk(line: usize, depth: usize, kind: Kind, label: String) -> Item {
    Item {
        line,
        depth,
        kind,
        label,
    }
}

/// Depth for an in-block marker: one level inside its enclosing opener.
fn depth_inside(stack: &[Kind]) -> usize {
    stack.len().saturating_sub(1)
}

/// Pop the stack back to (and including) the nearest entry of one of `wanted`,
/// discarding any unclosed inner blocks. A closer that matches nothing is a
/// no-op.
fn pop_to(stack: &mut Vec<Kind>, wanted: &[Kind]) {
    if let Some(pos) = stack.iter().rposition(|k| wanted.contains(k)) {
        stack.truncate(pos);
    }
}

/// `"if"` + a clipped condition, or just `"if"` when the rest is empty.
fn prefixed(word: &str, rest: &str) -> String {
    if rest.is_empty() {
        word.to_string()
    } else {
        format!("{word} {}", clip(rest, 26))
    }
}

/// `F greet(a, b)` rest -> `greet(a, b)`; falls back to the raw rest.
fn function_label(rest: &str) -> String {
    let bytes = rest.as_bytes();
    if bytes.is_empty() || !(bytes[0].is_ascii_alphabetic() || bytes[0] == b'_') {
        return if rest.is_empty() {
            "(anonymous)".to_string()
        } else {
            clip(rest, 28)
        };
    }
    let mut i = 0;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        i += 1;
    }
    let name = &rest[..i];
    let mut params = Vec::new();
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    if bytes.get(i) == Some(&b'(') {
        i += 1;
        loop {
            while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b',') {
                i += 1;
            }
            if i >= bytes.len() || !(bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                break;
            }
            let s = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            params.push(&rest[s..i]);
        }
    }
    format!("{name}({})", params.join(", "))
}

/// Truncate to `max` chars, appending `…` when cut.
fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    format!(
        "{}…",
        s.chars().take(max.saturating_sub(1)).collect::<String>()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(items: &[Item]) -> Vec<(usize, Kind)> {
        items.iter().map(|i| (i.depth, i.kind)).collect()
    }

    #[test]
    fn filter_zip_file_and_inline_if_are_not_blocks() {
        let src = "\
T odd [1,2,3]
W file \"o.txt\" \"hi\"
R file \"o.txt\"
?a==5;G\"yes\";
? x > 1
  G 1
:? x > 0
  G 2
;
Z a b
E i, v [1, 2]
  G v
&
R filename
G \"top\"";
        let items = outline(&Buffer::from_str(src));
        let got: Vec<(usize, usize, Kind, &str)> = items
            .iter()
            .map(|i| (i.line, i.depth, i.kind, i.label.as_str()))
            .collect();
        assert_eq!(
            got,
            vec![
                (4, 0, Kind::If, "if x > 1"),
                (6, 0, Kind::Else, "else if x > 0"),
                (10, 0, Kind::For, "enumerate i, v [1, 2]"),
                (13, 0, Kind::Return, "return filename"),
            ]
        );
    }

    #[test]
    fn nests_functions_loops_and_conditionals() {
        let src = "\
F process(data, limit)
  @ $i < $limit
    ? $data > 0
      G \"positive\"
    :
      G \"other\"
    ;
  &
  R $total
~
G \"top level\"";
        let items = outline(&Buffer::from_str(src));
        assert_eq!(
            kinds(&items),
            vec![
                (0, Kind::Function),
                (1, Kind::While),
                (2, Kind::If),
                (2, Kind::Else),
                (1, Kind::Return),
            ]
        );
        assert_eq!(items[0].label, "process(data, limit)");
        assert_eq!(items[1].label, "while $i < $limit");
    }

    #[test]
    fn outline_labels_and_lines() {
        let src = "\
F greet(name)
  ? $name == \"\"
    R \"hi\"
  ;
  G $name
~
L loop_start
J loop_start";
        let items = outline(&Buffer::from_str(src));
        let got: Vec<(usize, usize, Kind, &str)> = items
            .iter()
            .map(|i| (i.line, i.depth, i.kind, i.label.as_str()))
            .collect();
        assert_eq!(
            got,
            vec![
                (0, 0, Kind::Function, "greet(name)"),
                (1, 1, Kind::If, "if $name == \"\""),
                (2, 2, Kind::Return, "return \"hi\""),
                (6, 0, Kind::Label, "loop_start"),
                (7, 0, Kind::Jump, "→ loop_start"),
            ]
        );
    }

    #[test]
    fn stray_closer_does_not_flatten_the_rest() {
        let src = "\
G \"x\"
~
G \"y\"
F later()
  ? $z
    R 1
  ;
~";
        let items = outline(&Buffer::from_str(src));
        // the `~` on line 2 matches nothing; line 4's F still opens a block
        let f = items.iter().find(|i| i.kind == Kind::Function).unwrap();
        assert_eq!(f.depth, 0);
        let iff = items.iter().find(|i| i.kind == Kind::If).unwrap();
        assert_eq!(iff.depth, 1, "if still nests under the function");
    }

    #[test]
    fn switch_and_try_blocks() {
        let src = "\
W $day
V 1
  G \"mon\"
N
  G \"other\"
Z
T
  X \"boom\"
C err
  G $err
Y";
        let items = outline(&Buffer::from_str(src));
        // in-block markers align with their opener (like `:` else with `?` if)
        assert_eq!(
            kinds(&items),
            vec![
                (0, Kind::Switch),
                (0, Kind::Case),
                (0, Kind::Case),
                (0, Kind::Try),
                (0, Kind::Catch),
            ]
        );
        assert_eq!(items[1].label, "case 1");
        assert_eq!(items[2].label, "default");
        assert_eq!(items[4].label, "catch err");
    }
}
