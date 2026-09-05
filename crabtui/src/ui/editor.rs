//! The editor widget: renders a `&mut Buffer` (mut only to reconcile scroll)
//! with a line-number gutter, syntax highlighting, selection, and a
//! matched-bracket highlight. Places the real terminal cursor.
//!
//! With `wrap` on, a buffer line longer than the text area is shown on several
//! visual rows; `scroll_top` still counts buffer lines, `scroll_subrow` counts
//! wrapped rows of that top line. Up/Down still move by buffer line — moving by
//! visual row is a later change.

use ratatui::Frame;
use ratatui::layout::{Position as TermPos, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use unicode_width::UnicodeWidthChar;

use crate::buffer::movement;
use crate::buffer::{Buffer, Position};
use crate::syntax::Language;
use crate::theme::Theme;

const GUTTER_MIN: u16 = 4;

/// Char columns at which each visual row of `line` starts under wrap. Always
/// begins with `0`; the length is the visual-row count (>= 1).
fn wrap_starts(line: &str, text_w: usize) -> Vec<usize> {
    let mut starts = vec![0usize];
    if text_w == 0 {
        return starts;
    }
    let mut w = 0usize;
    for (i, ch) in line.chars().enumerate() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0).max(1);
        if w + cw > text_w && i > *starts.last().unwrap() {
            starts.push(i);
            w = cw;
        } else {
            w += cw;
        }
    }
    starts
}

/// Visual-row count for one buffer line.
fn wrap_rows(line: &str, text_w: usize) -> usize {
    wrap_starts(line, text_w).len()
}

/// Renders the editor and returns the cursor's screen cell, when it is visible
/// (the autocomplete popup anchors to it).
#[allow(clippy::too_many_arguments)]
pub fn render(
    f: &mut Frame,
    buf: &mut Buffer,
    theme: &Theme,
    show_numbers: bool,
    search: &[(Position, Position)],
    search_current: usize,
    diagnostics: &[(Position, Position)],
    wrap: bool,
    area: Rect,
) -> Option<(u16, u16)> {
    if area.width == 0 || area.height == 0 {
        return None;
    }
    f.render_widget(Block::default().style(Style::default().bg(theme.bg)), area);

    let total = buf.line_count().max(1);
    // +1 beyond the digits-and-a-space: a dedicated column for the
    // hex-color swatch (`colorswatch::line_color`), always reserved so the
    // swatch never depends on having spare line-number padding to borrow.
    let gutter_w = if show_numbers {
        ((total.to_string().len() as u16) + 1).max(GUTTER_MIN) + 1
    } else {
        0
    };
    let text_w = area.width.saturating_sub(gutter_w) as usize;
    let text_h = area.height as usize;
    if text_w == 0 {
        return None;
    }

    let cursor = buf.cursor();
    let cursor_text = buf.line_text(cursor.line);
    let cursor_disp = movement::display_col(&cursor_text, cursor.col);

    if wrap {
        buf.scroll_left = 0;
        reconcile_scroll_wrapped(buf, cursor, &cursor_text, text_w, text_h);
    } else {
        buf.scroll_subrow = 0;
        reconcile_scroll(buf, cursor, cursor_disp, text_w, text_h);
    }

    let selection = buf.selection();
    let bracket_match = buf.matching_bracket();
    let cursor_bracket = bracket_pair_at(buf, cursor);
    let language = buf.language();

    let mut lines: Vec<Line> = Vec::with_capacity(text_h);
    let mut cursor_screen = None;
    let mut screen_row = 0usize;
    let mut ln = buf.scroll_top;
    let mut skip = if wrap { buf.scroll_subrow } else { 0 };

    while screen_row < text_h {
        if ln >= total {
            lines.push(Line::default());
            screen_row += 1;
            continue;
        }
        let text = buf.line_text(ln);
        let nchars = text.chars().count();
        let starts = if wrap {
            wrap_starts(&text, text_w)
        } else {
            vec![0usize]
        };
        let is_current = ln == cursor.line;

        for vi in skip..starts.len() {
            if screen_row >= text_h {
                break;
            }
            let seg_start = starts[vi];
            let seg_end = starts.get(vi + 1).copied().unwrap_or(nchars);
            let last_seg = vi + 1 == starts.len();

            let mut spans: Vec<Span> = Vec::new();
            if gutter_w > 0 {
                let gutter_style = Style::default().bg(theme.bg).fg(if is_current {
                    theme.line_hl
                } else {
                    theme.line_fg
                });
                // Swatch column: only on a line's first visual row (matches
                // where the line number itself shows) — a wrapped
                // continuation row leaves it blank, same as the number.
                let swatch = if vi == 0 {
                    crate::colorswatch::line_color(&text)
                } else {
                    None
                };
                let (glyph, glyph_fg) = match swatch {
                    Some(c) => ("●", c),
                    None => (" ", gutter_style.fg.unwrap_or(theme.line_fg)),
                };
                spans.push(Span::styled(
                    glyph,
                    Style::default().bg(theme.bg).fg(glyph_fg),
                ));
                let number_w = (gutter_w - 2) as usize;
                let label = if vi == 0 {
                    format!("{:>number_w$} ", ln + 1)
                } else {
                    " ".repeat(number_w + 1)
                };
                spans.push(Span::styled(label, gutter_style));
            }

            let disp_start = if wrap { 0 } else { buf.scroll_left };
            spans.extend(styled_text(
                &text,
                ln,
                is_current,
                seg_start,
                seg_end,
                disp_start,
                text_w,
                selection,
                bracket_match,
                cursor_bracket,
                search,
                search_current,
                diagnostics,
                theme,
                language,
            ));
            lines.push(Line::from(spans));

            let cursor_here = is_current
                && cursor.col >= seg_start
                && (cursor.col < seg_end || (last_seg && cursor.col == nchars));
            if cursor_here {
                let y = area.y + screen_row as u16;
                let seg_disp = movement::display_col(&text, seg_start);
                if wrap {
                    let x = area.x + gutter_w + (cursor_disp - seg_disp) as u16;
                    if x < area.x + area.width {
                        f.set_cursor_position(TermPos::new(x, y));
                        cursor_screen = Some((x, y));
                    }
                } else if cursor_disp >= buf.scroll_left {
                    let x = area.x + gutter_w + (cursor_disp - buf.scroll_left) as u16;
                    if x < area.x + area.width {
                        f.set_cursor_position(TermPos::new(x, y));
                        cursor_screen = Some((x, y));
                    }
                }
            }
            screen_row += 1;
        }
        skip = 0;
        ln += 1;
    }

    f.render_widget(Paragraph::new(lines), area);
    cursor_screen
}

