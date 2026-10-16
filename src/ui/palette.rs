//! `Ctrl+P` command palette: a fuzzy-filtered list of actions.
//!
//! `App` builds the [`Entry`] list when the palette opens (so it can fold in
//! live state — the theme list, recent files) and matches on the returned
//! [`Cmd`]. Keeping the command set an enum means the palette has no callbacks
//! and stays a pure widget.

use std::path::PathBuf;

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Position as TermPos, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph};

use super::overlay::centered_rect;
use crate::buffer::Buffer;
use crate::theme::Theme;

const MAX_ROWS: usize = 12;

#[derive(Clone, Debug)]
pub enum Cmd {
    Quit,
    Save,
    SaveAs,
    OpenFile,
    NewTab,
    CloseTab,
    CloseTabDiscard,
    NextTab,
    PrevTab,
    ChooseTheme,
    SetTheme(String),
    ToggleLineNumbers,
    ToggleWordWrap,
    ToggleAutoClose,
    ToggleOutline,
    ToggleFileTree,
    ToggleSessionRestore,
    ToggleMouse,
    ReloadConfig,
    OpenRecent(PathBuf),
    InsertSnippet(&'static str),
    RunFile,
    StopRun,
    CloseOutput,
    Find,
    Replace,
    FindInFiles,
    GotoLine,
    Projects,
    Help,
}

pub struct Entry {
    pub label: String,
    pub cmd: Cmd,
}

impl Entry {
    pub fn new(label: impl Into<String>, cmd: Cmd) -> Self {
        Self {
            label: label.into(),
            cmd,
        }
    }
}

pub enum PaletteOutcome {
    Stay,
    Cancel,
    Run(Cmd),
}

pub struct Palette {
    input: Buffer,
    entries: Vec<Entry>,
    /// Indices into `entries` that match the query, best first.
    filtered: Vec<usize>,
    selected: usize,
}

impl Palette {
    pub fn new(entries: Vec<Entry>) -> Self {
        let filtered = (0..entries.len()).collect();
        Self {
            input: Buffer::new(),
            entries,
            filtered,
            selected: 0,
        }
    }

    pub fn query(&self) -> String {
        self.input.rope().to_string()
    }

    fn refilter(&mut self) {
        let q = self.query().to_lowercase();
        let mut scored: Vec<(i32, usize)> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| subseq_score(&q, &e.label.to_lowercase()).map(|s| (s, i)))
            .collect();
        // higher score first, stable on original order for ties
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        self.filtered = scored.into_iter().map(|(_, i)| i).collect();
        self.selected = self.selected.min(self.filtered.len().saturating_sub(1));
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> PaletteOutcome {
        let plain = !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        match key.code {
            KeyCode::Esc => PaletteOutcome::Cancel,
            KeyCode::Enter => match self.filtered.get(self.selected) {
                Some(&i) => PaletteOutcome::Run(self.entries[i].cmd.clone()),
                None => PaletteOutcome::Stay,
            },
            KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                PaletteOutcome::Stay
            }
            KeyCode::Down => {
                if self.selected + 1 < self.filtered.len() {
                    self.selected += 1;
                }
                PaletteOutcome::Stay
            }
            KeyCode::Backspace => {
                self.input.delete_backward();
                self.refilter();
                PaletteOutcome::Stay
            }
            KeyCode::Char(c) if plain => {
                self.input.insert_char(c);
                self.refilter();
                PaletteOutcome::Stay
            }
            _ => PaletteOutcome::Stay,
        }
    }

    /// Row-hit test against the list, given the box `render` last drew into.
    /// One border row plus the `> query` line sit above the list.
    fn row_at(&self, outer: Rect, row: u16) -> Option<usize> {
        let body_top = outer.y + 2;
        if row < body_top {
            return None;
        }
        let top = self.selected.saturating_sub(MAX_ROWS - 1);
        let idx = top + (row - body_top) as usize;
        (idx < self.filtered.len()).then_some(idx)
    }

    /// A left click at `row`: select and immediately run that entry, same as
    /// pressing Enter on it.
    pub fn click(&mut self, outer: Rect, row: u16) -> PaletteOutcome {
        match self.row_at(outer, row) {
            Some(idx) => {
                self.selected = idx;
                PaletteOutcome::Run(self.entries[self.filtered[idx]].cmd.clone())
            }
            None => PaletteOutcome::Stay,
        }
    }

