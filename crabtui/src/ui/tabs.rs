//! The tab strip — one row per open buffer. Mouse-friendly: each tab reports
//! its hit rect and a close-`✕` rect, and the hovered tab lights up.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;

/// Where a tab (and its `✕`) landed on screen, for click hit-testing.
#[derive(Clone, Copy)]
pub struct TabHit {
    pub index: usize,
    pub rect: Rect,
    pub close: Rect,
}

pub fn render(f: &mut Frame, app: &App, area: Rect) -> Vec<TabHit> {
    let theme = &app.theme;
    let strip_bg = Style::default().bg(theme.tab_bg);
    let sep = Style::default().fg(theme.gutter_border).bg(theme.tab_bg);

    f.render_widget(Paragraph::new("").style(strip_bg), area);

    let mut spans: Vec<Span> = Vec::new();
    let mut hits = Vec::with_capacity(app.buffers.len());
    let mut x = area.x;

    for (i, buf) in app.buffers.iter().enumerate() {
        let is_active = i == app.active;
        let is_hover = app.hovered_tab == Some(i);

        // `tab_active` is a background tint, not a text color — using it as
        // the active tab's foreground (on `theme.bg`, itself a near-identical
        // dark shade) made the active tab unreadable. The active tab gets
        // its own bright, bold background instead; the inactive tabs are
        // unchanged.
        let (fg, bg) = match (is_active, is_hover) {
            (true, _) => (theme.fg, theme.tab_active),
            (false, true) => (theme.toolbar_fg, theme.current_line),
            (false, false) => (theme.toolbar_fg, theme.tab_bg),
        };
        let mut style = Style::default().fg(fg).bg(bg);
        if is_active {
            style = style.add_modifier(Modifier::BOLD);
        }
        if is_hover && !is_active {
            style = style.add_modifier(Modifier::UNDERLINED);
        }

        // A thin accent-colored edge outlines the active tab; on every other
        // tab it's just a blank space, so the width math stays uniform.
        let edge = if is_active { "▎" } else { " " };
        let edge_style = Style::default()
            .fg(theme.accent)
            .bg(bg)
            .add_modifier(Modifier::BOLD);

        let title = format!(" {} ", buf.title());
        // The ✕ shows on the active tab and whichever tab is hovered.
        let close_glyph = if is_active || is_hover { "✕ " } else { "  " };

        let edge_w = 1u16;
        let title_w = title.chars().count() as u16;
        let close_w = close_glyph.chars().count() as u16;
        let tab_w = edge_w + title_w + close_w + edge_w;

        spans.push(Span::styled(edge, edge_style));
        spans.push(Span::styled(title, style));
        let close_style = if is_hover {
            style.fg(theme.output_err).add_modifier(Modifier::BOLD)
        } else {
            style
        };
        spans.push(Span::styled(close_glyph, close_style));
        spans.push(Span::styled(edge, edge_style));
        spans.push(Span::styled("│", sep));

        let rect = Rect {
            x,
            y: area.y,
            width: tab_w,
            height: 1,
        };
        let close = Rect {
            x: x + edge_w + title_w,
            y: area.y,
            width: close_w,
            height: 1,
        };
        hits.push(TabHit {
            index: i,
            rect,
            close,
        });
        x += tab_w + 1; // +1 for the separator
    }

    f.render_widget(Paragraph::new(Line::from(spans)).style(strip_bg), area);
    hits
}