/// No-wrap: keep the cursor cell inside the text box, scrolling both axes.
fn reconcile_scroll(
    buf: &mut Buffer,
    cursor: Position,
    cursor_disp: usize,
    text_w: usize,
    text_h: usize,
) {
    if cursor.line < buf.scroll_top {
        buf.scroll_top = cursor.line;
    } else if text_h > 0 && cursor.line >= buf.scroll_top + text_h {
        buf.scroll_top = cursor.line + 1 - text_h;
    }
    if cursor_disp < buf.scroll_left {
        buf.scroll_left = cursor_disp;
    } else if cursor_disp >= buf.scroll_left + text_w {
        buf.scroll_left = cursor_disp + 1 - text_w;
    }
}

/// Wrap: `scroll_top`/`scroll_subrow` is the first visible visual row; keep the
/// cursor's visual row within `text_h` of it.
fn reconcile_scroll_wrapped(
    buf: &mut Buffer,
    cursor: Position,
    cursor_text: &str,
    text_w: usize,
    text_h: usize,
) {
    if text_h == 0 {
        return;
    }
    let cur_sub = wrap_starts(cursor_text, text_w)
        .iter()
        .rposition(|&s| s <= cursor.col)
        .unwrap_or(0);

    // Above the top → anchor there.
    if cursor.line < buf.scroll_top
        || (cursor.line == buf.scroll_top && cur_sub < buf.scroll_subrow)
    {
        buf.scroll_top = cursor.line;
        buf.scroll_subrow = cur_sub;
        return;
    }

    // Visual rows from the current top down to the cursor's row.
    let mut dist = cur_sub as isize - buf.scroll_subrow as isize;
    for l in buf.scroll_top..cursor.line {
        dist += wrap_rows(&buf.line_text(l), text_w) as isize;
    }
    if dist < text_h as isize {
        return;
    }

    // Push the top down by the overflow, in visual rows.
    let mut top = buf.scroll_top;
    let mut sub = buf.scroll_subrow + (dist as usize) + 1 - text_h;
    loop {
        let rows = wrap_rows(&buf.line_text(top), text_w);
        if sub < rows || top + 1 >= buf.line_count() {
            buf.scroll_subrow = sub.min(rows.saturating_sub(1));
            break;
        }
        sub -= rows;
        top += 1;
    }
    buf.scroll_top = top;
}

