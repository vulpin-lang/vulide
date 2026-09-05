//! A `Buffer` is one open document: rope + cursor + selection + view state.
//!
//! Phase 1 keeps view state (`scroll_*`) on the buffer since there is exactly
//! one view per document. Phase 3 splits multiple buffers behind tabs.

pub mod history;
pub mod movement;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ropey::Rope;

use history::History;
use movement as mv;

use crate::syntax::Language;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub line: usize,
    /// Character index within the line (the trailing newline is not a column).
    pub col: usize,
}

/// Vulpin block openers — a newline after one of these adds an indent level.
/// Matches `BLOCK_OPENERS` in the Python IDE.
const BLOCK_OPENERS: &[char] = &['?', '@', 'O', 'F', 'T', 'W', 'V', 'N', ':'];

pub struct Buffer {
    rope: Rope,
    /// Content as of the last load/save. `is_dirty()` compares against it, so
    /// undoing back to the saved state clears the modified marker.
    saved: Rope,
    cursor: Position,
    anchor: Option<Position>,
    goal_col: Option<usize>,
    path: Option<PathBuf>,
    /// Grammar for highlighting — from the file extension, Vulpin for untitled.
    language: Language,
    history: History,
    pub tab_width: usize,
    /// Type a matching `)]}"` when an opener is inserted (config-driven).
    pub auto_close_brackets: bool,
    /// Copy the current line's indent (and add a level after a block opener)
    /// on newline (config-driven).
    pub auto_indent: bool,
    pub scroll_top: usize,
    pub scroll_left: usize,
    /// With word-wrap on: how many wrapped rows of the `scroll_top` line are
    /// scrolled off the top. Always 0 when wrap is off.
    pub scroll_subrow: usize,
}

/// Openers that get an auto-typed partner, with that partner.
const AUTO_PAIRS: &[(char, char)] = &[('(', ')'), ('[', ']'), ('{', '}'), ('"', '"')];

impl Buffer {
    pub fn new() -> Self {
        Self::from_str("")
    }

