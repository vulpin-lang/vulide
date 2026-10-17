//! The autocomplete popup.
//!
//! It is non-modal: Up/Down move the selection, Enter or Tab accepts, Esc
//! dismisses, and any other key edits as normal and refreshes the list. When the
//! selected row has nothing left to insert (a command reminder, a fully-typed
//! method) Enter still makes a newline.
//!
//! Contexts, tried in this order:
//!   1. **`$name`** — variables (every `$ref`, assignment target, loop/input/
//!      catch variable). Functions aren't offered here: `$f(…)` calls the
//!      function *named by the string in* `f`, so `$greet(…)` is a runtime error.
//!   2. **`.`** — `math.` / `random.` members, else the string methods
//!      `.U .L .S .T .C .R .SP .J .F`.
//!   3. one leading command letter — a one-line reminder of what it does.
//!   4. **a bare word** (2+ chars, outside strings/comments) — user functions,
//!      variables and builtins. This one is *passive*: only Tab accepts, and
//!      Enter / Up / Down keep their editing meaning, since it pops up while
//!      typing ordinary code.
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
use crate::syntax::TokenKind;
use crate::syntax::vulpin::{self, BUILTINS, MODULE_MEMBERS};
use crate::theme::Theme;

const MAX_VISIBLE: usize = 8;

/// Vulpin's string methods (`isStrMeth` in `Vulpin/src/parser.c`, `strMethod`
/// in `vm.c`).
const STR_METHODS: &[(&str, &str)] = &[
    ("U", "UPPERCASE"),
    ("L", "lowercase"),
    ("S", "strip whitespace"),
    ("T", "Title Case Each Word"),
    ("C", "Capitalise first letter"),
    ("R", "R(old, new) — replace"),
    ("SP", "SP(sep) — split into a List"),
    ("J", "J(list) — join list with this string"),
    ("F", "F(sub) — index of sub"),
];

/// Leading statement chars and what they do (`Vulpin/src/parser.c`
/// `parseStmtAt`). Shown as a one-line reminder when you type one.
const COMMANDS: &[(&str, &str)] = &[
    ("G", "print, with a newline"),
    ("P", "print, no newline"),
    ("K", "input   K name \"prompt\" [\"I\"|\"F\"]"),
    ("Q", "quit   [exit code]"),
    ("X", "run a shell command  (E\"msg\" raises an error)"),
    ("!", "run a shell command"),
    ("E", "E name value · E\"msg\" raises · E i, v list loops"),
    ("A", "in-place arithmetic   A\"x\"+5"),
    ("S", "string replace   S\"var\"\"old\"\"new\""),
    ("D", "delay seconds  (D \"x\" deletes x)"),
    ("I", "inspect — print type and value"),
    ("M", "map   M fn list"),
    ("U", "import a module"),
    ("F", "define a function   F name(a, !optional)"),
    ("R", "return a value   (R file \"p\" reads a file)"),
    ("L", "label — a jump target"),
    ("J", "jump to a label"),
    ("W", "switch on a value   (W file \"p\" \"text\" writes)"),
    ("V", "case value"),
    ("N", "default case"),
    ("Z", "end switch"),
    ("T", "try   (T fn list filters)"),
    ("C", "catch  [name]  — holds the error message"),
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
    /// Only Tab accepts; Enter / Up / Down go to the editor (bare-word context).
    pub passive: bool,
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
            .or_else(|| detect_word(buffer, &chars, col))
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
    if start == 0 || chars[start - 1] != '$' || in_string_or_comment(chars, col) {
        return None;
    }
    let prefix: String = chars[start..col].iter().collect();

    let (vars, funcs) = scan_buffer(buffer);
    // `$i` is complete when `i` is a defined variable, even if `idx` exists
    // too: then the popup only offers (Tab), and Enter stays a newline.
    // Definitions, not `vars` — that also holds the `$i` being typed.
    let mut defined = std::collections::HashSet::new();
    for i in 0..buffer.line_count() {
        crate::lint::collect_definitions(&buffer.line_text(i), &mut defined);
    }
    let complete = defined.contains(&prefix);

    let mut items: Vec<Candidate> = Vec::new();
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
        passive: complete,
    })
}

