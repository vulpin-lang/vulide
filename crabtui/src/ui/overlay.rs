//! The modal overlay layer.
//!
//! Members: the **path prompt** (Save As / Open File — `Ctrl+S` on an untitled
//! buffer, `Ctrl+O`), the **command palette** (`Ctrl+P`, in `palette.rs`), and
//! the **theme picker** (`Ctrl+T`, in `theme_picker.rs`). While an overlay is
//! open it captures all key input; `centered_rect` is the shared geometry helper.

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Position as TermPos, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph};

use super::help::Help;
use super::palette::Palette;
use super::theme_picker::ThemePicker;
use crate::buffer::Buffer;
use crate::theme::Theme;

pub enum Overlay {
    None,
    Prompt(Box<PathPrompt>),
    Palette(Box<Palette>),
    ThemePicker(Box<ThemePicker>),
    Help(Box<Help>),
}

impl Overlay {
    pub fn is_open(&self) -> bool {
        !matches!(self, Overlay::None)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    Save,
    Open,
}

/// What the app should do with a key the path prompt just consumed.
pub enum PromptOutcome {
    /// Stay open, keep editing the path.
    Stay,
    /// User cancelled.
    Cancel,
    /// User confirmed this (already whitespace-trimmed) path.
    Submit(String),
}

pub struct PathPrompt {
    /// Single-line path editor — a one-line `Buffer` so it reuses the tested
    /// insert/delete/movement code.
    input: Buffer,
    pub error: Option<String>,
    pub kind: PromptKind,
}

impl PathPrompt {
    pub fn save(seed: &str) -> Self {
        Self::seeded(seed, PromptKind::Save)
    }

    pub fn open(seed: &str) -> Self {
        Self::seeded(seed, PromptKind::Open)
    }

    fn seeded(seed: &str, kind: PromptKind) -> Self {
        let mut input = Buffer::from_str(seed);
        input.move_doc_end(false);
        Self {
            input,
            error: None,
            kind,
        }
    }

    pub fn path(&self) -> String {
        self.input.rope().to_string()
    }

    pub fn title(&self) -> &'static str {
        match self.kind {
            PromptKind::Save => " Save As ",
            PromptKind::Open => " Open File ",
        }
    }

    fn hint(&self) -> &'static str {
        match self.kind {
            PromptKind::Save => "      Enter save · Esc cancel",
            PromptKind::Open => "      Enter open · Esc cancel",
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> PromptOutcome {
        let plain = !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        match key.code {
            KeyCode::Esc => PromptOutcome::Cancel,
            KeyCode::Enter => PromptOutcome::Submit(self.path().trim().to_string()),
            KeyCode::Backspace => {
                self.input.delete_backward();
                PromptOutcome::Stay
            }
            KeyCode::Delete => {
                self.input.delete_forward();
                PromptOutcome::Stay
            }
            KeyCode::Left => {
                self.input.move_left(false);
                PromptOutcome::Stay
            }
            KeyCode::Right => {
                self.input.move_right(false);
                PromptOutcome::Stay
            }
            KeyCode::Home => {
                self.input.move_home(false);
                PromptOutcome::Stay
            }
            KeyCode::End => {
                self.input.move_end(false);
                PromptOutcome::Stay
            }
            KeyCode::Char(c) if plain && c != '\n' => {
                self.input.insert_char(c);
                self.error = None;
                PromptOutcome::Stay
            }
            _ => PromptOutcome::Stay,
        }
    }
}

/// Expand a leading `~` / `~/` to `$HOME` (Linux target — good enough for now).
pub fn expand_tilde(path: &str) -> String {
    if (path == "~" || path.starts_with("~/"))
        && let Ok(home) = std::env::var("HOME")
    {
        return format!("{home}{}", &path[1..]);
    }
    path.to_string()
}

pub fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    }
}

/// Returns the outer rect it drew into (for click-away hit-testing).
pub fn render_prompt(f: &mut Frame, prompt: &PathPrompt, theme: &Theme, area: Rect) -> Rect {
    let extra = if prompt.error.is_some() { 2 } else { 0 };
    let rect = centered_rect(64, 7 + extra, area);
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
        .title(Span::styled(prompt.title(), accent))
        .style(panel);
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let label = "Path: ";
    let mut lines = vec![
        Line::from(vec![
            Span::styled(label, muted),
            Span::styled(prompt.path(), panel),
        ]),
        Line::default(),
    ];
    if let Some(err) = &prompt.error {
        lines.push(Line::from(Span::styled(
            err.clone(),
            Style::default()
                .fg(theme.output_err)
                .bg(theme.statusbar_bg)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::default());
    }
    let verb = if prompt.kind == PromptKind::Save {
        "[ Save ]"
    } else {
        "[ Open ]"
    };
    lines.push(Line::from(vec![
        Span::styled("[ Cancel ]", muted),
        Span::styled("   ", panel),
        Span::styled(verb, accent),
        Span::styled(prompt.hint(), muted),
    ]));
    f.render_widget(Paragraph::new(lines).style(panel), inner);

    // caret in the path field (first inner row)
    let cx = inner.x + label.len() as u16 + prompt.input.cursor().col as u16;
    if cx < inner.x + inner.width {
        f.set_cursor_position(TermPos::new(cx, inner.y));
    }
    rect
}
