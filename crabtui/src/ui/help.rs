//! The keys-and-shortcuts overlay (`F1`, or the palette's "Help" entry).
//!
//! A static reference card — grouped key/description pairs, scrollable if the
//! terminal is short. Any of Esc / Enter / F1 / `q` closes it.

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap};

use super::overlay::centered_rect;
use crate::theme::Theme;

/// `(section, [(keys, what)])` — the whole card.
pub const CARD: &[(&str, &[(&str, &str)])] = &[
    (
        "Files & tabs",
        &[
            ("Ctrl+S", "save (Save As if untitled)"),
            (
                "Ctrl+O",
                "open a file (a directory shows it in the file tree)",
            ),
            ("Ctrl+N", "new tab"),
            (
                "Ctrl+W",
                "close tab (refuses if unsaved) — closes the output panel instead, if that's focused",
            ),
            ("Ctrl+Tab / Ctrl+Shift+Tab", "next / previous tab"),
        ],
    ),
    (
        "Editing",
        &[
            ("Ctrl+Z / Ctrl+Y", "undo / redo"),
            ("Ctrl+A", "select all"),
            ("Ctrl+C / Ctrl+V", "copy / paste (system clipboard)"),
            ("Ctrl+D", "duplicate the current line"),
            ("Alt+↑ / Alt+↓", "move the current line up / down"),
            (
                "Ctrl+F",
                "find / replace bar (^R replace, Alt+A all, Esc close)",
            ),
            (
                "find bar › Alt+C / Alt+X",
                "toggle case-sensitive / regex matching — or click [Aa] / [.*]",
            ),
            ("Tab / Shift+Tab", "indent / dedent"),
            ("Ctrl+←/→", "word-wise motion"),
            ("Ctrl+Backspace / Delete", "delete the word behind / ahead"),
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
            (
                "F6",
                "run (if nothing's run yet) — else cycle focus: editor/files/outline/output",
            ),
            (
                "Tab / Shift+Tab (sidebars)",
                "also cycles focus, from any non-editor pane",
            ),
            ("Enter (in panel)", "send the line to the program's stdin"),
            ("Ctrl+D (in panel)", "close stdin (EOF)"),
            ("Ctrl+C (in panel)", "stop the running program"),
            ("↑/↓ PgUp/PgDn Home/End", "scroll the output"),
            ("F9", "close the output panel"),
            ("F11 / F12", "grow / shrink the output panel"),
        ],
    ),
    (
        "View & commands",
        &[
            ("Ctrl+P", "command palette"),
            (
                "palette › snippet: …",
                "insert an if/while/function/try/switch/… skeleton",
            ),
            ("Ctrl+T", "theme picker (live preview)"),
            (
                "palette › word wrap",
                "wrap long lines instead of scrolling",
            ),
            ("Ctrl+G", "go to line"),
            (
                "F2",
                "file tree — ↑↓ select, → / ← expand / collapse, Enter opens, r refresh, . hidden",
            ),
            ("F3", "snippet menu — ↑↓ select, Enter choose"),
            (
                "F4",
                "find in files — type to search, ↑↓ select, Enter/click opens",
            ),
            (
                "find in files › Alt+C / Alt+X",
                "toggle case-sensitive / regex matching — or click [Aa] / [.*]",
            ),
            (
                "F7",
                "structure outline — ↑↓ select, Enter jumps to the line",
            ),
            ("F8", "Projects — new / open / delete a project directory"),
            (
                "Projects › Delete",
                "type the folder's exact name to confirm — this deletes it from disk",
            ),
            ("F1  /  Ctrl+H", "this help"),
            ("Esc", "dismiss popup / overlay / leave the output panel"),
            ("Ctrl+Q", "quit (asks first if a buffer is unsaved)"),
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
            // Pad to a 24-char key column when it fits — but always leave at
            // least a 2-space gap, even for a key label longer than that,
            // so it never runs straight into the description.
            let key_col = format!("  {k}");
            let pad = 26usize.saturating_sub(key_col.chars().count()).max(2);
            out.push(Line::from(vec![
                Span::styled(format!("{key_col}{}", " ".repeat(pad)), key),
                Span::styled((*what).to_string(), desc),
            ]));
        }
    }
    out
}

/// How many screen rows one line takes once wrapped to `width` columns.
fn row_span(line: &Line, width: usize) -> usize {
    let lw = line.width();
    if lw == 0 { 1 } else { lw.div_ceil(width) }
}

/// How many screen rows `all` will occupy once wrapped to `width` columns —
/// used to size the box itself, since one logical line can span several rows.
fn visual_rows(all: &[Line], width: u16) -> usize {
    let w = width.max(1) as usize;
    all.iter().map(|l| row_span(l, w)).sum()
}

/// The furthest `scroll` (a logical-line index into `all`) can go while still
/// filling the last `visible_rows` screen rows with content, instead of
/// scrolling past the end and leaving the box mostly blank.
fn max_scroll(all: &[Line], width: u16, visible_rows: usize) -> usize {
    let w = width.max(1) as usize;
    if all.is_empty() || visible_rows == 0 {
        return 0;
    }
    let mut acc = 0;
    for (i, line) in all.iter().enumerate().rev() {
        let rows = row_span(line, w);
        if acc + rows > visible_rows {
            return i + 1;
        }
        acc += rows;
    }
    0
}

/// Returns the outer rect it drew into (for click-away hit-testing).
pub fn render(f: &mut Frame, help: &Help, theme: &Theme, area: Rect) -> Rect {
    let all = lines(theme);
    let total = all.len();
    // Use the terminal's width when there's room, so entries only wrap when
    // they genuinely don't fit — not just because the box is a fixed 76
    // columns on an 90+-column terminal.
    let box_w = area.width.saturating_sub(4).clamp(60, 100);
    let inner_w = box_w - 2 - 4; // border (2) + symmetric padding (2 + 2)
    let rows = visual_rows(&all, inner_w);
    let rect = centered_rect(box_w, (rows as u16 + 4).min(area.height), area);
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

    // With wrapping on, one logical line can take more than one screen row, so
    // the scroll bound isn't just `total - 1` — that would let scrolling run
    // past the end and leave most of the box blank. Stop once the last
    // `inner.height` screen rows are exactly filled with the tail content.
    let cap = max_scroll(&all, inner_w, inner.height as usize);
    let scroll = help.scroll.min(cap);
    let shown: Vec<Line> = all.into_iter().skip(scroll).collect();
    f.render_widget(
        Paragraph::new(shown)
            .style(panel)
            .wrap(Wrap { trim: false }),
        inner,
    );
    super::sidebar_scrollbar(f, theme, rect, total, scroll);
    rect
}
