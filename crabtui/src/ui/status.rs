//! The one-row status bar: file, cursor position, transient messages.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let base = Style::default()
        .fg(theme.statusbar_fg)
        .bg(theme.statusbar_bg);
    let accent = Style::default().fg(theme.accent).bg(theme.statusbar_bg);

    // A clickable run / stop button occupies the far left (see `App.run_button`).
    let running = app
        .run
        .as_ref()
        .is_some_and(crate::run::RunConsole::is_running);
    let btn_label = app.run_button_label();
    let btn_style = Style::default()
        .fg(theme.statusbar_bg)
        .bg(if running {
            theme.output_err
        } else {
            theme.output_ok
        })
        .add_modifier(Modifier::BOLD);

    let cursor = app.buf().cursor();
    let tab_of = if app.buffers.len() > 1 {
        format!("[{}/{}] ", app.active + 1, app.buffers.len())
    } else {
        String::new()
    };
    let lang = format!(" {} ", app.buf().language().label());
    let pos = format!(" {tab_of}Ln {}, Col {} ", cursor.line + 1, cursor.col + 1);

    let left = if app.status.is_empty() {
        format!(" {} ", app.buf().title())
    } else {
        format!(" {} ", app.status)
    };

    let warn = match app.diagnostics.len() {
        0 => String::new(),
        1 => " ⚠ 1 undefined var ".to_string(),
        n => format!(" ⚠ {n} undefined vars "),
    };
    let warn_style = Style::default()
        .fg(theme.statusbar_bg)
        .bg(theme.output_err)
        .add_modifier(Modifier::BOLD);

    let used = btn_label.chars().count()
        + left.chars().count()
        + warn.chars().count()
        + lang.chars().count()
        + pos.chars().count();
    let gap = (area.width as usize).saturating_sub(used);
    let line = Line::from(vec![
        Span::styled(btn_label, btn_style),
        Span::styled(left, accent),
        Span::styled(" ".repeat(gap), base),
        Span::styled(warn, warn_style),
        Span::styled(lang, base),
        Span::styled(pos, base),
    ]);

    f.render_widget(Paragraph::new(line).style(base), area);
}
