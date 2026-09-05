//! Incremental find / replace.
//!
//! `Ctrl+F` opens a bar docked below the editor (and below the output panel, if
//! that is open). It captures all keys while open — like the overlays, but it
//! keeps the editor visible so matches highlight as you type:
//!
//!   type in **Find**    live-filter the matches, jump to the first
//!   Enter / Shift+Enter  next / previous match (wraps)
//!   Tab / Shift+Tab      switch between the Find and Replace fields
//!   Ctrl+R               replace the current match, advance to the next
//!   Alt+A                replace every match
//!   Alt+C / click [Aa]   toggle case sensitivity (ASCII fold; off by default)
//!   Alt+X / click [.*]   toggle regex (Find is a pattern, not a literal)
//!   Esc                  close the bar
//!
//! Match positions are `(start, end)` char indices — `Position::col` is a char
//! index, so matching walks each line char-by-char (a byte scan would misalign
//! on a line with a non-ASCII glyph before the match). The actual substring /
//! regex matching is shared with project-wide search via `crate::matcher`.

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Position as TermPos, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::buffer::{Buffer, Position};
use crate::matcher::Matcher;
use crate::theme::Theme;

/// Rows the bar occupies when open: Find, Replace, hints.
pub const SEARCH_ROWS: u16 = 3;

/// Width of the `"▸ Find "` / `"▸ Repl "` label column.
const LABEL_W: u16 = 7;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Find,
    Replace,
}

/// What the app should do with a key the bar just consumed.
pub enum SearchAction {
    /// Field focus / caret moved — just redraw the bar.
    Stay,
    /// The find query or case flag changed — recompute the match list.
    Requery,
    Close,
    Next,
    Prev,
    ReplaceOne,
    ReplaceAll,
}

pub struct Search {
    pub find: Buffer,
    pub replace: Buffer,
    pub field: Field,
    pub case_sensitive: bool,
    pub regex: bool,
    /// Hit rects for the `[Aa]` / `[.*]` toggle buttons — set by `render`,
    /// read by the app's mouse handler.
    pub case_rect: Option<Rect>,
    pub regex_rect: Option<Rect>,
}

impl Search {
    pub fn new(seed: &str) -> Self {
        let mut find = Buffer::from_str(seed);
        find.move_doc_end(false);
        Self {
            find,
            replace: Buffer::new(),
            field: Field::Find,
            case_sensitive: false,
            regex: false,
            case_rect: None,
            regex_rect: None,
        }
    }

    /// `Some(message)` when regex mode is on and the query doesn't compile —
    /// shown in the counter slot in place of "no matches".
    pub fn regex_error(&self) -> Option<String> {
        if !self.regex {
            return None;
        }
        Matcher::compile(&self.query(), self.case_sensitive, true).err()
    }

    pub fn query(&self) -> String {
        self.find.rope().to_string()
    }

    pub fn replacement(&self) -> String {
        self.replace.rope().to_string()
    }

    fn active(&mut self) -> &mut Buffer {
        match self.field {
            Field::Find => &mut self.find,
            Field::Replace => &mut self.replace,
        }
    }

    /// The query field just changed — `Requery` when editing Find, else `Stay`.
    fn edited(&self) -> SearchAction {
        match self.field {
            Field::Find => SearchAction::Requery,
            Field::Replace => SearchAction::Stay,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> SearchAction {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);

        if alt {
            return match key.code {
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    self.case_sensitive = !self.case_sensitive;
                    SearchAction::Requery
                }
                KeyCode::Char('x') | KeyCode::Char('X') => {
                    self.regex = !self.regex;
                    SearchAction::Requery
                }
                KeyCode::Char('a') | KeyCode::Char('A') => SearchAction::ReplaceAll,
                _ => SearchAction::Stay,
            };
        }
        if ctrl {
            return match key.code {
                KeyCode::Char('r') | KeyCode::Char('R') => SearchAction::ReplaceOne,
                _ => SearchAction::Stay,
            };
        }

        match key.code {
            KeyCode::Esc => SearchAction::Close,
            KeyCode::Enter if shift => SearchAction::Prev,
            KeyCode::Enter => match self.field {
                Field::Find => SearchAction::Next,
                Field::Replace => SearchAction::ReplaceOne,
            },
            KeyCode::Tab | KeyCode::Down => {
                self.field = Field::Replace;
                SearchAction::Stay
            }
            KeyCode::BackTab | KeyCode::Up => {
                self.field = Field::Find;
                SearchAction::Stay
            }
            KeyCode::Left => {
                self.active().move_left(false);
                SearchAction::Stay
            }
            KeyCode::Right => {
                self.active().move_right(false);
                SearchAction::Stay
            }
            KeyCode::Home => {
                self.active().move_home(false);
                SearchAction::Stay
            }
            KeyCode::End => {
                self.active().move_end(false);
                SearchAction::Stay
            }
            KeyCode::Backspace => {
                self.active().delete_backward();
                self.edited()
            }
            KeyCode::Delete => {
                self.active().delete_forward();
                self.edited()
            }
            KeyCode::Char(c) => {
                self.active().insert_char(c);
                self.edited()
            }
            _ => SearchAction::Stay,
        }
    }