    pub fn from_str(text: &str) -> Self {
        let rope = Rope::from_str(text);
        Self {
            saved: rope.clone(),
            rope,
            cursor: Position::default(),
            anchor: None,
            goal_col: None,
            path: None,
            language: Language::Vulpin,
            history: History::new(),
            tab_width: 4,
            auto_close_brackets: false,
            auto_indent: true,
            scroll_top: 0,
            scroll_left: 0,
            scroll_subrow: 0,
        }
    }

    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let mut text = fs::read_to_string(path)?;
        // Drop one trailing newline so a file that ends in `\n` (almost all of
        // them) doesn't open with a blank last line. `save_as` puts it back.
        if text.ends_with('\n') {
            text.pop();
            if text.ends_with('\r') {
                text.pop();
            }
        }
        let mut buf = Self::from_str(&text);
        buf.path = Some(path.to_path_buf());
        buf.language = Language::from_path(path);
        Ok(buf)
    }

    pub fn save(&mut self) -> io::Result<()> {
        let path = self
            .path
            .clone()
            .ok_or_else(|| io::Error::other("buffer has no path"))?;
        self.save_as(path)
    }

    pub fn save_as(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        let mut text = self.rope.to_string();
        if !text.ends_with('\n') {
            text.push('\n');
        }
        fs::write(path, text)?;
        self.path = Some(path.to_path_buf());
        self.language = Language::from_path(path);
        self.saved = self.rope.clone();
        self.history.set_break();
        Ok(())
    }

    // ---- accessors ----

    pub fn rope(&self) -> &Rope {
        &self.rope
    }

    pub fn cursor(&self) -> Position {
        self.cursor
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn language(&self) -> Language {
        self.language
    }

    pub fn is_dirty(&self) -> bool {
        self.rope.len_bytes() != self.saved.len_bytes() || self.rope != self.saved
    }

    pub fn line_count(&self) -> usize {
        mv::line_count(&self.rope)
    }

    pub fn line_text(&self, line: usize) -> String {
        mv::line_text(&self.rope, line)
    }

    /// Selection as an ordered `(start, end)` pair, or `None`.
    pub fn selection(&self) -> Option<(Position, Position)> {
        let anchor = self.anchor?;
        if anchor == self.cursor {
            return None;
        }
        Some(if anchor < self.cursor {
            (anchor, self.cursor)
        } else {
            (self.cursor, anchor)
        })
    }

    /// The selected text, or `None` when there is no selection. Used to seed the
    /// find field from whatever is highlighted.
    pub fn selection_text(&self) -> Option<String> {
        let (a, b) = self.selection()?;
        let (i, j) = (self.char_index(a), self.char_index(b));
        Some(self.rope.slice(i..j).to_string())
    }

    pub fn title(&self) -> String {
        let name = self
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "untitled".to_string());
        if self.is_dirty() {
            format!("{name} *")
        } else {
            name
        }
    }

    fn char_index(&self, pos: Position) -> usize {
        let line = pos.line.min(mv::last_line(&self.rope));
        let base = self.rope.line_to_char(line);
        let max = mv::line_char_len(&self.rope, line);
        base + pos.col.min(max)
    }

    fn display_col(&self, pos: Position) -> usize {
        mv::display_col(&self.line_text(pos.line), pos.col)
    }

    // ---- cursor ----

    pub fn set_cursor(&mut self, pos: Position, extend: bool) {
        let pos = mv::clamp(&self.rope, pos);
        if extend {
            if self.anchor.is_none() {
                self.anchor = Some(self.cursor);
            }
        } else {
            self.anchor = None;
        }
        self.cursor = pos;
    }

    pub fn select_all(&mut self) {
        self.anchor = Some(Position::default());
        self.cursor = mv::doc_end(&self.rope);
    }

    fn horiz<F: FnOnce(&Rope, Position) -> Position>(&mut self, f: F, extend: bool) {
        self.goal_col = None;
        let next = f(&self.rope, self.cursor);
        self.set_cursor(next, extend);
        self.history.set_break();
    }

    fn vert<F: FnOnce(&Rope, Position, usize) -> Position>(&mut self, f: F, extend: bool) {
        let goal = match self.goal_col {
            Some(g) => g,
            None => {
                let g = self.display_col(self.cursor);
                self.goal_col = Some(g);
                g
            }
        };
        let next = f(&self.rope, self.cursor, goal);
        self.set_cursor(next, extend);
        self.history.set_break();
    }

    pub fn move_left(&mut self, extend: bool) {
        self.horiz(mv::left, extend);
    }
    pub fn move_right(&mut self, extend: bool) {
        self.horiz(mv::right, extend);
    }
    pub fn move_word_left(&mut self, extend: bool) {
        self.horiz(mv::word_left, extend);
    }
    pub fn move_word_right(&mut self, extend: bool) {
        self.horiz(mv::word_right, extend);
    }
    pub fn move_home(&mut self, extend: bool) {
        self.horiz(mv::line_home, extend);
    }
    pub fn move_end(&mut self, extend: bool) {
        self.horiz(mv::line_end, extend);
    }
    pub fn move_doc_start(&mut self, extend: bool) {
        self.horiz(|r, _| mv::doc_start(r), extend);
    }
    pub fn move_doc_end(&mut self, extend: bool) {
        self.horiz(|r, _| mv::doc_end(r), extend);
    }
    pub fn move_up(&mut self, extend: bool) {
        self.vert(mv::up, extend);
    }
    pub fn move_down(&mut self, extend: bool) {
        self.vert(mv::down, extend);
    }
    pub fn move_page_up(&mut self, rows: usize, extend: bool) {
        self.vert(|r, p, g| mv::page_up(r, p, rows, g), extend);
    }
    pub fn move_page_down(&mut self, rows: usize, extend: bool) {
        self.vert(|r, p, g| mv::page_down(r, p, rows, g), extend);
    }

    // ---- edits ----

    fn delete_selection(&mut self) -> bool {
        let Some((start, end)) = self.selection() else {
            return false;
        };
        let (a, b) = (self.char_index(start), self.char_index(end));
        self.history.record(&self.rope, self.cursor, false);
        self.rope.remove(a..b);
        self.anchor = None;
        self.cursor = start;
        self.history.set_break();
        true
    }

    pub fn insert_char(&mut self, ch: char) {
        self.goal_col = None;
        let had_selection = self.selection().is_some();
        self.delete_selection();
        if ch == '\n' {
            self.newline();
            return;
        }

        // Type straight through a matching closer/quote that auto-close put there.
        if self.auto_close_brackets
            && !had_selection
            && matches!(ch, ')' | ']' | '}' | '"')
            && self.char_after() == Some(ch)
        {
            self.cursor = mv::right(&self.rope, self.cursor);
            self.history.set_break();
            return;
        }

        self.history.record(&self.rope, self.cursor, true);
        let idx = self.char_index(self.cursor);
        self.rope.insert_char(idx, ch);
        self.cursor = self.pos_after(idx + 1);

        // Auto-type the partner, leaving the cursor between the pair.
        if self.auto_close_brackets
            && !had_selection
            && self.should_auto_close(ch)
            && let Some(&(_, close)) = AUTO_PAIRS.iter().find(|&&(open, _)| open == ch)
        {
            let cidx = self.char_index(self.cursor);
            self.rope.insert_char(cidx, close);
        }
    }

    /// Whether inserting `ch` should pull in its auto-pair partner right now.
    fn should_auto_close(&self, ch: char) -> bool {
        if !AUTO_PAIRS.iter().any(|&(open, _)| open == ch) {
            return false;
        }
        // Don't wrap into an adjacent word (`foo|` + `"` shouldn't become `foo"|"`).
        match self.char_after() {
            Some('"') if ch == '"' => false,
            Some(c) => !c.is_alphanumeric() && c != '_',
            None => true,
        }
    }

    fn char_after(&self) -> Option<char> {
        self.rope.get_char(self.char_index(self.cursor))
    }

    fn char_before(&self) -> Option<char> {
        let idx = self.char_index(self.cursor);
        (idx > 0).then(|| self.rope.char(idx - 1))
    }

    pub fn insert_str(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.goal_col = None;
        self.delete_selection();
        self.history.record(&self.rope, self.cursor, false);
        let idx = self.char_index(self.cursor);
        self.rope.insert(idx, text);
        self.cursor = self.pos_after(idx + text.chars().count());
        self.history.set_break();
    }

    pub fn newline(&mut self) {
        self.goal_col = None;
        self.delete_selection();
        self.history.record(&self.rope, self.cursor, false);

        let current = self.line_text(self.cursor.line);
        let (indent, extra): (String, usize) = if self.auto_indent {
            let indent = current.chars().take_while(|c| c.is_whitespace()).collect();
            let first_non_ws = current.trim_start().chars().next();
            let extra = if first_non_ws.is_some_and(|c| BLOCK_OPENERS.contains(&c)) {
                self.tab_width
            } else {
                0
            };
            (indent, extra)
        } else {
            (String::new(), 0)
        };
        let mut insert = String::with_capacity(1 + indent.len() + extra);
        insert.push('\n');
        insert.push_str(&indent);
        insert.extend(std::iter::repeat_n(' ', extra));

        let idx = self.char_index(self.cursor);
        self.rope.insert(idx, &insert);
        self.cursor = self.pos_after(idx + insert.chars().count());
        self.history.set_break();
    }

    pub fn delete_backward(&mut self) {
        self.goal_col = None;
        if self.delete_selection() {
            return;
        }
        // Backspace between an empty auto-pair (`(|)`) removes both sides.
        if self.auto_close_brackets
            && let (Some(b), Some(a)) = (self.char_before(), self.char_after())
            && AUTO_PAIRS
                .iter()
                .any(|&(open, close)| open == b && close == a)
        {
            let prev = mv::left(&self.rope, self.cursor);
            let next = mv::right(&self.rope, self.cursor);
            let (x, z) = (self.char_index(prev), self.char_index(next));
            self.history.record(&self.rope, self.cursor, false);
            self.rope.remove(x..z);
            self.cursor = prev;
            self.history.set_break();
            return;
        }
        let prev = mv::left(&self.rope, self.cursor);
        if prev == self.cursor {
            return;
        }
        let (a, b) = (self.char_index(prev), self.char_index(self.cursor));
        self.history.record(&self.rope, self.cursor, false);
        self.rope.remove(a..b);
        self.cursor = prev;
        self.history.set_break();
    }

    pub fn delete_forward(&mut self) {
        self.goal_col = None;
        if self.delete_selection() {
            return;
        }
        let next = mv::right(&self.rope, self.cursor);
        if next == self.cursor {
            return;
        }
        let (a, b) = (self.char_index(self.cursor), self.char_index(next));
        self.history.record(&self.rope, self.cursor, false);
        self.rope.remove(a..b);
        self.history.set_break();
    }

    /// `Ctrl+Backspace`: delete the word behind the cursor, same as selecting
    /// it with `Ctrl+Shift+Left` then pressing Backspace. An existing
    /// selection is just deleted as-is — the word motion never runs, so it
    /// can't silently grow a selection the user already made.
    pub fn delete_word_backward(&mut self) {
        if self.selection().is_none() {
            self.move_word_left(true);
        }
        self.delete_backward();
    }

    /// `Ctrl+Delete`: delete the word ahead of the cursor. Same
    /// existing-selection rule as `delete_word_backward`.
    pub fn delete_word_forward(&mut self) {
        if self.selection().is_none() {
            self.move_word_right(true);
        }
        self.delete_forward();
    }

    /// `Ctrl+D`: insert a copy of the cursor's line directly below it, cursor
    /// moving down onto the copy at the same column. A line operation, not a
    /// text operation — any active selection is cleared, not duplicated.
    pub fn duplicate_line(&mut self) {
        self.goal_col = None;
        let line = self.cursor.line;
        let start = self.rope.line_to_char(line);
        let end = if line + 1 < self.rope.len_lines() {
            self.rope.line_to_char(line + 1)
        } else {
            self.rope.len_chars()
        };
        let block = self.rope.slice(start..end).to_string();
        let insert = if block.ends_with('\n') {
            block
        } else {
            format!("\n{block}")
        };
        self.history.record(&self.rope, self.cursor, false);
        self.rope.insert(end, &insert);
        self.cursor = Position {
            line: line + 1,
            col: self.cursor.col,
        };
        self.anchor = None;
        self.history.set_break();
    }

    /// The `[start, end)` char range of a line's own content — the same as
    /// `line_char_len` (a newline / `\r\n` excluded), so swapping two lines'
    /// content never has to reason about which one owns the line break
    /// between (or after) them.
    fn line_content_range(&self, line: usize) -> (usize, usize) {
        let start = self.rope.line_to_char(line);
        (start, start + mv::line_char_len(&self.rope, line))
    }

    /// `Alt+Up` / `Alt+Down`: swap the cursor's line with its neighbor,
    /// cursor following its own (unchanged) text to the new line. A no-op at
    /// the top/bottom edge of the buffer.
    fn move_line(&mut self, dir: isize) {
        self.goal_col = None;
        let line = self.cursor.line;
        let last = mv::last_line(&self.rope);
        let target = if dir < 0 {
            if line == 0 {
                return;
            }
            line - 1
        } else {
            if line >= last {
                return;
            }
            line + 1
        };
        let (a, b) = (line.min(target), line.max(target));
        let (a_start, a_end) = self.line_content_range(a);
        let (b_start, b_end) = self.line_content_range(b);
        let text_a = self.rope.slice(a_start..a_end).to_string();
        let text_b = self.rope.slice(b_start..b_end).to_string();
        self.history.record(&self.rope, self.cursor, false);
        // The `b` range sits later in the rope — rewrite it first so `a`'s
        // indices (both before it) stay valid no matter how the lengths differ.
        self.rope.remove(b_start..b_end);
        self.rope.insert(b_start, &text_a);
        self.rope.remove(a_start..a_end);
        self.rope.insert(a_start, &text_b);
        self.cursor = Position {
            line: target,
            col: self.cursor.col,
        };
        self.anchor = None;
        self.history.set_break();
    }

    pub fn move_line_up(&mut self) {
        self.move_line(-1);
    }

    pub fn move_line_down(&mut self) {
        self.move_line(1);
    }

    /// Indent (or dedent) every line the cursor/selection touches.
    pub fn indent(&mut self, dedent: bool) {
        let (start_line, end_line) = match self.selection() {
            Some((a, b)) => (a.line, b.line),
            None => (self.cursor.line, self.cursor.line),
        };
        self.history.record(&self.rope, self.cursor, false);
        let pad: String = std::iter::repeat_n(' ', self.tab_width).collect();
        // Per-line signed column shift, so a partially-indented line that only
        // gave up two spaces moves the cursor by two, not by `tab_width`.
        let mut delta = vec![0isize; end_line - start_line + 1];
        for line in start_line..=end_line {
            let idx = self.rope.line_to_char(line);
            if dedent {
                let removed = self
                    .line_text(line)
                    .chars()
                    .take(self.tab_width)
                    .take_while(|c| *c == ' ')
                    .count();
                if removed > 0 {
                    self.rope.remove(idx..idx + removed);
                }
                delta[line - start_line] = -(removed as isize);
            } else {
                self.rope.insert(idx, &pad);
                delta[line - start_line] = self.tab_width as isize;
            }
        }
        let shift = |p: Position| -> Position {
            let d = p
                .line
                .checked_sub(start_line)
                .and_then(|i| delta.get(i))
                .copied()
                .unwrap_or(0);
            let col = (p.col as isize + d).max(0) as usize;
            Position { line: p.line, col }
        };
        self.cursor = mv::clamp(&self.rope, shift(self.cursor));
        if let Some(a) = self.anchor {
            self.anchor = Some(mv::clamp(&self.rope, shift(a)));
        }
        self.history.set_break();
    }

    /// Replace each `(start, end)` range with `with` as one undo step. Ranges
    /// must be non-overlapping; they are applied bottom-to-top so the char
    /// offsets of the not-yet-done ranges stay valid. The cursor lands after the
    /// topmost replacement (so a single-range call leaves it past the new text).
    /// Returns how many ranges were replaced.
    pub fn replace_ranges(&mut self, ranges: &[(Position, Position)], with: &str) -> usize {
        if ranges.is_empty() {
            return 0;
        }
        let mut sorted: Vec<(Position, Position)> = ranges.to_vec();
        sorted.sort_by_key(|r| r.0);
        self.history.record(&self.rope, self.cursor, false);
        let with_len = with.chars().count();
        let mut top_end = 0;
        for &(start, end) in sorted.iter().rev() {
            let (i, j) = (self.char_index(start), self.char_index(end));
            self.rope.remove(i..j);
            self.rope.insert(i, with);
            top_end = i + with_len; // last iteration = smallest offset
        }
        self.cursor = self.pos_after(top_end);
        self.anchor = None;
        self.goal_col = None;
        self.history.set_break();
        sorted.len()
    }

    pub fn undo(&mut self) -> bool {
        if let Some((rope, cursor)) = self.history.undo(&self.rope, self.cursor) {
            self.rope = rope;
            self.cursor = mv::clamp(&self.rope, cursor);
            self.anchor = None;
            self.goal_col = None;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some((rope, cursor)) = self.history.redo(&self.rope, self.cursor) {
            self.rope = rope;
            self.cursor = mv::clamp(&self.rope, cursor);
            self.anchor = None;
            self.goal_col = None;
            true
        } else {
            false
        }
    }

    fn pos_after(&self, char_idx: usize) -> Position {
        let line = self.rope.char_to_line(char_idx.min(self.rope.len_chars()));
        let col = char_idx - self.rope.line_to_char(line);
        Position { line, col }
    }

    /// Partner of the bracket at/just-before the cursor, if any.
    pub fn matching_bracket(&self) -> Option<Position> {
        let line = self.line_text(self.cursor.line);
        let chars: Vec<char> = line.chars().collect();
        let at = self.cursor.col;
        let candidates = [at, at.wrapping_sub(1)];
        for &c in candidates.iter() {
            if c < chars.len()
                && let Some(p) = self.scan_match(
                    Position {
                        line: self.cursor.line,
                        col: c,
                    },
                    chars[c],
                )
            {
                return Some(p);
            }
        }
        None
    }

    fn scan_match(&self, from: Position, bracket: char) -> Option<Position> {
        let (open, close, forward) = match bracket {
            '(' => ('(', ')', true),
            '[' => ('[', ']', true),
            '{' => ('{', '}', true),
            ')' => ('(', ')', false),
            ']' => ('[', ']', false),
            '}' => ('{', '}', false),
            _ => return None,
        };
        let mut depth = 0i32;
        let mut pos = from;
        loop {
            let ch = {
                let line = self.line_text(pos.line);
                line.chars().nth(pos.col)
            };
            if let Some(ch) = ch {
                if ch == open {
                    depth += if forward { 1 } else { -1 };
                } else if ch == close {
                    depth += if forward { -1 } else { 1 };
                }
                if depth == 0 && pos != from {
                    return Some(pos);
                }
            }
            let next = if forward {
                mv::right(&self.rope, pos)
            } else {
                mv::left(&self.rope, pos)
            };
            if next == pos {
                return None;
            }
            pos = next;
        }
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_move() {
        let mut b = Buffer::new();
        for ch in "G\"hi\"".chars() {
            b.insert_char(ch);
        }
        assert_eq!(b.rope().to_string(), "G\"hi\"");
        assert_eq!(b.cursor(), Position { line: 0, col: 5 });
        b.move_home(false);
        assert_eq!(b.cursor(), Position { line: 0, col: 0 });
    }

    #[test]
    fn trailing_newline_end_is_renderable() {
        // A rope that ends in '\n' has a phantom empty last line; the cursor may
        // sit on it and the editor must be able to draw it (line < line_count).
        let mut b = Buffer::from_str("G\"a\"\nG\"b\"\n");
        b.move_doc_end(false);
        assert!(b.cursor().line < b.line_count());
        assert_eq!(b.cursor(), Position { line: 2, col: 0 });
    }

    #[test]
    fn open_strips_one_trailing_newline() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("vulide_open_test_{}.vp", std::process::id()));
        std::fs::write(&path, "G\"a\"\nG\"b\"\n").unwrap();
        let b = Buffer::open(&path).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(b.line_count(), 2);
        assert_eq!(b.line_text(1), "G\"b\"");
    }

    #[test]
    fn language_is_picked_from_the_extension() {
        assert_eq!(Buffer::from_str("").language(), Language::Vulpin); // untitled
        let dir = std::env::temp_dir();
        for (ext, want) in [
            ("vul", Language::Vulpin),
            ("py", Language::Python),
            ("rs", Language::Rust),
            ("c", Language::C),
            ("sh", Language::Shell),
            ("toml", Language::Toml),
            ("json", Language::Json),
            ("md", Language::Markdown),
            ("ps1", Language::Plain),
        ] {
            let p = dir.join(format!("vulide_lang_{}_{}.{ext}", std::process::id(), ext));
            std::fs::write(&p, "x\n").unwrap();
            assert_eq!(Buffer::open(&p).unwrap().language(), want, "for .{ext}");
            std::fs::remove_file(&p).ok();
        }
    }

    #[test]
    fn word_left_across_line_start() {
        let mut b = Buffer::from_str("one\ntwo");
        b.set_cursor(Position { line: 1, col: 0 }, false);
        b.move_word_left(false); // must not panic; lands on the previous line
        assert_eq!(b.cursor().line, 0);
    }

    #[test]
    fn dedent_partial_indent_moves_cursor_by_actual() {
        let mut b = Buffer::from_str("  x"); // only 2 leading spaces
        b.set_cursor(Position { line: 0, col: 3 }, false);
        b.indent(true);
        assert_eq!(b.rope().to_string(), "x");
        assert_eq!(b.cursor(), Position { line: 0, col: 1 });
    }

    #[test]
    fn undo_to_saved_state_clears_dirty() {
        let mut b = Buffer::from_str("start");
        assert!(!b.is_dirty());
        b.insert_str(" more");
        assert!(b.is_dirty());
        b.undo();
        assert!(!b.is_dirty());
    }

    #[test]
    fn newline_autoindent_block_opener() {
        let mut b = Buffer::from_str("    ? $x > 1");
        b.move_end(false);
        b.newline();
        // 4 existing indent + 4 for the `?` opener
        assert_eq!(b.line_text(1), "        ");
        assert_eq!(b.cursor(), Position { line: 1, col: 8 });
    }

    #[test]
    fn selection_delete() {
        let mut b = Buffer::from_str("hello world");
        b.move_end(false);
        for _ in 0..6 {
            b.move_left(true);
        }
        assert_eq!(
            b.selection(),
            Some((Position { line: 0, col: 5 }, Position { line: 0, col: 11 }))
        );
        b.delete_backward();
        assert_eq!(b.rope().to_string(), "hello");
    }

    #[test]
    fn undo_coalesces_typing() {
        let mut b = Buffer::from_str("");
        for ch in "abc".chars() {
            b.insert_char(ch);
        }
        assert!(b.undo());
        assert_eq!(b.rope().to_string(), "");
        assert!(!b.undo());
    }

    #[test]
    fn undo_breaks_on_newline() {
        let mut b = Buffer::from_str("");
        for ch in "ab".chars() {
            b.insert_char(ch);
        }
        b.insert_char('\n');
        for ch in "cd".chars() {
            b.insert_char(ch);
        }
        assert!(b.undo()); // removes "cd"
        assert_eq!(b.rope().to_string(), "ab\n");
        assert!(b.undo()); // removes "\n"
        assert_eq!(b.rope().to_string(), "ab");
        assert!(b.undo()); // removes "ab"
        assert_eq!(b.rope().to_string(), "");
    }

    #[test]
    fn redo_after_undo() {
        let mut b = Buffer::from_str("");
        b.insert_str("hello");
        assert!(b.undo());
        assert_eq!(b.rope().to_string(), "");
        assert!(b.redo());
        assert_eq!(b.rope().to_string(), "hello");
    }

    #[test]
    fn vertical_keeps_goal_column() {
        let mut b = Buffer::from_str("longer line\nx\nanother line");
        b.set_cursor(Position { line: 0, col: 9 }, false);
        b.move_down(false); // line "x" — clamps to col 1
        assert_eq!(b.cursor(), Position { line: 1, col: 1 });
        b.move_down(false); // goal column 9 restored
        assert_eq!(b.cursor(), Position { line: 2, col: 9 });
    }

    #[test]
    fn matching_bracket_found() {
        let mut b = Buffer::from_str("F add(a, b)");
        b.set_cursor(Position { line: 0, col: 5 }, false); // on '('
        assert_eq!(b.matching_bracket(), Some(Position { line: 0, col: 10 }));
    }

    #[test]
    fn auto_close_inserts_and_overtypes_pair() {
        let mut b = Buffer::from_str("");
        b.auto_close_brackets = true;
        b.insert_char('F');
        b.insert_char('(');
        assert_eq!(b.rope().to_string(), "F()");
        assert_eq!(b.cursor(), Position { line: 0, col: 2 });
        b.insert_char(')'); // overtype, not a second ')'
        assert_eq!(b.rope().to_string(), "F()");
        assert_eq!(b.cursor(), Position { line: 0, col: 3 });
    }

    #[test]
    fn auto_close_backspace_removes_both() {
        let mut b = Buffer::from_str("");
        b.auto_close_brackets = true;
        b.insert_char('(');
        assert_eq!(b.rope().to_string(), "()");
        b.delete_backward();
        assert_eq!(b.rope().to_string(), "");
    }

    #[test]
    fn auto_indent_off_keeps_column_zero() {
        let mut b = Buffer::from_str("    ? $x > 1");
        b.auto_indent = false;
        b.move_end(false);
        b.newline();
        assert_eq!(b.line_text(1), "");
        assert_eq!(b.cursor(), Position { line: 1, col: 0 });
    }

    #[test]
    fn indent_and_dedent_selection() {
        let mut b = Buffer::from_str("a\nb\nc");
        b.set_cursor(Position { line: 0, col: 0 }, false);
        b.set_cursor(Position { line: 2, col: 1 }, true);
        b.indent(false);
        assert_eq!(b.rope().to_string(), "    a\n    b\n    c");
        b.indent(true);
        assert_eq!(b.rope().to_string(), "a\nb\nc");
    }

    #[test]
    fn delete_word_backward_eats_the_word_behind_the_cursor() {
        let mut b = Buffer::from_str("foo bar baz");
        b.set_cursor(Position { line: 0, col: 11 }, false); // end of "baz"
        b.delete_word_backward();
        assert_eq!(b.rope().to_string(), "foo bar ");
        b.delete_word_backward();
        assert_eq!(b.rope().to_string(), "foo ");
    }

    #[test]
    fn delete_word_forward_eats_the_word_ahead_of_the_cursor() {
        let mut b = Buffer::from_str("foo bar baz");
        b.set_cursor(Position { line: 0, col: 0 }, false);
        b.delete_word_forward();
        assert_eq!(b.rope().to_string(), "bar baz");
    }

    #[test]
    fn delete_word_backward_with_an_existing_selection_just_deletes_it() {
        let mut b = Buffer::from_str("foo bar baz");
        b.set_cursor(Position { line: 0, col: 4 }, false);
        b.set_cursor(Position { line: 0, col: 7 }, true); // selects "bar"
        b.delete_word_backward();
        assert_eq!(b.rope().to_string(), "foo  baz");
    }

    #[test]
    fn duplicate_line_copies_a_middle_line_and_moves_the_cursor_down() {
        let mut b = Buffer::from_str("one\ntwo\nthree");
        b.set_cursor(Position { line: 1, col: 2 }, false);
        b.duplicate_line();
        assert_eq!(b.rope().to_string(), "one\ntwo\ntwo\nthree");
        assert_eq!(b.cursor(), Position { line: 2, col: 2 });
    }

    #[test]
    fn duplicate_line_handles_the_last_line_with_no_trailing_newline() {
        let mut b = Buffer::from_str("only");
        b.set_cursor(Position { line: 0, col: 2 }, false);
        b.duplicate_line();
        assert_eq!(b.rope().to_string(), "only\nonly");
        assert_eq!(b.cursor(), Position { line: 1, col: 2 });
    }

    #[test]
    fn move_line_up_and_down_swap_adjacent_lines() {
        let mut b = Buffer::from_str("one\ntwo\nthree");
        b.set_cursor(Position { line: 1, col: 1 }, false); // "two"
        b.move_line_up();
        assert_eq!(b.rope().to_string(), "two\none\nthree");
        assert_eq!(
            b.cursor(),
            Position { line: 0, col: 1 },
            "follows its text up"
        );

        b.move_line_down();
        assert_eq!(b.rope().to_string(), "one\ntwo\nthree");
        assert_eq!(
            b.cursor(),
            Position { line: 1, col: 1 },
            "follows its text back down"
        );
    }

    #[test]
    fn move_line_is_a_no_op_at_the_buffer_edges() {
        let mut b = Buffer::from_str("one\ntwo");
        b.set_cursor(Position { line: 0, col: 0 }, false);
        b.move_line_up(); // already at the top
        assert_eq!(b.rope().to_string(), "one\ntwo");

        b.set_cursor(Position { line: 1, col: 0 }, false);
        b.move_line_down(); // already at the bottom
        assert_eq!(b.rope().to_string(), "one\ntwo");
    }

    #[test]
    fn move_line_swaps_with_the_last_line_lacking_a_trailing_newline() {
        let mut b = Buffer::from_str("one\ntwo\nthree"); // "three" has no \n
        b.set_cursor(Position { line: 2, col: 0 }, false);
        b.move_line_up();
        assert_eq!(b.rope().to_string(), "one\nthree\ntwo");
    }
}
