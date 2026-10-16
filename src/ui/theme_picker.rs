//! `Ctrl+T` theme picker: a short list of the bundled themes with live preview.
//!
//! Arrow keys preview the highlighted theme on the real editor behind the
//! overlay; Enter keeps it (and persists to config), Esc reverts to the theme
//! that was active when the picker opened. `App` owns the preview swap — the
//! widget just tracks the selection and reports what to do.

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph};

use super::overlay::centered_rect;
use crate::theme::Theme;

pub enum ThemePickerOutcome {
    /// Show theme `usize` on the editor behind the overlay.
    Preview(usize),
    /// Keep theme `usize` and close.
    Commit(usize),
    /// Restore the theme from when the picker opened and close.
    Cancel,
}

pub struct ThemePicker {
    pub names: Vec<String>,
    pub selected: usize,
    pub original: usize,
}

impl ThemePicker {
    pub fn new(names: Vec<String>, current: usize) -> Self {
        Self {
            names,
            selected: current,
            original: current,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> ThemePickerOutcome {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.checked_sub(1).unwrap_or(self.names.len() - 1);
                ThemePickerOutcome::Preview(self.selected)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected = (self.selected + 1) % self.names.len();
                ThemePickerOutcome::Preview(self.selected)
            }
            KeyCode::Enter => ThemePickerOutcome::Commit(self.selected),
            KeyCode::Esc => ThemePickerOutcome::Cancel,
            _ => ThemePickerOutcome::Preview(self.selected),
        }
    }
}

/// Returns the outer rect it drew into (for click-away hit-testing).
pub fn render(f: &mut Frame, picker: &ThemePicker, theme: &Theme, area: Rect) -> Rect {
    let rect = centered_rect(44, picker.names.len() as u16 + 4, area);
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
        .title(Span::styled(" Theme ", accent))
        .style(panel);
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let mut lines: Vec<Line> = picker
        .names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let selected = i == picker.selected;
            let style = if selected {
                Style::default()
                    .fg(theme.autocomplete_fg)
                    .bg(theme.autocomplete_sel)
                    .add_modifier(Modifier::BOLD)
            } else {
                panel
            };
            let marker = if selected { "▸ " } else { "  " };
            Line::from(Span::styled(format!("{marker}{name}"), style))
        })
        .collect();
    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        "↑↓ preview · Enter keep · Esc cancel",
        Style::default().fg(theme.statusbar_fg).bg(theme.menu_bg),
    )));

    f.render_widget(Paragraph::new(lines).style(panel), inner);
    rect
}
