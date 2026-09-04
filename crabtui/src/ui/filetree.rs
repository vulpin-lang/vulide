//! The file-tree sidebar (`F2`). Renders `FileTree::rows` as an indented list;
//! the app handles selection, expand/collapse, and open-file.

use ratatui::Frame;
use ratatui::layout::{Position as TermPos, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph};

use crate::filetree::{FileTree, Row};
use crate::theme::Theme;

/// Draw the sidebar into `area`. `scroll` is the index of the first visible row.
pub fn render(
    f: &mut Frame,
    tree: Option<&FileTree>,
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

    let heading = match tree.map(|t| t.root.as_path()) {
        Some(root) => {
            let name = root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| root.display().to_string());
            // Keep the tail of a long folder name — the border eats the rest.
            let budget = area.width.saturating_sub(11) as usize; // "┌ Files — " + "┐"
            let name = if name.chars().count() > budget && budget > 1 {
                let tail: String = name
                    .chars()
                    .skip(name.chars().count() - (budget - 1))
                    .collect();
                format!("…{tail}")
            } else {
                name
            };
            format!(" Files — {name} ")
        }
        None => " Files ".to_string(),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::new(1, 1, 0, 0))
        .border_style(border)
        .title(Span::styled(heading, title))
        .style(panel);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let rows = tree.map(FileTree::rows).unwrap_or(&[]);
    if rows.is_empty() {
        // No tree yet vs. a real directory that is empty / unreadable.
        let msg = if tree.is_none() {
            "…"
        } else {
            "empty or unreadable"
        };
        f.render_widget(
            Paragraph::new(Span::styled(
                msg,
                Style::default().fg(theme.comment).bg(theme.dock_bg),
            ))
            .style(panel),
            inner,
        );
        return;
    }

    let lines: Vec<Line> = rows
        .iter()
        .enumerate()
        .skip(scroll)
        .take(inner.height as usize)
        .map(|(i, r)| row_line(r, i == selected, inner.width as usize, theme))
        .collect();
    f.render_widget(Paragraph::new(lines).style(panel), inner);

    if focused {
        let vis = selected.saturating_sub(scroll);
        if (vis as u16) < inner.height {
            f.set_cursor_position(TermPos::new(inner.x, inner.y + vis as u16));
        }
    }
}

fn row_line(r: &Row, selected: bool, width: usize, theme: &Theme) -> Line<'static> {
    let bg = if selected {
        theme.current_line
    } else {
        theme.dock_bg
    };
    let indent = "  ".repeat(r.depth);
    let marker = if r.is_dir {
        if r.expanded { "▾ " } else { "▸ " }
    } else {
        "  "
    };
    let head = format!("{indent}{marker}");

    let name_fg = if r.is_dir {
        theme.function
    } else if r.is_vul {
        theme.string
    } else {
        theme.dock_fg
    };
    let head_style = Style::default().fg(theme.comment).bg(bg);
    let name_style = Style::default()
        .fg(name_fg)
        .bg(bg)
        .add_modifier(if selected || r.is_dir {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });

    let used = head.chars().count() + r.name.chars().count();
    let pad = " ".repeat(width.saturating_sub(used));
    Line::from(vec![
        Span::styled(head, head_style),
        Span::styled(format!("{}{pad}", r.name), name_style),
    ])
}