/// Every variable and user function (`F name(...)`) in the buffer, sorted.
fn scan_buffer(buffer: &Buffer) -> (Vec<String>, Vec<(String, Vec<String>)>) {
    let mut vars: Vec<String> = Vec::new();
    let mut funcs: Vec<(String, Vec<String>)> = Vec::new();
    let mut defined = std::collections::HashSet::new();
    for i in 0..buffer.line_count() {
        let line = buffer.line_text(i);
        collect_names(&line, &mut vars);
        crate::lint::collect_definitions(&line, &mut defined);
        if let Some(f) = parse_function(&line) {
            funcs.push(f);
        }
    }
    vars.extend(defined);
    vars.sort();
    vars.dedup();
    funcs.sort();
    funcs.dedup();
    (vars, funcs)
}

/// A bare identifier being typed — functions, variables, builtins.
fn detect_word(buffer: &Buffer, chars: &[char], col: usize) -> Option<Completion> {
    let mut start = col;
    while start > 0 && is_word_char(chars[start - 1]) {
        start -= 1;
    }
    // Compact `Gname` / `Kname`: the leading command letter isn't part of it.
    let first = chars.iter().position(|c| !c.is_whitespace())?;
    if start == first && col > start && chars[start].is_ascii_uppercase() {
        start += 1;
    }
    if col < start + 2 || chars[start].is_ascii_digit() {
        return None;
    }
    if start > 0 && matches!(chars[start - 1], '$' | '.') {
        return None;
    }
    if in_string_or_comment(chars, col) {
        return None;
    }
    let prefix: String = chars[start..col].iter().collect();
    let (vars, funcs) = scan_buffer(buffer);

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
    for (b, detail) in BUILTINS {
        if b.starts_with(&prefix) && *b != prefix && !items.iter().any(|c| c.text == *b) {
            items.push(Candidate::new(*b, *detail));
        }
    }
    if items.is_empty() {
        return None;
    }
    Some(Completion {
        anchor_back: (col - start) as u16,
        prefix,
        items,
        selected: 0,
        passive: true,
    })
}

/// `.` — a built-in module's members, or the string methods.
/// Whether `col` is inside a string or comment on this line.
fn in_string_or_comment(chars: &[char], col: usize) -> bool {
    let line: String = chars.iter().collect();
    vulpin::tokenize(&line).iter().any(|t| {
        matches!(t.kind, TokenKind::String | TokenKind::Comment) && t.start < col && col <= t.end
    })
}

