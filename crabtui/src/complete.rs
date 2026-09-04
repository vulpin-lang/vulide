//! The autocomplete popup.
//!
//! It is non-modal: Up/Down move the selection, Enter or Tab accepts, Esc
//! dismisses, and any other key edits as normal and refreshes the list. When the
//! selected row has nothing left to insert (a command reminder, a fully-typed
//! method) Enter still makes a newline.
//!
//! Four contexts, tried in this order:
//!   1. **`$name`** — variables (every `$ref` + leading `name =` target) and
//!      user functions (`F name(...)`), prefix-filtered.
//!   2. **`.`** — the string methods `.U .L .S .T .C`.
//!   3. one leading command letter — a one-line reminder of what it does.
//!
//! `Position::col` is a char index, so every scan walks the line as `Vec<char>`
//! (a byte scan would misalign after a non-ASCII glyph). The structural scans
//! for `F name(...)` stay byte-wise because Vulpin identifiers and the leading
//! command are ASCII and strings/comments are never reached.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use crate::buffer::Buffer;
use crate::theme::Theme;

const MAX_VISIBLE: usize = 8;

/// Vulpin's string methods (`Vulpin/src/vm.c` `strMethod`).
const STR_METHODS: &[(&str, &str)] = &[
    ("U", "UPPERCASE"),
    ("L", "lowercase"),
    ("S", "strip whitespace"),
    ("T", "Title Case Each Word"),
    ("C", "Capitalise first letter"),
];

/// Leading statement chars and what they do (`Vulpin/src/parser.c`
/// `parseStatement`). Shown as a one-line reminder when you type one.
const COMMANDS: &[(&str, &str)] = &[
    ("G", "print, with a newline"),
    ("P", "print, no newline"),
    ("K", "read a line of input"),
    ("Q", "quit the program"),
    ("X", "raise an error"),
    ("E", "evaluate / assign a variable"),
    ("A", "in-place arithmetic on a var"),
    ("S", "string find & replace"),
    ("D", "delay seconds  (D \"x\" deletes x)"),
    ("U", "import a module"),
    ("F", "define a function"),
    ("R", "return a value from a function"),
    ("L", "label — a jump target"),
    ("J", "jump to a label"),
    ("W", "switch on a value"),
    ("V", "case value"),
    ("N", "default case"),
    ("Z", "end switch"),
    ("T", "try"),
    ("C", "catch  [name]"),
    ("Y", "end try"),
    ("O", "for   i  start end [step]"),
    ("?", "if  (condition)"),
    (":", "else"),
    (";", "end if"),
    ("@", "while  (condition)"),
    ("&", "end while"),
    ("~", "end function"),
];

/// One row in the popup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// The full token; accepting inserts the part past `Completion::prefix`.
    pub text: String,
    /// Dimmed hint shown to the right (a kind, a signature, a description).
    pub detail: String,
}

impl Candidate {
    fn new(text: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            detail: detail.into(),
        }
    }
}

pub struct Completion {
    /// Columns from the popup's anchor (the `$` / `.` / command char) to the
    /// cursor — how far left of the cursor to draw the box.
    pub anchor_back: u16,
    /// Text already typed from the anchor's token start to the cursor.
    pub prefix: String,
    pub items: Vec<Candidate>,
    pub selected: usize,
}

impl Completion {
    /// Build a completion for the cursor's context, or `None`.
    pub fn detect(buffer: &Buffer) -> Option<Completion> {
        let cur = buffer.cursor();
        let chars: Vec<char> = buffer.line_text(cur.line).chars().collect();
        let col = cur.col.min(chars.len());

        detect_dollar(buffer, &chars, col)
            .or_else(|| detect_dot(&chars, col))
            .or_else(|| detect_command(&chars, col))
    }

    pub fn current(&self) -> &Candidate {
        &self.items[self.selected]
    }

    /// The text to insert when accepted (the part not already typed).
    pub fn completion_tail(&self) -> &str {
        &self.current().text[self.prefix.len()..]
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.checked_sub(1).unwrap_or(self.items.len() - 1);
    }

    pub fn move_down(&mut self) {
        self.selected = (self.selected + 1) % self.items.len();
    }
}

// ---- context detection ----

/// `$name` — variables and user functions.
fn detect_dollar(buffer: &Buffer, chars: &[char], col: usize) -> Option<Completion> {
    let mut start = col;
    while start > 0 && is_word_char(chars[start - 1]) {
        start -= 1;
    }
    if start == 0 || chars[start - 1] != '$' {
        return None;
    }
    let prefix: String = chars[start..col].iter().collect();

    let mut vars: Vec<String> = Vec::new();
    let mut funcs: Vec<(String, Vec<String>)> = Vec::new();
    for i in 0..buffer.line_count() {
        let line = buffer.line_text(i);
        collect_names(&line, &mut vars);
        if let Some(f) = parse_function(&line) {
            funcs.push(f);
        }
    }
    vars.sort();
    vars.dedup();
    funcs.sort();
    funcs.dedup();

    let mut items: Vec<Candidate> = Vec::new();
    for (name, params) in &funcs {
        if name.starts_with(&prefix) && *name != prefix {
            items.push(Candidate::new(
                name.clone(),
                format!("fn({})", params.join(", ")),
            ));
        }
    }
    for v in &vars {
        if v.starts_with(&prefix) && *v != prefix && !funcs.iter().any(|(n, _)| n == v) {
            items.push(Candidate::new(v.clone(), "var"));
        }
    }
    if items.is_empty() {
        return None;
    }
    Some(Completion {
        anchor_back: (col - (start - 1)) as u16, // include the `$`
        prefix,
        items,
        selected: 0,
    })
}

