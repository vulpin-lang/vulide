//! `Ctrl+G`: jump the cursor to a 1-based line number.

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Position as TermPos, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph};

use super::overlay::centered_rect;
use crate::buffer::Buffer;
use crate::theme::Theme;

pub enum GotoOutcome {
    Stay,
    Cancel,
    /// A 1-based line number the caller should jump to.
    Submit(usize),
}

pub struct GotoLine {
    /// Digits-only single-line editor — reuses the tested Buffer insert/delete.
    input: Buffer,
    pub total_lines: usize,
    pub error: Option<String>,
}

impl GotoLine {
    pub fn new(total_lines: usize) -> Self {
        Self {
            input: Buffer::new(),
            total_lines,
            error: None,
        }
    }

    pub fn text(&self) -> String {
        self.input.rope().to_string()
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> GotoOutcome {
        let plain = !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        match key.code {
            KeyCode::Esc => GotoOutcome::Cancel,
            KeyCode::Enter => match self.text().trim().parse::<usize>() {
                Ok(n) if n >= 1 => GotoOutcome::Submit(n),
                _ => {
                    self.error = Some("enter a line number".to_string());
                    GotoOutcome::Stay
                }
            },
            KeyCode::Backspace => {
                self.input.delete_backward();
                self.error = None;
                GotoOutcome::Stay
            }
            KeyCode::Delete => {
                self.input.delete_forward();
                self.error = None;
                GotoOutcome::Stay
            }
            KeyCode::Left => {
                self.input.move_left(false);
                GotoOutcome::Stay
            }
            KeyCode::Right => {
                self.input.move_right(false);
                GotoOutcome::Stay
            }
            KeyCode::Home => {
                self.input.move_home(false);
                GotoOutcome::Stay
            }
            KeyCode::End => {
                self.input.move_end(false);
                GotoOutcome::Stay
            }
            // Line numbers only — no point letting letters into the field.
            KeyCode::Char(c) if plain && c.is_ascii_digit() => {
                self.input.insert_char(c);
                self.error = None;
                GotoOutcome::Stay
            }
            _ => GotoOutcome::Stay,
        }
    }
}

/// Returns the outer rect it drew into (for click-away hit-testing).
pub fn render(f: &mut Frame, g: &GotoLine, theme: &Theme, area: Rect) -> Rect {
    let extra = if g.error.is_some() { 2 } else { 0 };
    let rect = centered_rect(40, 6 + extra, area);
    f.render_widget(Clear, rect);

    let panel = Style::default().fg(theme.fg).bg(theme.statusbar_bg);
    let muted = Style::default()
        .fg(theme.statusbar_fg)
        .bg(theme.statusbar_bg);
    let accent = Style::default()
        .fg(theme.accent)
        .bg(theme.statusbar_bg)
        .add_modifier(Modifier::BOLD);
    let block = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::symmetric(2, 1))
        .border_style(Style::default().fg(theme.accent).bg(theme.statusbar_bg))
        .title(Span::styled(" Go to Line ", accent))
        .style(panel);
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let label = format!("Line (1-{}): ", g.total_lines);
    let mut lines = vec![Line::from(vec![
        Span::styled(label.clone(), muted),
        Span::styled(g.text(), panel),
    ])];
    if let Some(err) = &g.error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            err.clone(),
            Style::default()
                .fg(theme.output_err)
                .bg(theme.statusbar_bg)
                .add_modifier(Modifier::BOLD),
        )));
    }
    lines.push(Line::default());
    lines.push(Line::from(Span::styled("Enter go · Esc cancel", muted)));
    f.render_widget(Paragraph::new(lines).style(panel), inner);

    let cx = inner.x + label.chars().count() as u16 + g.text().chars().count() as u16;
    if cx < inner.x + inner.width {
        f.set_cursor_position(TermPos::new(cx, inner.y));
    }
    rect
}
