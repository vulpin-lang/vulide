//! Cursor motions over a rope. All functions are pure: `(rope, pos) -> pos`.
//!
//! `col` is a **character** index within the line (newline excluded). Horizontal
//! motion steps whole grapheme clusters; vertical motion preserves a target
//! *display* column so moving through a line of tabs/wide glyphs lands sensibly.

use ropey::Rope;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::Position;

/// Line text with the trailing `\n` (if any) removed.
pub fn line_text(rope: &Rope, line: usize) -> String {
    if line >= rope.len_lines() {
        return String::new();
    }
    let s = rope.line(line).to_string();
    s.strip_suffix('\n')
        .map(|s| s.strip_suffix('\r').unwrap_or(s))
        .unwrap_or(&s)
        .to_string()
}

pub fn line_char_len(rope: &Rope, line: usize) -> usize {
    line_text(rope, line).chars().count()
}

/// Number of lines. `ropey` counts a final `\n` as a separator, so a rope
/// holding `"a\nb\n"` has three lines (the last empty). `Buffer::open` strips
/// one trailing newline on load so a freshly-opened file reads cleanly; a
/// trailing empty line only appears transiently, e.g. right after pressing
/// Enter at end of file — and the cursor is allowed to sit there.
pub fn line_count(rope: &Rope) -> usize {
    rope.len_lines()
}

pub fn last_line(rope: &Rope) -> usize {
    rope.len_lines().saturating_sub(1)
}

/// Clamp a position onto real text.
pub fn clamp(rope: &Rope, pos: Position) -> Position {
    let line = pos.line.min(last_line(rope));
    let col = pos.col.min(line_char_len(rope, line));
    Position { line, col }
}

/// Character index at the start of each grapheme in `line`, plus the end index.
fn grapheme_bounds(line: &str) -> Vec<usize> {
    let mut bounds = vec![0];
    let mut count = 0;
    for g in line.graphemes(true) {
        count += g.chars().count();
        bounds.push(count);
    }
    bounds
}

/// Display width of the first `char_col` characters of `line`.
pub fn display_col(line: &str, char_col: usize) -> usize {
    line.chars().take(char_col).collect::<String>().width()
}

/// The character column whose display position is closest to (but not past)
/// `target` display columns.
fn char_col_for_display(line: &str, target: usize) -> usize {
    let mut w = 0usize;
    for (i, ch) in line.chars().enumerate() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
        if w + cw > target {
            return i;
        }
        w += cw;
    }
    line.chars().count()
}

pub fn left(rope: &Rope, pos: Position) -> Position {
    if pos.col > 0 {
        let line = line_text(rope, pos.line);
        let bounds = grapheme_bounds(&line);
        let prev = bounds
            .iter()
            .rev()
            .find(|&&b| b < pos.col)
            .copied()
            .unwrap_or(0);
        Position {
            line: pos.line,
            col: prev,
        }
    } else if pos.line > 0 {
        let line = pos.line - 1;
        Position {
            line,
            col: line_char_len(rope, line),
        }
    } else {
        pos
    }
}

pub fn right(rope: &Rope, pos: Position) -> Position {
    let line = line_text(rope, pos.line);
    let len = line.chars().count();
    if pos.col < len {
        let bounds = grapheme_bounds(&line);
        let next = bounds
            .iter()
            .find(|&&b| b > pos.col)
            .copied()
            .unwrap_or(len);
        Position {
            line: pos.line,
            col: next,
        }
    } else if pos.line < last_line(rope) {
        Position {
            line: pos.line + 1,
            col: 0,
        }
    } else {
        pos
    }
}

pub fn up(rope: &Rope, pos: Position, target_display: usize) -> Position {
    if pos.line == 0 {
        return Position { line: 0, col: 0 };
    }
    let line = pos.line - 1;
    let col = char_col_for_display(&line_text(rope, line), target_display);
    Position { line, col }
}

pub fn down(rope: &Rope, pos: Position, target_display: usize) -> Position {
    if pos.line >= last_line(rope) {
        let line = last_line(rope);
        return Position {
            line,
            col: line_char_len(rope, line),
        };
    }
    let line = pos.line + 1;
    let col = char_col_for_display(&line_text(rope, line), target_display);
    Position { line, col }
}

pub fn line_start(_rope: &Rope, pos: Position) -> Position {
    Position {
        line: pos.line,
        col: 0,
    }
}

/// Smart home: first non-whitespace, then column 0.
pub fn line_home(rope: &Rope, pos: Position) -> Position {
    let line = line_text(rope, pos.line);
    let indent = line.chars().take_while(|c| c.is_whitespace()).count();
    let col = if pos.col == indent { 0 } else { indent };
    Position {
        line: pos.line,
        col,
    }
}

pub fn line_end(rope: &Rope, pos: Position) -> Position {
    Position {
        line: pos.line,
        col: line_char_len(rope, pos.line),
    }
}

pub fn doc_start(_rope: &Rope) -> Position {
    Position { line: 0, col: 0 }
}

pub fn doc_end(rope: &Rope) -> Position {
    let line = last_line(rope);
    Position {
        line,
        col: line_char_len(rope, line),
    }
}

pub fn page_up(rope: &Rope, pos: Position, rows: usize, target_display: usize) -> Position {
    let line = pos.line.saturating_sub(rows.max(1));
    Position {
        line,
        col: char_col_for_display(&line_text(rope, line), target_display),
    }
}

pub fn page_down(rope: &Rope, pos: Position, rows: usize, target_display: usize) -> Position {
    let line = (pos.line + rows.max(1)).min(last_line(rope));
    Position {
        line,
        col: char_col_for_display(&line_text(rope, line), target_display),
    }
}

fn class(c: char) -> u8 {
    if c.is_whitespace() {
        0
    } else if c.is_alphanumeric() || c == '_' {
        1
    } else {
        2
    }
}

pub fn word_left(rope: &Rope, pos: Position) -> Position {
    let mut p = pos;
    // step back over whitespace, then over one run of the same class
    let first = left(rope, p);
    if first == p {
        return p;
    }
    p = first;
    let line = line_text(rope, p.line);
    let ch: Vec<char> = line.chars().collect();
    while p.col > 0 && p.line == pos.line && class(ch[p.col - 1]) == 0 {
        p.col -= 1;
    }
    if p.col > 0 && p.line == pos.line {
        let target = class(ch[p.col - 1]);
        while p.col > 0 && class(ch[p.col - 1]) == target {
            p.col -= 1;
        }
    }
    p
}

pub fn word_right(rope: &Rope, pos: Position) -> Position {
    let line = line_text(rope, pos.line);
    let ch: Vec<char> = line.chars().collect();
    let len = ch.len();
    if pos.col >= len {
        return right(rope, pos);
    }
    let mut col = pos.col;
    let target = class(ch[col]);
    while col < len && class(ch[col]) == target {
        col += 1;
    }
    while col < len && class(ch[col]) == 0 {
        col += 1;
    }
    Position {
        line: pos.line,
        col,
    }
}
