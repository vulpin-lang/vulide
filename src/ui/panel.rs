//! The run-output console panel: a title/status line, scrollable stdout+stderr
//! scrollback, and (while the child runs and the panel has focus) a stdin line.

use ratatui::Frame;
use ratatui::layout::{Position as TermPos, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::event::OutputStream;
use crate::run::RunConsole;
use crate::theme::Theme;

/// Renders the panel; returns the screen rect of the close-`✕` button.
pub fn render(
    f: &mut Frame,
    console: &RunConsole,
    theme: &Theme,
    focused: bool,
    hover_close: bool,
    area: Rect,
) -> Rect {
    if area.height == 0 || area.width == 0 {
        return Rect::default();
    }
    let bg = Style::default().bg(theme.output_bg).fg(theme.output_fg);
    f.render_widget(Paragraph::new("").style(bg), area);

    // ---- title / status row ----
    let (state, state_style) = if console.is_running() {
        (
            format!("running  {:.1}s", console.elapsed().as_secs_f32()),
            Style::default().fg(theme.accent).bg(theme.output_bg),
        )
    } else if console.stopped {
        (
            "stopped".to_string(),
            Style::default().fg(theme.output_err).bg(theme.output_bg),
        )
    } else {
        let code = console.exit_code.unwrap_or(-1);
        let style = if code == 0 {
            Style::default().fg(theme.output_ok).bg(theme.output_bg)
        } else {
            Style::default().fg(theme.output_err).bg(theme.output_bg)
        };
        (
            format!("exit {code}  ({:.1}s)", console.elapsed().as_secs_f32()),
            style,
        )
    };

    let title_style = Style::default()
        .fg(if focused {
            theme.accent
        } else {
            theme.output_fg
        })
        .bg(theme.output_bg)
        .add_modifier(if focused {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });
    let hint = if focused { "  Esc → editor" } else { "" };
    let close = " ✕ ";
    let right = format!("{state}{hint}{close}");
    let title = truncate(
        &format!("▶ {}", console.command),
        area.width.saturating_sub(right.chars().count() as u16 + 2) as usize,
    );
    let pad =
        (area.width as usize).saturating_sub(title.chars().count() + right.chars().count() + 1);
    let close_style = Style::default()
        .bg(theme.output_bg)
        .fg(if hover_close {
            theme.output_err
        } else {
            theme.output_fg
        })
        .add_modifier(if hover_close {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });
    let title_line = Line::from(vec![
        Span::styled(format!(" {title}"), title_style),
        Span::styled(" ".repeat(pad), bg),
        Span::styled(state.clone(), state_style),
        Span::styled(hint.to_string(), title_style),
        Span::styled(close, close_style),
    ]);
    f.render_widget(
        Paragraph::new(title_line).style(bg),
        Rect { height: 1, ..area },
    );
    let close_rect = Rect {
        x: area.x + area.width.saturating_sub(close.chars().count() as u16),
        y: area.y,
        width: close.chars().count() as u16,
        height: 1,
    };

    // ---- output body ----
    let running_input = console.is_running() && focused;
    let body_h = area.height.saturating_sub(1 + running_input as u16) as usize;
    if body_h == 0 {
        return close_rect;
    }
    let total = console.rows.len();
    // `scroll` counts lines up from the bottom; never scroll the last line above
    // the top of the viewport (so "scroll to top" shows the first page).
    let end = total.saturating_sub(console.scroll).max(body_h.min(total));
    let start = end.saturating_sub(body_h);
    let mut lines: Vec<Line> = Vec::with_capacity(body_h);
    for row in console.rows.iter().take(end).skip(start) {
        let style = match row.stream {
            OutputStream::Stdout if row.text.starts_with("< ") => Style::default()
                .fg(theme.statusbar_fg)
                .bg(theme.output_bg)
                .add_modifier(Modifier::DIM),
            OutputStream::Stdout => Style::default().fg(theme.output_fg).bg(theme.output_bg),
            OutputStream::Stderr => Style::default().fg(theme.output_err).bg(theme.output_bg),
        };
        lines.push(Line::from(Span::styled(format!(" {}", row.text), style)));
    }
    if console.scroll > 0 {
        // hint that there's more below
        while lines.len() < body_h {
            lines.push(Line::default());
        }
    }
    f.render_widget(
        Paragraph::new(lines).style(bg),
        Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: body_h as u16,
        },
    );

    // ---- stdin line ----
    if running_input {
        let y = area.y + area.height - 1;
        let prompt = "» ";
        let text = console.input.rope().to_string();
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    prompt,
                    Style::default().fg(theme.accent).bg(theme.output_bg),
                ),
                Span::styled(
                    text.clone(),
                    Style::default().fg(theme.output_fg).bg(theme.output_bg),
                ),
            ]))
            .style(bg),
            Rect {
                x: area.x,
                y,
                width: area.width,
                height: 1,
            },
        );
        let cx = area.x + prompt.chars().count() as u16 + console.input.cursor().col as u16;
        if cx < area.x + area.width {
            f.set_cursor_position(TermPos::new(cx, y));
        }
    }

    close_rect
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let keep = max.saturating_sub(1);
    format!("{}…", s.chars().take(keep).collect::<String>())
}