    /// A left click at `(col, row)`: toggle a button if it landed on one.
    /// `true` means the query changed and matches need recomputing.
    pub fn click(&mut self, col: u16, row: u16) -> bool {
        let hit =
            |r: Option<Rect>| r.is_some_and(|r| r.x <= col && col < r.x + r.width && r.y == row);
        if hit(self.case_rect) {
            self.case_sensitive = !self.case_sensitive;
            true
        } else if hit(self.regex_rect) {
            self.regex = !self.regex;
            true
        } else {
            false
        }
    }
}

/// Every non-overlapping match of `needle` in `buf`, as ordered `(start, end)`
/// char positions — a literal substring, or (if `regex`) a pattern. An
/// invalid regex yields no matches (the bar shows the compiler's error
/// instead, via `Search::regex_error`). Case-insensitive substring matching
/// folds ASCII only (a code editor is mostly ASCII, and folding non-ASCII can
/// change char counts and so the offsets).
pub fn find_all(
    buf: &Buffer,
    needle: &str,
    case_sensitive: bool,
    regex: bool,
) -> Vec<(Position, Position)> {
    let Ok(matcher) = Matcher::compile(needle, case_sensitive, regex) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in 0..buf.line_count() {
        let text = buf.line_text(line);
        for (start, end) in matcher.find_in_line(&text) {
            out.push((Position { line, col: start }, Position { line, col: end }));
        }
    }
    out
}