/// `.` followed by 0–1 letters — the string methods.
fn detect_dot(chars: &[char], col: usize) -> Option<Completion> {
    let mut start = col;
    while start > 0 && chars[start - 1].is_ascii_alphabetic() {
        start -= 1;
    }
    if start == 0 || chars[start - 1] != '.' {
        return None;
    }
    let prefix: String = chars[start..col].iter().collect();
    if prefix.chars().count() > 1 {
        return None; // `.ident` module member, not a method
    }
    let items: Vec<Candidate> = STR_METHODS
        .iter()
        .filter(|(k, _)| k.starts_with(&prefix))
        .map(|(k, d)| Candidate::new(*k, *d))
        .collect();
    if items.is_empty() {
        return None;
    }
    Some(Completion {
        anchor_back: (col - (start - 1)) as u16, // include the `.`
        prefix,
        items,
        selected: 0,
    })
}

/// Exactly one leading command char and nothing else on the line — a reminder.
fn detect_command(chars: &[char], col: usize) -> Option<Completion> {
    let before: String = chars[..col].iter().collect();
    let lead = before.trim_start();
    if lead.chars().count() != 1 {
        return None;
    }
    if !chars[col..].iter().collect::<String>().trim().is_empty() {
        return None;
    }
    let (key, desc) = COMMANDS.iter().find(|(k, _)| *k == lead)?;
    let cmd_pos = chars.iter().position(|c| !c.is_whitespace()).unwrap_or(col);
    Some(Completion {
        anchor_back: col.saturating_sub(cmd_pos) as u16,
        prefix: lead.to_string(),
        items: vec![Candidate::new(*key, *desc)],
        selected: 0,
    })
}

// ---- line scans ----

fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Push every identifier this line defines or references:
///   - a leading `ident =` assignment (Vulpin's `parseStatement` default case)
///   - every `$ident` reference
fn collect_names(line: &str, out: &mut Vec<String>) {
    let bytes = line.as_bytes();

    let mut i = 0;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    if i < bytes.len() && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
        let s = i;
        while i < bytes.len() && is_word(bytes[i]) {
            i += 1;
        }
        let mut j = i;
        while j < bytes.len() && bytes[j] == b' ' {
            j += 1;
        }
        if j < bytes.len() && bytes[j] == b'=' && bytes.get(j + 1) != Some(&b'=') {
            out.push(line[s..i].to_string());
        }
    }

    let mut k = 0;
    while k + 1 < bytes.len() {
        if bytes[k] == b'$' && (bytes[k + 1].is_ascii_alphabetic() || bytes[k + 1] == b'_') {
            let s = k + 1;
            let mut e = s;
            while e < bytes.len() && is_word(bytes[e]) {
                e += 1;
            }
            out.push(line[s..e].to_string());
            k = e;
        } else {
            k += 1;
        }
    }
}

/// `F name(a, b)` on this line → `(name, [a, b])`.
fn parse_function(line: &str) -> Option<(String, Vec<String>)> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    if bytes.get(i) != Some(&b'F') {
        return None;
    }
    i += 1;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    if i >= bytes.len() || !(bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
        return None;
    }
    let s = i;
    while i < bytes.len() && is_word(bytes[i]) {
        i += 1;
    }
    let name = line[s..i].to_string();

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
            if i >= bytes.len() || !(bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
                break;
            }
            let ps = i;
            while i < bytes.len() && is_word(bytes[i]) {
                i += 1;
            }
            params.push(line[ps..i].to_string());
        }
    }
    Some((name, params))
}

