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
//!   Alt+C                toggle case sensitivity (ASCII fold; off by default)
//!   Esc                  close the bar
//!
//! Match positions are `(start, end)` char indices — `Position::col` is a char
//! index, so `find_all` walks each line as `Vec<char>` (a byte scan would
//! misalign on a line with a non-ASCII glyph before the match).

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Position as TermPos, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::buffer::{Buffer, Position};
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
        }
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
}

/// Every non-overlapping match of `needle` in `buf`, as ordered `(start, end)`
/// char positions. Case-insensitive folds ASCII only (a code editor is mostly
/// ASCII, and folding non-ASCII can change char counts and so the offsets).
pub fn find_all(buf: &Buffer, needle: &str, case_sensitive: bool) -> Vec<(Position, Position)> {
    let needle: Vec<char> = needle.chars().collect();
    if needle.is_empty() {
        return Vec::new();
    }
    let eq = |a: char, b: char| {
        if case_sensitive {
            a == b
        } else {
            a.eq_ignore_ascii_case(&b)
        }
    };

    let mut out = Vec::new();
    for line in 0..buf.line_count() {
        let chars: Vec<char> = buf.line_text(line).chars().collect();
        let mut i = 0;
        while i + needle.len() <= chars.len() {
            if chars[i..i + needle.len()]
                .iter()
                .zip(&needle)
                .all(|(&a, &b)| eq(a, b))
            {
                out.push((
                    Position { line, col: i },
                    Position {
                        line,
                        col: i + needle.len(),
                    },
                ));
                i += needle.len();
            } else {
                i += 1;
            }
        }
    }
    out
}

/// Draw the bar. `info` is `(current 1-based, total)` — `current` is 0 when
/// there are no matches.
pub fn render(f: &mut Frame, s: &Search, theme: &Theme, info: (usize, usize), area: Rect) {
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
    let counter = if total > 0 {
        format!(" {cur}/{total} ")
    } else if s.query().is_empty() {
        String::new()
    } else {
        " no matches ".to_string()
    };
    let case = if s.case_sensitive {
        " case: on "
    } else {
        " case: off "
    };

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
            Span::styled(counter, accent),
            Span::styled(case, label(s.case_sensitive)),
            Span::styled(
                "  ↵ next · ⇧↵ prev · Tab field · ^R replace · ⌥A all · ⌥C case · Esc",
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
        let m = find_all(&b, "alpha", false);
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
        let m = find_all(&b, "café", false);
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
        assert_eq!(find_all(&b, "foo", false).len(), 3);
        assert_eq!(find_all(&b, "foo", true).len(), 1);
    }

    #[test]
    fn find_all_is_non_overlapping() {
        let b = Buffer::from_str("aaaa");
        assert_eq!(find_all(&b, "aa", false).len(), 2);
    }
}