/// Draw the bar. `info` is `(current 1-based, total)` — `current` is 0 when
/// there are no matches. Records the `[Aa]` / `[.*]` button rects onto `s`
/// for the app's mouse handler.
pub fn render(f: &mut Frame, s: &mut Search, theme: &Theme, info: (usize, usize), area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    f.render_widget(
        Block::default().style(Style::default().bg(theme.statusbar_bg)),
        area,
    );

    let dim = Style::default()
        .fg(theme.statusbar_fg)
        .bg(theme.statusbar_bg);
    let text = Style::default().fg(theme.fg).bg(theme.statusbar_bg);
    let accent = Style::default()
        .fg(theme.accent)
        .bg(theme.statusbar_bg)
        .add_modifier(Modifier::BOLD);
    let label = |on: bool| if on { accent } else { dim };

    let (cur, total) = info;
    let error = s.regex_error();
    let counter = if let Some(err) = &error {
        // The compiler's message can be long (and regex errors are often
        // multi-line) — keep it to one line so it can't push the buttons
        // off the right edge of a narrow terminal.
        format!(" regex error: {} ", err.lines().next().unwrap_or(err))
    } else if total > 0 {
        format!(" {cur}/{total} ")
    } else if s.query().is_empty() {
        String::new()
    } else {
        " no matches ".to_string()
    };

    // Buttons: same toggle as Alt+C / Alt+X, clickable for mouse users.
    // "[Aa]" (case) and "[.*]" (regex) — ASCII, not the ⌥/⌘ symbol glyphs
    // that a lot of terminal fonts render as a blank box. Fixed at the left
    // edge (not after the counter) so they can't be pushed off-screen by a
    // long "no matches" / regex-error message.
    let case_text = "[Aa]";
    let regex_text = "[.*]";
    let mut x = area.x;
    let case_rect = Rect {
        x,
        y: area.y + 2,
        width: case_text.chars().count() as u16,
        height: 1,
    };
    x += case_rect.width + 1;
    let regex_rect = Rect {
        x,
        y: area.y + 2,
        width: regex_text.chars().count() as u16,
        height: 1,
    };
    s.case_rect = Some(case_rect);
    s.regex_rect = Some(regex_rect);

    let find_focused = s.field == Field::Find;
    let rows = vec![
        Line::from(vec![
            Span::styled(
                if find_focused { "▸ Find " } else { "  Find " },
                label(find_focused),
            ),
            Span::styled(s.query(), text),
        ]),
        Line::from(vec![
            Span::styled(
                if find_focused { "  Repl " } else { "▸ Repl " },
                label(!find_focused),
            ),
            Span::styled(s.replacement(), text),
        ]),
        Line::from(vec![
            Span::styled(case_text, label(s.case_sensitive)),
            Span::styled(" ", dim),
            Span::styled(regex_text, label(s.regex)),
            Span::styled(
                counter,
                if error.is_some() {
                    Style::default()
                        .fg(theme.output_err)
                        .bg(theme.statusbar_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    accent
                },
            ),
            Span::styled(
                "  Enter next · Shift+Enter prev · Tab · ^R replace · Alt+A all · Esc",
                dim,
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(rows), area);

    // Caret in the focused field.
    let col = match s.field {
        Field::Find => s.find.cursor().col,
        Field::Replace => s.replace.cursor().col,
    } as u16;
    let cy = area.y + u16::from(!find_focused);
    let cx = area.x + LABEL_W + col;
    if cx < area.x + area.width {
        f.set_cursor_position(TermPos::new(cx, cy));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_all_char_positions() {
        let b = Buffer::from_str("alpha beta alpha\ngamma alpha");
        let m = find_all(&b, "alpha", false, false);
        assert_eq!(
            m,
            vec![
                (Position { line: 0, col: 0 }, Position { line: 0, col: 5 }),
                (Position { line: 0, col: 11 }, Position { line: 0, col: 16 }),
                (Position { line: 1, col: 6 }, Position { line: 1, col: 11 }),
            ]
        );
    }

    #[test]
    fn find_all_aligns_after_non_ascii() {
        // 'é' is one char but two bytes; a byte scan would report col 13/17.
        let b = Buffer::from_str("café résumé café");
        let m = find_all(&b, "café", false, false);
        assert_eq!(
            m,
            vec![
                (Position { line: 0, col: 0 }, Position { line: 0, col: 4 }),
                (Position { line: 0, col: 12 }, Position { line: 0, col: 16 }),
            ]
        );
    }

    #[test]
    fn find_all_case_fold_and_sensitivity() {
        let b = Buffer::from_str("Foo foo FOO");
        assert_eq!(find_all(&b, "foo", false, false).len(), 3);
        assert_eq!(find_all(&b, "foo", true, false).len(), 1);
    }

    #[test]
    fn find_all_is_non_overlapping() {
        let b = Buffer::from_str("aaaa");
        assert_eq!(find_all(&b, "aa", false, false).len(), 2);
    }

    #[test]
    fn find_all_regex_mode() {
        let b = Buffer::from_str("a12 b345\nc6");
        let m = find_all(&b, r"\d+", false, true);
        assert_eq!(
            m,
            vec![
                (Position { line: 0, col: 1 }, Position { line: 0, col: 3 }),
                (Position { line: 0, col: 5 }, Position { line: 0, col: 8 }),
                (Position { line: 1, col: 1 }, Position { line: 1, col: 2 }),
            ]
        );
    }

    #[test]
    fn find_all_invalid_regex_yields_no_matches() {
        let b = Buffer::from_str("anything");
        assert!(find_all(&b, "(unclosed", false, true).is_empty());
    }

    #[test]
    fn regex_error_reports_the_compiler_message_only_in_regex_mode() {
        let mut s = Search::new("(unclosed");
        assert!(s.regex_error().is_none(), "regex mode is off by default");
        s.regex = true;
        assert!(s.regex_error().is_some());
        s.find = Buffer::from_str(r"\d+");
        assert!(s.regex_error().is_none(), "a valid pattern has no error");
    }
}
