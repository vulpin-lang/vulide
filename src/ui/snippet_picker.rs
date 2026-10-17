//! The snippet picker (`F3`): a two-pane list + live syntax-highlighted
//! preview, rather than burying snippets in the general command palette.

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph};

use super::overlay::centered_rect;
use crate::snippets::SNIPPETS;
use crate::syntax::Language;
use crate::theme::Theme;

#[derive(Default)]
pub struct SnippetPicker {
    pub selected: usize,
}

pub enum SnippetOutcome {
    Stay,
    Cancel,
    Insert(&'static str),
}

impl SnippetPicker {
    pub fn handle_key(&mut self, key: KeyEvent) -> SnippetOutcome {
        match key.code {
            KeyCode::Esc => SnippetOutcome::Cancel,
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
                SnippetOutcome::Stay
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < SNIPPETS.len() {
                    self.selected += 1;
                }
                SnippetOutcome::Stay
            }
            KeyCode::Home => {
                self.selected = 0;
                SnippetOutcome::Stay
            }
            KeyCode::End => {
                self.selected = SNIPPETS.len().saturating_sub(1);
                SnippetOutcome::Stay
            }
            KeyCode::Enter => SnippetOutcome::Insert(SNIPPETS[self.selected].body),
            _ => SnippetOutcome::Stay,
        }
    }

    /// Row-hit test against the name list (not the preview beside it) — one
    /// border row sits above it.
    fn row_at(&self, outer: Rect, col: u16, row: u16) -> Option<usize> {
        let list_x = outer.x + 1;
        if col < list_x || col >= list_x + LIST_WIDTH {
            return None;
        }
        let rows = list_rows(outer);
        let off = super::visible_row(outer, outer.y + 1, row, rows)?;
        let idx = list_top(self.selected, rows) + off;
        (idx < SNIPPETS.len()).then_some(idx)
    }

    /// A left click at `(col, row)`: select and immediately insert, same as Enter.
    pub fn click(&mut self, outer: Rect, col: u16, row: u16) -> SnippetOutcome {
        match self.row_at(outer, col, row) {
            Some(idx) => {
                self.selected = idx;
                SnippetOutcome::Insert(SNIPPETS[idx].body)
            }
            None => SnippetOutcome::Stay,
        }
    }

    /// Mouse wheel: move the selection by one row.
    pub fn scroll(&mut self, delta: isize) {
        let n = SNIPPETS.len() as isize;
        self.selected = (self.selected as isize + delta).clamp(0, n - 1) as usize;
    }
}

/// Width of the name list; the preview takes the rest.
const LIST_WIDTH: u16 = 20;

/// Name rows that fit inside the box's borders.
fn list_rows(outer: Rect) -> usize {
    outer.height.saturating_sub(2) as usize
}

/// First name shown: scrolled just enough to keep the selection in view.
fn list_top(selected: usize, rows: usize) -> usize {
    selected.saturating_sub(rows.max(1) - 1)
}

/// Returns the outer rect it drew into (for click-away hit-testing).
pub fn render(f: &mut Frame, p: &SnippetPicker, theme: &Theme, area: Rect) -> Rect {
    // Tall enough for every snippet name when the terminal allows; it scrolls
    // when it doesn't.
    let rect = centered_rect(
        72,
        (SNIPPETS.len() as u16 + 2).max(10).min(area.height),
        area,
    );
    f.render_widget(Clear, rect);

    let panel = Style::default().fg(theme.menu_fg).bg(theme.menu_bg);
    let accent = Style::default()
        .fg(theme.accent)
        .bg(theme.menu_bg)
        .add_modifier(Modifier::BOLD);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent).bg(theme.menu_bg))
        .title(Span::styled(
            " Snippets — ↑↓ choose, Enter insert, Esc cancel ",
            accent,
        ))
        .style(panel);
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let cols =
        Layout::horizontal([Constraint::Length(LIST_WIDTH), Constraint::Min(24)]).split(inner);

    let rows = list_rows(rect);
    let list: Vec<Line> = SNIPPETS
        .iter()
        .enumerate()
        .skip(list_top(p.selected, rows))
        .take(rows)
        .map(|(i, s)| {
            let selected = i == p.selected;
            let style = if selected {
                Style::default()
                    .fg(theme.autocomplete_fg)
                    .bg(theme.autocomplete_sel)
                    .add_modifier(Modifier::BOLD)
            } else {
                panel
            };
            let marker = if selected { "▸ " } else { "  " };
            let mut line = format!("{marker}{}", s.name);
            while line.chars().count() < cols[0].width as usize {
                line.push(' ');
            }
            Line::from(Span::styled(line, style))
        })
        .collect();
    f.render_widget(Paragraph::new(list).style(panel), cols[0]);

    let preview_block = Block::default()
        .borders(Borders::LEFT)
        .padding(Padding::horizontal(1))
        .border_style(Style::default().fg(theme.dock_fg).bg(theme.menu_bg))
        .style(panel);
    let preview_area = preview_block.inner(cols[1]);
    f.render_widget(preview_block, cols[1]);

    let body = SNIPPETS[p.selected].body;
    let preview: Vec<Line> = body
        .lines()
        .map(|line| {
            let tokens = Language::Vulpin.tokenize_line(line);
            let spans: Vec<Span> = line
                .chars()
                .enumerate()
                .map(|(i, ch)| {
                    let color = tokens
                        .iter()
                        .find(|t| i >= t.start && i < t.end)
                        .and_then(|t| theme.token_color(t.kind))
                        .unwrap_or(theme.fg);
                    Span::styled(ch.to_string(), Style::default().fg(color).bg(theme.menu_bg))
                })
                .collect();
            Line::from(spans)
        })
        .collect();
    f.render_widget(Paragraph::new(preview), preview_area);

    rect
}