fn detect_dot(chars: &[char], col: usize) -> Option<Completion> {
    let mut start = col;
    while start > 0 && chars[start - 1].is_ascii_alphabetic() {
        start -= 1;
    }
    if start == 0 || chars[start - 1] != '.' {
        return None;
    }
    // A method needs something to call it on: a name, a string or a closing
    // bracket — not the full stop ending a comment, nor `3.` in a number.
    let dot = start - 1;
    let owner = dot.checked_sub(1).map(|i| chars[i]);
    if !owner.is_some_and(|c| is_word_char(c) || matches!(c, '"' | ')' | ']'))
        || in_string_or_comment(chars, dot + 1)
    {
        return None;
    }
    let mut word = dot;
    while word > 0 && is_word_char(chars[word - 1]) {
        word -= 1;
    }
    if word < dot && chars[word..dot].iter().all(char::is_ascii_digit) {
        return None;
    }
    let prefix: String = chars[start..col].iter().collect();

    // The word before the dot: `math.` / `random.` list their members.
    let mut owner_start = start - 1;
    while owner_start > 0 && is_word_char(chars[owner_start - 1]) {
        owner_start -= 1;
    }
    let owner: String = chars[owner_start..start - 1].iter().collect();
    let items: Vec<Candidate> =
        if let Some((_, members)) = MODULE_MEMBERS.iter().find(|(m, _)| *m == owner) {
            members
                .iter()
                .filter(|(k, _)| k.starts_with(&prefix) && *k != prefix)
                .map(|(k, d)| Candidate::new(*k, *d))
                .collect()
        } else {
            // Method names are all caps; anything longer is a module member.
            STR_METHODS
                .iter()
                .filter(|(k, _)| k.starts_with(&prefix))
                .map(|(k, d)| Candidate::new(*k, *d))
                .collect()
        };
    if items.is_empty() {
        return None;
    }
    Some(Completion {
        anchor_back: (col - (start - 1)) as u16, // include the `.`
        prefix,
        items,
        selected: 0,
        passive: false,
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
        passive: false,
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
            // `!` marks an optional parameter
            while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b',' || bytes[i] == b'!') {
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
    // Never wider or taller than the editor; a rect past the frame's edge
    // would make `Clear` index out of the buffer.
    let width = ((text_w + detail_w + 4).clamp(8, 48) as u16).min(area.width);
    let visible = (c.items.len().min(MAX_VISIBLE) as u16).min(area.height);
    if width == 0 || visible == 0 {
        return;
    }

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
    }
    .intersection(f.area());
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
    fn suggests_functions_with_signature_as_bare_words() {
        let mut b = Buffer::from_str("F greet(name, !punct)\n  G $name\n~\nG gr");
        b.set_cursor(Position { line: 3, col: 4 }, false);
        let c = Completion::detect(&b).unwrap();
        assert_eq!(texts(&c), vec!["greet"]);
        assert_eq!(c.current().detail, "fn(name, punct)");
        assert!(c.passive);
    }

    #[test]
    fn dollar_offers_variables_not_functions() {
        // `$greet(…)` is a runtime error in Vulpin, so `$` never offers it.
        let mut b = Buffer::from_str("F greet(n)\n~\ngreeting = 1\nG $gr");
        b.set_cursor(Position { line: 3, col: 5 }, false);
        let c = Completion::detect(&b).unwrap();
        assert_eq!(texts(&c), vec!["greeting"]);
        assert!(!c.passive);
    }

    #[test]
    fn suggests_string_methods_after_a_dot() {
        let mut b = Buffer::from_str("x = $name.");
        b.set_cursor(Position { line: 0, col: 10 }, false);
        let c = Completion::detect(&b).unwrap();
        assert_eq!(
            texts(&c),
            vec!["U", "L", "S", "T", "C", "R", "SP", "J", "F"]
        );
        // narrow to `.S`
        let mut b = Buffer::from_str("x = $name.S");
        b.set_cursor(Position { line: 0, col: 11 }, false);
        let c = Completion::detect(&b).unwrap();
        assert_eq!(texts(&c), vec!["S", "SP"]);
        assert_eq!(c.completion_tail(), "");
    }

    #[test]
    fn module_members_after_math_dot() {
        let mut b = Buffer::from_str("G math.s");
        b.set_cursor(Position { line: 0, col: 8 }, false);
        let c = Completion::detect(&b).unwrap();
        assert_eq!(texts(&c), vec!["sqrt"]);
        let mut b = Buffer::from_str("G random.");
        b.set_cursor(Position { line: 0, col: 9 }, false);
        assert_eq!(texts(&Completion::detect(&b).unwrap()), vec!["randint"]);
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
    fn bare_words_complete_passively() {
        // Was `none_outside_dollar_context`: bare names are the normal style
        // now, so they complete too — passively (Tab only).
        let mut b = Buffer::from_str("name = 1\nGG na");
        b.set_cursor(Position { line: 1, col: 5 }, false);
        let c = Completion::detect(&b).unwrap();
        assert_eq!(texts(&c), vec!["name"]);
        assert!(c.passive);
        // compact `Gna` — the command letter isn't part of the word
        let mut b = Buffer::from_str("name = 1\nGna");
        b.set_cursor(Position { line: 1, col: 3 }, false);
        assert_eq!(texts(&Completion::detect(&b).unwrap()), vec!["name"]);
        // builtins too
        let mut b = Buffer::from_str("G le");
        b.set_cursor(Position { line: 0, col: 4 }, false);
        assert_eq!(texts(&Completion::detect(&b).unwrap()), vec!["len"]);
    }

    #[test]
    fn no_bare_word_completion_in_strings_comments_or_one_char() {
        for (src, col) in [
            ("name = 1\nG \"na", 5),
            ("name = 1\nG 1 # na", 8),
            ("name = 1\nG n", 3),
        ] {
            let mut b = Buffer::from_str(src);
            b.set_cursor(Position { line: 1, col }, false);
            assert!(Completion::detect(&b).is_none(), "{src:?}");
        }
    }

    #[test]
    fn none_when_only_match_is_what_was_typed() {
        let mut b = Buffer::from_str("$name\n$name");
        b.set_cursor(Position { line: 1, col: 5 }, false);
        assert!(Completion::detect(&b).is_none());
    }
}
