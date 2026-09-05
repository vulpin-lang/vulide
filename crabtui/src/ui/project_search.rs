//! `F4`: find in files — a substring or regex search across every text file
//! under the project root, opened as a jump-to-result list. `Alt+C` / `Alt+X`
//! (or clicking `[Aa]` / `[.*]`) toggle case sensitivity and regex mode,
//! matching the in-file find bar (`search.rs`) — both share the actual
//! matching via `crate::matcher`.

use std::path::{Path, PathBuf};

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Position as TermPos, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph};

use super::overlay::centered_rect;
use crate::buffer::Buffer;
use crate::matcher::Matcher;
use crate::theme::Theme;

const MAX_ROWS: usize = 14;
/// Stop scanning once this many matches are found — keeps a huge project
/// (or an accidental one-char query) from hanging the UI.
const MAX_MATCHES: usize = 300;
/// Files above this size are skipped rather than read in full.
const MAX_FILE_BYTES: u64 = 2_000_000;
const IGNORED_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "dist",
    "build",
    ".venv",
    "venv",
    "__pycache__",
    ".idea",
    ".vscode",
];

pub struct Match {
    pub path: PathBuf,
    /// 0-based line / char-column, matching `buffer::Position`.
    pub line: usize,
    pub col: usize,
    pub preview: String,
}

pub enum ProjectSearchAction {
    Stay,
    Requery,
    Cancel,
    /// Jump to this match: path, 0-based line, 0-based char column.
    Open(PathBuf, usize, usize),
}

pub struct ProjectSearch {
    input: Buffer,
    pub matches: Vec<Match>,
    pub selected: usize,
    pub truncated: bool,
    pub case_sensitive: bool,
    pub regex: bool,
    /// `Some` when `regex` is on and the query doesn't compile — the result
    /// list shows this instead of a match count.
    pub error: Option<String>,
    /// Hit rects for the `[Aa]` / `[.*]` toggle buttons — set by `render`,
    /// read by `click`.
    case_rect: Option<Rect>,
    regex_rect: Option<Rect>,
}

impl Default for ProjectSearch {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectSearch {
    pub fn new() -> Self {
        Self {
            input: Buffer::new(),
            matches: Vec::new(),
            selected: 0,
            truncated: false,
            case_sensitive: false,
            regex: false,
            error: None,
            case_rect: None,
            regex_rect: None,
        }
    }

    pub fn query(&self) -> String {
        self.input.rope().to_string()
    }

    fn open_selected(&self) -> ProjectSearchAction {
        match self.matches.get(self.selected) {
            Some(m) => ProjectSearchAction::Open(m.path.clone(), m.line, m.col),
            None => ProjectSearchAction::Stay,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> ProjectSearchAction {
        if key.modifiers.contains(KeyModifiers::ALT) {
            return match key.code {
                KeyCode::Char('c' | 'C') => {
                    self.case_sensitive = !self.case_sensitive;
                    ProjectSearchAction::Requery
                }
                KeyCode::Char('x' | 'X') => {
                    self.regex = !self.regex;
                    ProjectSearchAction::Requery
                }
                _ => ProjectSearchAction::Stay,
            };
        }
        let plain = !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        match key.code {
            KeyCode::Esc => ProjectSearchAction::Cancel,
            KeyCode::Enter => self.open_selected(),
            KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                ProjectSearchAction::Stay
            }
            KeyCode::Down => {
                if self.selected + 1 < self.matches.len() {
                    self.selected += 1;
                }
                ProjectSearchAction::Stay
            }
            KeyCode::Backspace => {
                self.input.delete_backward();
                ProjectSearchAction::Requery
            }
            KeyCode::Delete => {
                self.input.delete_forward();
                ProjectSearchAction::Requery
            }
            KeyCode::Char(c) if plain => {
                self.input.insert_char(c);
                ProjectSearchAction::Requery
            }
            _ => ProjectSearchAction::Stay,
        }
    }

    /// Row-hit test against the list. One border row, the `> query` line, and
    /// the case/regex indicator line sit above it.
    fn row_at(&self, outer: Rect, row: u16) -> Option<usize> {
        let body_top = outer.y + 3;
        if row < body_top {
            return None;
        }
        let top = self.selected.saturating_sub(MAX_ROWS - 1);
        let idx = top + (row - body_top) as usize;
        (idx < self.matches.len()).then_some(idx)
    }

    fn button_hit(r: Option<Rect>, col: u16, row: u16) -> bool {
        r.is_some_and(|r| r.x <= col && col < r.x + r.width && r.y == row)
    }

    /// A left click at `(col, row)`: toggle a button if it landed on one,
    /// else select-and-jump on a result row, same as Enter.
    pub fn click(&mut self, outer: Rect, col: u16, row: u16) -> ProjectSearchAction {
        if Self::button_hit(self.case_rect, col, row) {
            self.case_sensitive = !self.case_sensitive;
            return ProjectSearchAction::Requery;
        }
        if Self::button_hit(self.regex_rect, col, row) {
            self.regex = !self.regex;
            return ProjectSearchAction::Requery;
        }
        match self.row_at(outer, row) {
            Some(idx) => {
                self.selected = idx;
                self.open_selected()
            }
            None => ProjectSearchAction::Stay,
        }
    }

    /// Mouse wheel: move the selection by one row.
    pub fn scroll(&mut self, delta: isize) {
        if self.matches.is_empty() {
            return;
        }
        let n = self.matches.len() as isize;
        self.selected = (self.selected as isize + delta).clamp(0, n - 1) as usize;
    }
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for e in entries {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || IGNORED_DIRS.contains(&name.as_str()) {
            continue;
        }
        let path = e.path();
        if e.file_type().is_ok_and(|t| t.is_dir()) {
            collect_files(&path, out);
        } else {
            out.push(path);
        }
    }
}

pub struct RunResult {
    pub matches: Vec<Match>,
    pub truncated: bool,
    /// `Some` when `regex` was on and `query` didn't compile.
    pub error: Option<String>,
}

/// Scan every text file under `root` for `query` (a literal substring, or —
/// if `regex` — a pattern), capped at `MAX_MATCHES`.
pub fn run(root: &Path, query: &str, case_sensitive: bool, regex: bool) -> RunResult {
    if query.is_empty() {
        return RunResult {
            matches: Vec::new(),
            truncated: false,
            error: None,
        };
    }
    let matcher = match Matcher::compile(query, case_sensitive, regex) {
        Ok(m) => m,
        Err(error) => {
            return RunResult {
                matches: Vec::new(),
                truncated: false,
                error: Some(error),
            };
        }
    };

    let mut files = Vec::new();
    collect_files(root, &mut files);

    let mut out = Vec::new();
    for path in files {
        if out.len() >= MAX_MATCHES {
            return RunResult {
                matches: out,
                truncated: true,
                error: None,
            };
        }
        if std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_FILE_BYTES) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue; // binary / not UTF-8 — skip rather than error
        };
        for (line_no, line) in text.lines().enumerate() {
            for (col, _) in matcher.find_in_line(line) {
                out.push(Match {
                    path: path.clone(),
                    line: line_no,
                    col,
                    preview: line.trim().to_string(),
                });
                if out.len() >= MAX_MATCHES {
                    return RunResult {
                        matches: out,
                        truncated: true,
                        error: None,
                    };
                }
            }
        }
    }
    RunResult {
        matches: out,
        truncated: false,
        error: None,
    }
}