    /// Mouse wheel: move the selection by one row.
    pub fn scroll(&mut self, delta: isize) {
        if self.filtered.is_empty() {
            return;
        }
        let n = self.filtered.len() as isize;
        self.selected = (self.selected as isize + delta).clamp(0, n - 1) as usize;
    }
}

/// Subsequence match with a light score (contiguous runs and word-starts win).
/// `None` if `needle` is not a subsequence of `haystack`.
fn subseq_score(needle: &str, haystack: &str) -> Option<i32> {
    if needle.is_empty() {
        return Some(0);
    }
    let hay: Vec<char> = haystack.chars().collect();
    let mut score = 0i32;
    let mut hi = 0usize;
    let mut prev_match = None::<usize>;
    for nc in needle.chars() {
        loop {
            if hi >= hay.len() {
                return None;
            }
            if hay[hi] == nc {
                if prev_match == Some(hi.wrapping_sub(1)) {
                    score += 3; // contiguous
                }
                if hi == 0 || !hay[hi - 1].is_alphanumeric() {
                    score += 2; // word start
                }
                prev_match = Some(hi);
                hi += 1;
                break;
            }
            hi += 1;
        }
    }
    Some(score)
}

/// Returns the outer rect it drew into (for click-away hit-testing).
pub fn render(f: &mut Frame, p: &Palette, theme: &Theme, area: Rect) -> Rect {
    let rows = p.filtered.len().clamp(1, MAX_ROWS) as u16;
    let rect = centered_rect(72, rows + 4, area);
    f.render_widget(Clear, rect);

    let panel = Style::default().fg(theme.menu_fg).bg(theme.menu_bg);
    let accent = Style::default()
        .fg(theme.accent)
        .bg(theme.menu_bg)
        .add_modifier(Modifier::BOLD);
    let block = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::symmetric(1, 0))
        .border_style(Style::default().fg(theme.accent).bg(theme.menu_bg))
        .title(Span::styled(" Commands ", accent))
        .style(panel);
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let query = p.query();
    let mut lines = vec![Line::from(vec![
        Span::styled("> ", accent),
        Span::styled(query.clone(), panel),
    ])];

    let top = p.selected.saturating_sub(MAX_ROWS - 1);
    for (row, &idx) in p.filtered.iter().enumerate().skip(top).take(MAX_ROWS) {
        let selected = row == p.selected;
        let style = if selected {
            Style::default()
                .fg(theme.autocomplete_fg)
                .bg(theme.autocomplete_sel)
                .add_modifier(Modifier::BOLD)
        } else {
            panel
        };
        let marker = if selected { "▸ " } else { "  " };
        // Truncate rather than wrap: each entry is one row, which is what
        // keeps click / scroll row-hit-testing lined up with `p.filtered`.
        let budget = (inner.width as usize).saturating_sub(marker.chars().count());
        let label = &p.entries[idx].label;
        let text = if label.chars().count() > budget && budget > 1 {
            let head: String = label.chars().take(budget - 1).collect();
            format!("{head}…")
        } else {
            label.clone()
        };
        lines.push(Line::from(Span::styled(format!("{marker}{text}"), style)));
    }
    if p.filtered.is_empty() {
        lines.push(Line::from(Span::styled("  (no matches)", panel)));
    }

    f.render_widget(Paragraph::new(lines).style(panel), inner);
    super::sidebar_scrollbar(f, theme, rect, p.filtered.len(), top);
    f.set_cursor_position(TermPos::new(
        inner.x + 2 + query.chars().count() as u16,
        inner.y,
    ));
    rect
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subsequence_filters_and_scores() {
        assert!(subseq_score("nt", "next tab").is_some());
        assert!(subseq_score("xyz", "next tab").is_none());
        // "nt" as two word-starts should beat a scattered match
        let a = subseq_score("nt", "next tab").unwrap();
        let b = subseq_score("nt", "annotate").unwrap();
        assert!(a > b, "word-start match {a} should beat scattered {b}");
    }
}