/// Draw the popup anchored under the context sigil (falls back to above).
pub fn render_popup(
    f: &mut Frame,
    c: &Completion,
    cursor_screen: (u16, u16),
    theme: &Theme,
    area: Rect,
) {
    let (cx, cy) = cursor_screen;
    let text_w = c
        .items
        .iter()
        .map(|i| i.text.chars().count())
        .max()
        .unwrap_or(1);
    let detail_w = c
        .items
        .iter()
        .map(|i| i.detail.chars().count())
        .max()
        .unwrap_or(0);
    // ` text  detail ` — one leading space, two between, one trailing.
    let width = (text_w + detail_w + 4).clamp(8, 48) as u16;
    let visible = c.items.len().min(MAX_VISIBLE) as u16;

    let top = if c.selected as u16 >= visible {
        c.selected as u16 + 1 - visible
    } else {
        0
    };

    let anchor_x = cx.saturating_sub(c.anchor_back);
    let x = anchor_x.min(area.x + area.width.saturating_sub(width));
    let below = cy + 1 + visible <= area.y + area.height;
    let y = if below {
        cy + 1
    } else {
        cy.saturating_sub(visible)
    };

    let rect = Rect {
        x,
        y,
        width,
        height: visible,
    };
    f.render_widget(Clear, rect);

    let rows: Vec<Line> = (top..top + visible)
        .filter_map(|i| c.items.get(i as usize).map(|item| (i, item)))
        .map(|(i, item)| {
            let selected = i as usize == c.selected;
            let (bg, modifier) = if selected {
                (theme.autocomplete_sel, Modifier::BOLD)
            } else {
                (theme.autocomplete_bg, Modifier::empty())
            };
            let name = Style::default()
                .fg(theme.autocomplete_fg)
                .bg(bg)
                .add_modifier(modifier);
            let detail = Style::default().fg(theme.comment).bg(bg);
            let pad = (width as usize).saturating_sub(text_w + item.detail.chars().count() + 4);
            Line::from(vec![
                Span::styled(format!(" {:<w$}  ", item.text, w = text_w), name),
                Span::styled(format!("{}{}", item.detail, " ".repeat(pad + 1)), detail),
            ])
        })
        .collect();

    f.render_widget(
        Paragraph::new(rows).style(Style::default().bg(theme.autocomplete_bg)),
        rect,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Position;

    fn texts(c: &Completion) -> Vec<&str> {
        c.items.iter().map(|i| i.text.as_str()).collect()
    }

    #[test]
    fn suggests_referenced_identifiers() {
        let mut b = Buffer::from_str("G $name + $number\nG $na");
        b.set_cursor(Position { line: 1, col: 5 }, false);
        let c = Completion::detect(&b).expect("in a $word context");
        assert_eq!(c.prefix, "na");
        assert_eq!(texts(&c), vec!["name"]);
        assert_eq!(c.completion_tail(), "me");
    }

    #[test]
    fn suggests_assignment_targets() {
        let mut b = Buffer::from_str("counter = 0\nG $c");
        b.set_cursor(Position { line: 1, col: 4 }, false);
        let c = Completion::detect(&b).unwrap();
        assert_eq!(texts(&c), vec!["counter"]);
    }

    #[test]
    fn suggests_functions_with_signature() {
        let mut b = Buffer::from_str("F greet(name, punct)\n  G $name\n~\nG $gr");
        b.set_cursor(Position { line: 3, col: 5 }, false);
        let c = Completion::detect(&b).unwrap();
        assert_eq!(texts(&c), vec!["greet"]);
        assert_eq!(c.current().detail, "fn(name, punct)");
    }

    #[test]
    fn suggests_string_methods_after_a_dot() {
        let mut b = Buffer::from_str("x = $name.");
        b.set_cursor(Position { line: 0, col: 10 }, false);
        let c = Completion::detect(&b).unwrap();
        assert_eq!(texts(&c), vec!["U", "L", "S", "T", "C"]);
        // narrow to `.S`
        let mut b = Buffer::from_str("x = $name.S");
        b.set_cursor(Position { line: 0, col: 11 }, false);
        let c = Completion::detect(&b).unwrap();
        assert_eq!(texts(&c), vec!["S"]);
        assert_eq!(c.completion_tail(), "");
    }

    #[test]
    fn dot_with_two_letters_is_not_a_method() {
        let mut b = Buffer::from_str("U mymod\n$mymod.foo");
        b.set_cursor(Position { line: 1, col: 10 }, false);
        assert!(Completion::detect(&b).is_none());
    }

    #[test]
    fn lone_command_letter_shows_its_hint() {
        let mut b = Buffer::from_str("K");
        b.set_cursor(Position { line: 0, col: 1 }, false);
        let c = Completion::detect(&b).unwrap();
        assert_eq!(texts(&c), vec!["K"]);
        assert!(c.current().detail.contains("input"));

        // indented is fine too
        let mut b = Buffer::from_str("  @");
        b.set_cursor(Position { line: 0, col: 3 }, false);
        let c = Completion::detect(&b).unwrap();
        assert_eq!(c.current().detail, "while  (condition)");
    }

    #[test]
    fn command_hint_stops_once_the_statement_has_more() {
        let mut b = Buffer::from_str("G $x");
        b.set_cursor(Position { line: 0, col: 1 }, false); // right after G
        assert!(
            Completion::detect(&b).is_none(),
            "`$x` follows, not a bare G"
        );
    }

    #[test]
    fn none_outside_dollar_context() {
        let mut b = Buffer::from_str("name = 1\nGG na");
        b.set_cursor(Position { line: 1, col: 5 }, false);
        assert!(Completion::detect(&b).is_none());
    }

    #[test]
    fn none_when_only_match_is_what_was_typed() {
        let mut b = Buffer::from_str("$name\n$name");
        b.set_cursor(Position { line: 1, col: 5 }, false);
        assert!(Completion::detect(&b).is_none());
    }
}