#[allow(clippy::too_many_arguments)]
fn styled_text(
    text: &str,
    line: usize,
    is_current: bool,
    char_start: usize,
    char_end: usize,
    disp_start: usize,
    disp_width: usize,
    selection: Option<(Position, Position)>,
    bracket_match: Option<Position>,
    cursor_bracket: Option<Position>,
    search: &[(Position, Position)],
    search_current: usize,
    diagnostics: &[(Position, Position)],
    theme: &Theme,
    language: Language,
) -> Vec<Span<'static>> {
    let base_bg = if is_current {
        theme.current_line
    } else {
        theme.bg
    };
    let tokens = language.tokenize_line(text);

    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut group = String::new();
    let mut group_style: Option<Style> = None;
    // Display column measured from `char_start`.
    let mut disp = 0usize;

    for (col, ch) in text.chars().enumerate() {
        if col < char_start {
            continue;
        }
        if col >= char_end {
            break;
        }
        let w = UnicodeWidthChar::width(ch).unwrap_or(0).max(1);
        if disp + w <= disp_start {
            disp += w;
            continue;
        }
        if disp >= disp_start + disp_width {
            break;
        }

        let mut style = Style::default().bg(base_bg).fg(theme.fg);
        if let Some(kind) = tokens
            .iter()
            .find(|t| col >= t.start && col < t.end)
            .map(|t| t.kind)
            && let Some(fg) = theme.token_color(kind)
        {
            style = style.fg(fg);
        }
        let here = Position { line, col };
        if selection.is_some_and(|(a, b)| a <= here && here < b) {
            style = style.bg(theme.sel);
        }
        if Some(here) == bracket_match || Some(here) == cursor_bracket {
            style = style.fg(theme.match_bracket).add_modifier(Modifier::BOLD);
        }
        if diagnostics.iter().any(|(a, b)| *a <= here && here < *b) {
            style = style
                .fg(theme.output_err)
                .add_modifier(Modifier::UNDERLINED);
        }
        if let Some((mi, _)) = search
            .iter()
            .enumerate()
            .find(|(_, (a, b))| *a <= here && here < *b)
        {
            if mi == search_current {
                style = style
                    .bg(theme.accent)
                    .fg(theme.bg)
                    .add_modifier(Modifier::BOLD);
            } else {
                style = style.bg(theme.match_bracket).fg(theme.bg);
            }
        }

        if group_style == Some(style) {
            group.push(ch);
        } else {
            flush(&mut spans, &mut group, &mut group_style);
            group.push(ch);
            group_style = Some(style);
        }
        disp += w;
    }
    flush(&mut spans, &mut group, &mut group_style);
    spans
}

fn flush(spans: &mut Vec<Span<'static>>, group: &mut String, style: &mut Option<Style>) {
    if !group.is_empty() {
        spans.push(Span::styled(
            std::mem::take(group),
            style.unwrap_or_default(),
        ));
    }
    *style = None;
}

/// If the cursor sits on or just after a bracket, the position of that bracket.
fn bracket_pair_at(buf: &Buffer, cursor: Position) -> Option<Position> {
    let line = buf.line_text(cursor.line);
    let chars: Vec<char> = line.chars().collect();
    for c in [cursor.col, cursor.col.wrapping_sub(1)] {
        if c < chars.len() && "()[]{}".contains(chars[c]) {
            return Some(Position {
                line: cursor.line,
                col: c,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{wrap_rows, wrap_starts};

    #[test]
    fn empty_and_short_lines_are_one_row() {
        assert_eq!(wrap_rows("", 10), 1);
        assert_eq!(wrap_rows("abc", 10), 1);
        assert_eq!(wrap_rows("abcdefghij", 10), 1); // exactly the width
    }

    #[test]
    fn overflow_wraps() {
        assert_eq!(wrap_rows("abcdefghijk", 10), 2); // width + 1
        assert_eq!(wrap_starts("abcdefghijk", 10), vec![0, 10]);
        assert_eq!(wrap_rows(&"x".repeat(25), 10), 3);
        assert_eq!(wrap_starts(&"x".repeat(25), 10), vec![0, 10, 20]);
    }

    #[test]
    fn zero_width_is_one_row() {
        assert_eq!(wrap_rows("anything", 0), 1);
    }
}
