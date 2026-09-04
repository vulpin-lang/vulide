//! The structure-outline sidebar (`F7`). Renders `algo::outline` as an indented
//! tree; the app handles selection and jump-to-line.

use ratatui::Frame;
use ratatui::layout::{Position as TermPos, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph};

use crate::algo::{Item, Kind};
use crate::theme::Theme;

/// Colour for a kind's tag.
fn tag_colour(kind: Kind, theme: &Theme) -> ratatui::style::Color {
    match kind {
        Kind::Function => theme.function,
        Kind::Label => theme.command,
        Kind::Jump | Kind::Return => theme.keyword,
        _ => theme.control,
    }
}

/// Draw the sidebar into `area`. `scroll` is the index of the first visible row.
pub fn render(
    f: &mut Frame,
    items: &[Item],
    selected: usize,
    scroll: usize,
    theme: &Theme,
    focused: bool,
    area: Rect,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let panel = Style::default().fg(theme.dock_fg).bg(theme.dock_bg);
    let border = Style::default()
        .fg(if focused { theme.accent } else { theme.dock_fg })
        .bg(theme.dock_bg);
    let title = Style::default()
        .fg(theme.accent)
        .bg(theme.dock_bg)
        .add_modifier(if focused {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });

    let block = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::new(1, 1, 0, 0))
        .border_style(border)
        .title(Span::styled(" Outline ", title))
        .style(panel);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    if items.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                "no functions, labels,\nor blocks yet",
                Style::default().fg(theme.comment).bg(theme.dock_bg),
            ))
            .style(panel),
            inner,
        );
        return;
    }

    let rows: Vec<Line> = items
        .iter()
        .enumerate()
        .skip(scroll)
        .take(inner.height as usize)
        .map(|(i, it)| row(it, i == selected, inner.width as usize, theme))
        .collect();
    f.render_widget(Paragraph::new(rows).style(panel), inner);

    // Park the cursor on the selected row when focused.
    if focused {
        let vis = selected.saturating_sub(scroll);
        if (vis as u16) < inner.height {
            f.set_cursor_position(TermPos::new(inner.x, inner.y + vis as u16));
        }
    }
}

fn row(it: &Item, selected: bool, width: usize, theme: &Theme) -> Line<'static> {
    let bg = if selected {
        theme.current_line
    } else {
        theme.dock_bg
    };
    let indent = "  ".repeat(it.depth);
    let head = format!("{indent}{:>2} ", it.kind.tag());
    let tag_style = Style::default()
        .fg(tag_colour(it.kind, theme))
        .bg(bg)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default()
        .fg(theme.dock_fg)
        .bg(bg)
        .add_modifier(if selected {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });
    // Pad so the selected-row background fills the panel.
    let used = head.chars().count() + it.label.chars().count();
    let pad = " ".repeat(width.saturating_sub(used));
    Line::from(vec![
        Span::styled(head, tag_style),
        Span::styled(format!("{}{pad}", it.label), label_style),
    ])
}
