//! The draggable divider between the editor and the output panel — one row,
//! a rule with a centred grab handle. Brightens while hovered or dragged.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::theme::Theme;

pub fn render(f: &mut Frame, theme: &Theme, active: bool, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let (fg, handle_fg) = if active {
        (theme.accent, theme.accent)
    } else {
        (theme.gutter_border, theme.line_fg)
    };
    let rule = Style::default().fg(fg).bg(theme.bg);
    let handle = Style::default()
        .fg(handle_fg)
        .bg(theme.bg)
        .add_modifier(Modifier::BOLD);

    let w = area.width as usize;
    let handle_str = " ╍╍╍╍ ";
    let side = w.saturating_sub(handle_str.chars().count()) / 2;
    let line = Line::from(vec![
        Span::styled("─".repeat(side), rule),
        Span::styled(handle_str, handle),
        Span::styled(
            "─".repeat(w.saturating_sub(side + handle_str.chars().count())),
            rule,
        ),
    ]);
    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(theme.bg)),
        area,
    );
}
