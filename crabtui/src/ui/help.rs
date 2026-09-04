//! The keys-and-shortcuts overlay (`F1`, or the palette's "Help" entry).
//!
//! A static reference card — grouped key/description pairs, scrollable if the
//! terminal is short. Any of Esc / Enter / F1 / `q` closes it.

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph};

use super::overlay::centered_rect;
use crate::theme::Theme;

/// `(section, [(keys, what)])` — the whole card.
pub const CARD: &[(&str, &[(&str, &str)])] = &[
    (
        "Files & tabs",
        &[
            ("Ctrl+S", "save (Save As if untitled)"),
            ("Ctrl+O", "open a file"),
            ("Ctrl+N", "new tab"),
            ("Ctrl+W", "close tab (refuses if unsaved)"),
            ("Ctrl+PgUp / PgDn", "previous / next tab"),
        ],
    ),
    (
        "Editing",
        &[
            ("Ctrl+Z / Ctrl+Y", "undo / redo"),
            ("Ctrl+A", "select all"),
            (
                "Ctrl+F",
                "find / replace bar (^R replace, Alt+A all, Esc close)",
            ),
            ("Tab / Shift+Tab", "indent / dedent"),
            ("Ctrl+←/→", "word-wise motion"),
            (
                "autocomplete",
                "$vars · functions · .U/.L/.S/.T/.C · command hints — Enter/Tab accepts",
            ),
        ],
    ),
    (
        "Vulpin commands",
        &[
            ("G / P", "print — with / without a newline"),
            ("K  Q  X", "read input · quit · raise error"),
            ("? : ;", "if · else · end if"),
            ("@ &   O", "while · end while   ·   for i start end [step]"),
            ("F ~ R", "function · end function · return"),
            ("L  J", "label · jump to label"),
            ("W V N Z", "switch · case · default · end switch"),
            ("T C Y", "try · catch · end try"),
            ("E  A  S", "assign · in-place arithmetic · string replace"),
            ("U   $x", "import module   ·   $x dereferences a variable"),
        ],
    ),
    (
        "Run & output",
        &[
            ("F5  /  ▶ button", "run the current file"),
            ("F6", "toggle focus editor <-> output"),
            ("Enter (in panel)", "send the line to the program's stdin"),
            ("Ctrl+D (in panel)", "close stdin (EOF)"),
            ("Ctrl+C (in panel)", "stop the running program"),
            ("↑/↓ PgUp/PgDn Home/End", "scroll the output"),
        ],
    ),
    (
        "View & commands",
        &[
            ("Ctrl+P", "command palette"),
            ("Ctrl+T", "theme picker (live preview)"),
            (
                "palette › Word Wrap",
                "wrap long lines instead of scrolling",
            ),
            (
                "F2",
                "file tree — ↑↓ select, → / ← expand / collapse, Enter opens, r refresh, . hidden",
            ),
            (
                "F7",
                "structure outline — ↑↓ select, Enter jumps to the line",
            ),
            ("F1  /  Ctrl+H", "this help"),
            ("Esc", "dismiss popup / overlay / leave the output panel"),
            (
                "Ctrl+Q / Ctrl+C",
                "quit (asks first if a buffer is unsaved)",
            ),
        ],
    ),
    (
        "Mouse",
        &[
            ("click tab / its ✕", "switch to it / close it"),
            ("drag the ╍╍ divider", "resize the output panel"),
            ("panel ✕", "close the output panel"),
            ("click a pane", "move focus there"),
            ("click outside a popup", "dismiss it"),
            ("Shift + drag", "the terminal's own text selection"),
        ],
    ),
];

pub enum HelpOutcome {
    Stay,
    Close,
}

#[derive(Default)]
pub struct Help {
    pub scroll: usize,
}

impl Help {
    pub fn handle_key(&mut self, key: KeyEvent) -> HelpOutcome {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::F(1) | KeyCode::Char('q') => {
                HelpOutcome::Close
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll = self.scroll.saturating_sub(1);
                HelpOutcome::Stay
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll += 1;
                HelpOutcome::Stay
            }
            _ => HelpOutcome::Stay,
        }
    }
}

/// Every rendered line of the card, flattened (for scroll math).
fn lines(theme: &Theme) -> Vec<Line<'static>> {
    let head = Style::default()
        .fg(theme.accent)
        .bg(theme.menu_bg)
        .add_modifier(Modifier::BOLD);
    let key = Style::default()
        .fg(theme.keyword)
        .bg(theme.menu_bg)
        .add_modifier(Modifier::BOLD);
    let desc = Style::default().fg(theme.menu_fg).bg(theme.menu_bg);

    let mut out = Vec::new();
    for (i, (section, rows)) in CARD.iter().enumerate() {
        if i > 0 {
            out.push(Line::default());
        }
        out.push(Line::from(Span::styled(section.to_string(), head)));
        for (k, what) in *rows {
            out.push(Line::from(vec![
                Span::styled(format!("  {k:<24}"), key),
                Span::styled((*what).to_string(), desc),
            ]));
        }
    }
    out
}

/// Returns the outer rect it drew into (for click-away hit-testing).
pub fn render(f: &mut Frame, help: &Help, theme: &Theme, area: Rect) -> Rect {
    let all = lines(theme);
    let rect = centered_rect(76, (all.len() as u16 + 4).min(area.height), area);
    f.render_widget(Clear, rect);

    let panel = Style::default().fg(theme.menu_fg).bg(theme.menu_bg);
    let block = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::symmetric(2, 0))
        .border_style(Style::default().fg(theme.accent).bg(theme.menu_bg))
        .title(Span::styled(
            " Keys & Shortcuts ",
            Style::default()
                .fg(theme.accent)
                .bg(theme.menu_bg)
                .add_modifier(Modifier::BOLD),
        ))
        .style(panel);
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let max_scroll = all.len().saturating_sub(inner.height as usize);
    let scroll = help.scroll.min(max_scroll);
    let shown: Vec<Line> = all
        .into_iter()
        .skip(scroll)
        .take(inner.height as usize)
        .collect();
    f.render_widget(Paragraph::new(shown).style(panel), inner);
    rect
}