/// Returns the outer rect it drew into (for click-away hit-testing). Records
/// the `[Aa]` / `[.*]` button rects onto `ps` for `click`.
pub fn render(
    f: &mut Frame,
    ps: &mut ProjectSearch,
    theme: &Theme,
    root: &Path,
    area: Rect,
) -> Rect {
    let rows = ps.matches.len().clamp(1, MAX_ROWS) as u16;
    let rect = centered_rect(96, rows + 5, area);
    f.render_widget(Clear, rect);

    let panel = Style::default().fg(theme.menu_fg).bg(theme.menu_bg);
    let accent = Style::default()
        .fg(theme.accent)
        .bg(theme.menu_bg)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(theme.dock_fg).bg(theme.menu_bg);
    let label = |on: bool| if on { accent } else { dim };
    let title = format!(
        " Find in Files — {} ",
        root.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.display().to_string())
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::symmetric(1, 0))
        .border_style(Style::default().fg(theme.accent).bg(theme.menu_bg))
        .title(Span::styled(title, accent))
        .style(panel);
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let query = ps.query();

    // Buttons: same toggle as Alt+C / Alt+X, clickable for mouse users.
    // "[Aa]" (case) and "[.*]" (regex) — ASCII, not the ⌥ symbol glyph a lot
    // of terminal fonts render as a blank box.
    let case_text = "[Aa]";
    let regex_text = "[.*]";
    let mut x = inner.x + 2;
    let case_rect = Rect {
        x,
        y: inner.y + 1,
        width: case_text.chars().count() as u16,
        height: 1,
    };
    x += case_rect.width + 1;
    let regex_rect = Rect {
        x,
        y: inner.y + 1,
        width: regex_text.chars().count() as u16,
        height: 1,
    };
    ps.case_rect = Some(case_rect);
    ps.regex_rect = Some(regex_rect);

    let mut lines = vec![
        Line::from(vec![
            Span::styled("> ", accent),
            Span::styled(query.clone(), panel),
        ]),
        Line::from(vec![
            Span::styled("  ", panel),
            Span::styled(case_text, label(ps.case_sensitive)),
            Span::styled(" ", panel),
            Span::styled(regex_text, label(ps.regex)),
            Span::styled("  Alt+C case · Alt+X regex", dim),
        ]),
    ];

    let top = ps.selected.saturating_sub(MAX_ROWS - 1);
    for (row, m) in ps.matches.iter().enumerate().skip(top).take(MAX_ROWS) {
        let selected = row == ps.selected;
        let style = if selected {
            Style::default()
                .fg(theme.autocomplete_fg)
                .bg(theme.autocomplete_sel)
                .add_modifier(Modifier::BOLD)
        } else {
            panel
        };
        let marker = if selected { "▸ " } else { "  " };
        let rel = m.path.strip_prefix(root).unwrap_or(&m.path);
        let loc = format!("{}:{}", rel.display(), m.line + 1);
        let text = format!("{marker}{loc}  {}", m.preview);
        let budget = inner.width as usize;
        let text = if text.chars().count() > budget && budget > 1 {
            let head: String = text.chars().take(budget - 1).collect();
            format!("{head}…")
        } else {
            text
        };
        lines.push(Line::from(Span::styled(text, style)));
    }
    if let Some(err) = &ps.error {
        lines.push(Line::from(Span::styled(
            format!("  regex error: {}", err.lines().next().unwrap_or(err)),
            Style::default()
                .fg(theme.output_err)
                .bg(theme.menu_bg)
                .add_modifier(Modifier::BOLD),
        )));
    } else if query.is_empty() {
        lines.push(Line::from(Span::styled(
            "  type to search every file under the project root",
            dim,
        )));
    } else if ps.matches.is_empty() {
        lines.push(Line::from(Span::styled("  (no matches)", dim)));
    } else if ps.truncated {
        lines.push(Line::from(Span::styled(
            format!("  showing the first {MAX_MATCHES} matches…"),
            dim,
        )));
    }

    f.render_widget(Paragraph::new(lines).style(panel), inner);
    super::sidebar_scrollbar(f, theme, rect, ps.matches.len(), top);
    f.set_cursor_position(TermPos::new(
        inner.x + 2 + query.chars().count() as u16,
        inner.y,
    ));
    rect
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vulide_psearch_{}_{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::create_dir_all(dir.join(".git")).unwrap();
        fs::write(dir.join("main.vul"), "G\"needle here\"\nK x\n").unwrap();
        fs::write(dir.join("sub").join("inner.vul"), "P\"Needle again\"\n").unwrap();
        fs::write(dir.join(".git").join("config"), "needle\n").unwrap();
        dir
    }

    #[test]
    fn finds_matches_case_insensitively_by_default_across_files() {
        let dir = fixture();
        let r = run(&dir, "needle", false, false);
        assert!(!r.truncated);
        assert!(r.error.is_none());
        assert_eq!(r.matches.len(), 2, "the .git file must be skipped");
        assert!(r.matches.iter().any(|m| m.path.ends_with("main.vul")));
        assert!(r.matches.iter().any(|m| m.path.ends_with("inner.vul")));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn case_sensitive_excludes_differently_cased_matches() {
        let dir = fixture();
        // main.vul has "needle", sub/inner.vul has "Needle" — only the exact
        // case should match.
        let r = run(&dir, "needle", true, false);
        assert_eq!(r.matches.len(), 1);
        assert!(r.matches[0].path.ends_with("main.vul"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn regex_mode_matches_a_pattern_across_files() {
        let dir = fixture();
        let r = run(&dir, "[Nn]eedle", false, true);
        assert_eq!(r.matches.len(), 2);
        assert!(r.error.is_none());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn invalid_regex_reports_an_error_and_no_matches() {
        let dir = fixture();
        let r = run(&dir, "(unclosed", false, true);
        assert!(r.matches.is_empty());
        assert!(r.error.is_some());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn reports_the_exact_char_column() {
        let dir = fixture();
        let r = run(&dir, "needle", false, false);
        let m = r
            .matches
            .iter()
            .find(|m| m.path.ends_with("main.vul"))
            .unwrap();
        assert_eq!(m.line, 0);
        assert_eq!(m.col, 2); // G"needle here" — 'n' of needle is char index 2
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn empty_query_yields_no_matches() {
        let dir = fixture();
        let r = run(&dir, "", false, false);
        assert!(r.matches.is_empty());
        assert!(!r.truncated);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn alt_c_and_alt_x_toggle_the_flags() {
        let mut ps = ProjectSearch::new();
        assert!(!ps.case_sensitive && !ps.regex);
        ps.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::ALT));
        assert!(ps.case_sensitive);
        ps.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT));
        assert!(ps.regex);
    }

    #[test]
    fn clicking_the_buttons_toggles_the_flags_and_leaves_the_row_hit_test_alone() {
        let mut ps = ProjectSearch::new();
        // Same rects `render` would have stashed — an [Aa] button at (5,1)
        // and a [.*] button at (10,1), inside an outer rect starting at (0,0).
        ps.case_rect = Some(Rect {
            x: 5,
            y: 1,
            width: 4,
            height: 1,
        });
        ps.regex_rect = Some(Rect {
            x: 10,
            y: 1,
            width: 4,
            height: 1,
        });
        let outer = Rect {
            x: 0,
            y: 0,
            width: 40,
            height: 10,
        };

        assert!(matches!(
            ps.click(outer, 5, 1),
            ProjectSearchAction::Requery
        ));
        assert!(ps.case_sensitive);
        assert!(matches!(
            ps.click(outer, 10, 1),
            ProjectSearchAction::Requery
        ));
        assert!(ps.regex);

        // A click elsewhere on that row still falls through to row selection,
        // not a button.
        assert!(matches!(ps.click(outer, 30, 1), ProjectSearchAction::Stay));
    }
}
