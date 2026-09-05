//! `F8`: the Projects picker — new / open / delete, PyCharm-style.
//!
//! Row 0 and 1 are always the "+ New Project…" / "+ Open Project…" actions
//! (both hand off to the existing path prompt — `PromptKind::NewProject` /
//! `PromptKind::Open`); the rest are recent project directories. `Delete`
//! on a recent row opens [`DeleteConfirm`], which requires **typing the
//! folder's own name** to arm the button — Enter alone does nothing, same
//! spirit as GitHub's "type the repo name to confirm" delete dialog. The
//! actual filesystem guardrails (never the cwd, `$HOME`, a filesystem root,
//! or the currently-open project) live in `App::delete_project`, not here —
//! this module only decides what the user typed matches what was asked.

use std::path::PathBuf;

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Position as TermPos, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph};

use super::overlay::centered_rect;
use crate::buffer::Buffer;
use crate::theme::Theme;

const MAX_ROWS: usize = 12;
/// Rows before the recent-projects list: "+ New Project…", "+ Open Project…".
const ACTION_ROWS: usize = 2;

pub enum ProjectsAction {
    Stay,
    Cancel,
    NewProject,
    OpenProjectPrompt,
    OpenRecent(PathBuf),
    DeleteRecent(PathBuf),
}

pub struct ProjectsPicker {
    recent: Vec<PathBuf>,
    selected: usize,
}

impl ProjectsPicker {
    pub fn new(recent: Vec<PathBuf>) -> Self {
        Self {
            recent,
            selected: 0,
        }
    }

    fn row_count(&self) -> usize {
        ACTION_ROWS + self.recent.len()
    }

    fn activate(&self, row: usize) -> ProjectsAction {
        match row {
            0 => ProjectsAction::NewProject,
            1 => ProjectsAction::OpenProjectPrompt,
            i => self
                .recent
                .get(i - ACTION_ROWS)
                .map(|p| ProjectsAction::OpenRecent(p.clone()))
                .unwrap_or(ProjectsAction::Stay),
        }
    }

    fn delete(&self, row: usize) -> ProjectsAction {
        match row.checked_sub(ACTION_ROWS) {
            Some(i) => self
                .recent
                .get(i)
                .map(|p| ProjectsAction::DeleteRecent(p.clone()))
                .unwrap_or(ProjectsAction::Stay),
            None => ProjectsAction::Stay, // an action row has nothing to delete
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> ProjectsAction {
        match key.code {
            KeyCode::Esc => ProjectsAction::Cancel,
            KeyCode::Enter => self.activate(self.selected),
            KeyCode::Delete => self.delete(self.selected),
            KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                ProjectsAction::Stay
            }
            KeyCode::Down => {
                if self.selected + 1 < self.row_count() {
                    self.selected += 1;
                }
                ProjectsAction::Stay
            }
            _ => ProjectsAction::Stay,
        }
    }

    fn row_at(&self, outer: Rect, row: u16) -> Option<usize> {
        let body_top = outer.y + 1;
        if row < body_top {
            return None;
        }
        let idx = (row - body_top) as usize;
        (idx < self.row_count()).then_some(idx)
    }

    /// A left click at `row`: select and immediately activate, same as Enter.
    pub fn click(&mut self, outer: Rect, row: u16) -> ProjectsAction {
        match self.row_at(outer, row) {
            Some(idx) => {
                self.selected = idx;
                self.activate(idx)
            }
            None => ProjectsAction::Stay,
        }
    }

    pub fn scroll(&mut self, delta: isize) {
        let n = self.row_count() as isize;
        if n == 0 {
            return;
        }
        self.selected = (self.selected as isize + delta).clamp(0, n - 1) as usize;
    }
}

/// Returns the outer rect it drew into (for click-away hit-testing).
pub fn render(f: &mut Frame, p: &ProjectsPicker, theme: &Theme, area: Rect) -> Rect {
    let rows = p.row_count().clamp(1, MAX_ROWS) as u16;
    let rect = centered_rect(64, rows + 3, area);
    f.render_widget(Clear, rect);

    let panel = Style::default().fg(theme.menu_fg).bg(theme.menu_bg);
    let accent = Style::default()
        .fg(theme.accent)
        .bg(theme.menu_bg)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(theme.dock_fg).bg(theme.menu_bg);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent).bg(theme.menu_bg))
        .title(Span::styled(
            " Projects — ↑↓ select, Enter open, Delete remove ",
            accent,
        ))
        .style(panel);
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let top = p.selected.saturating_sub(MAX_ROWS - 1);
    let mut lines = Vec::new();
    for row in top..p.row_count().min(top + MAX_ROWS) {
        let selected = row == p.selected;
        let style = if selected {
            Style::default()
                .fg(theme.autocomplete_fg)
                .bg(theme.autocomplete_sel)
                .add_modifier(Modifier::BOLD)
        } else {
            panel
        };
        let marker = if selected { "▸ " } else { "  " };
        let label = match row {
            0 => "+ New Project…".to_string(),
            1 => "+ Open Project…".to_string(),
            i => p
                .recent
                .get(i - ACTION_ROWS)
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
        };
        let budget = (inner.width as usize).saturating_sub(marker.chars().count());
        let text = if label.chars().count() > budget && budget > 1 {
            let head: String = label.chars().take(budget - 1).collect();
            format!("{head}…")
        } else {
            label
        };
        lines.push(Line::from(Span::styled(format!("{marker}{text}"), style)));
    }
    if p.recent.is_empty() {
        lines.push(Line::from(Span::styled("  (no recent projects yet)", dim)));
    }
    f.render_widget(Paragraph::new(lines).style(panel), inner);
    super::sidebar_scrollbar(f, theme, rect, p.row_count(), top);
    rect
}

// ---- delete confirmation ("type the folder name to confirm") ----

pub enum DeleteOutcome {
    Stay,
    Cancel,
    /// The typed name matched — the app should actually delete this path.
    Confirmed(PathBuf),
}

pub struct DeleteConfirm {
    pub path: PathBuf,
    /// What has to be typed exactly (the directory's own name) to arm delete.
    expected: String,
    input: Buffer,
    pub error: Option<String>,
}

impl DeleteConfirm {
    pub fn new(path: PathBuf) -> Self {
        let expected = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        Self {
            path,
            expected,
            input: Buffer::new(),
            error: None,
        }
    }

    pub fn text(&self) -> String {
        self.input.rope().to_string()
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> DeleteOutcome {
        match key.code {
            KeyCode::Esc => DeleteOutcome::Cancel,
            KeyCode::Enter => {
                if self.text() == self.expected {
                    DeleteOutcome::Confirmed(self.path.clone())
                } else {
                    self.error = Some(format!("type \"{}\" exactly to confirm", self.expected));
                    DeleteOutcome::Stay
                }
            }
            KeyCode::Backspace => {
                self.input.delete_backward();
                self.error = None;
                DeleteOutcome::Stay
            }
            KeyCode::Delete => {
                self.input.delete_forward();
                self.error = None;
                DeleteOutcome::Stay
            }
            KeyCode::Left => {
                self.input.move_left(false);
                DeleteOutcome::Stay
            }
            KeyCode::Right => {
                self.input.move_right(false);
                DeleteOutcome::Stay
            }
            KeyCode::Home => {
                self.input.move_home(false);
                DeleteOutcome::Stay
            }
            KeyCode::End => {
                self.input.move_end(false);
                DeleteOutcome::Stay
            }
            KeyCode::Char(c) => {
                self.input.insert_char(c);
                self.error = None;
                DeleteOutcome::Stay
            }
            _ => DeleteOutcome::Stay,
        }
    }
}

pub fn render_delete(f: &mut Frame, d: &DeleteConfirm, theme: &Theme, area: Rect) -> Rect {
    let extra = if d.error.is_some() { 2 } else { 0 };
    let rect = centered_rect(66, 9 + extra, area);
    f.render_widget(Clear, rect);

    let panel = Style::default().fg(theme.fg).bg(theme.statusbar_bg);
    let muted = Style::default()
        .fg(theme.statusbar_fg)
        .bg(theme.statusbar_bg);
    let warn = Style::default()
        .fg(theme.output_err)
        .bg(theme.statusbar_bg)
        .add_modifier(Modifier::BOLD);
    let block = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::symmetric(2, 1))
        .border_style(warn)
        .title(Span::styled(
            " Delete Project — This Cannot Be Undone ",
            warn,
        ))
        .style(panel);
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let mut lines = vec![
        Line::from(Span::styled(
            format!("This permanently deletes {}", d.path.display()),
            panel,
        )),
        Line::from(Span::styled(
            "and everything inside it. This is not a trash-can move.",
            panel,
        )),
        Line::default(),
        Line::from(vec![
            Span::styled("Type ", muted),
            Span::styled(format!("\"{}\"", d.expected), warn),
            Span::styled(" to confirm:  ", muted),
            Span::styled(d.text(), panel),
        ]),
    ];
    if let Some(err) = &d.error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(err.clone(), warn)));
    }
    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        "Enter confirm · Esc cancel",
        muted,
    )));
    f.render_widget(Paragraph::new(lines).style(panel), inner);

    let label_w = "Type \"\" to confirm:  ".chars().count() + d.expected.chars().count();
    let cx = inner.x + label_w as u16 + d.text().chars().count() as u16;
    let cy = inner.y + 3;
    if cx < inner.x + inner.width {
        f.set_cursor_position(TermPos::new(cx, cy));
    }
    rect
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyModifiers;

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    #[test]
    fn enter_on_action_rows_returns_the_right_action() {
        let mut p = ProjectsPicker::new(vec![PathBuf::from("/tmp/x")]);
        assert!(matches!(
            p.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            ProjectsAction::NewProject
        ));
        p.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert!(matches!(
            p.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            ProjectsAction::OpenProjectPrompt
        ));
    }

    #[test]
    fn enter_on_a_recent_row_opens_it() {
        let mut p = ProjectsPicker::new(vec![PathBuf::from("/tmp/proj")]);
        p.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        p.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        match p.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)) {
            ProjectsAction::OpenRecent(path) => assert_eq!(path, PathBuf::from("/tmp/proj")),
            _ => panic!("expected OpenRecent"),
        }
    }

    #[test]
    fn delete_key_on_an_action_row_does_nothing() {
        let mut p = ProjectsPicker::new(vec![PathBuf::from("/tmp/proj")]);
        assert!(matches!(
            p.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE)),
            ProjectsAction::Stay
        ));
    }

    #[test]
    fn delete_key_on_a_recent_row_targets_it() {
        let mut p = ProjectsPicker::new(vec![PathBuf::from("/tmp/proj")]);
        p.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        p.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        match p.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE)) {
            ProjectsAction::DeleteRecent(path) => assert_eq!(path, PathBuf::from("/tmp/proj")),
            _ => panic!("expected DeleteRecent"),
        }
    }

    #[test]
    fn wrong_typed_name_stays_open_with_an_error() {
        let mut d = DeleteConfirm::new(PathBuf::from("/tmp/my-project"));
        for c in "not-the-name".chars() {
            d.handle_key(key(c));
        }
        assert!(matches!(
            d.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            DeleteOutcome::Stay
        ));
        assert!(d.error.is_some());
    }

    #[test]
    fn exact_typed_name_confirms() {
        let mut d = DeleteConfirm::new(PathBuf::from("/tmp/my-project"));
        for c in "my-project".chars() {
            d.handle_key(key(c));
        }
        match d.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)) {
            DeleteOutcome::Confirmed(path) => assert_eq!(path, PathBuf::from("/tmp/my-project")),
            _ => panic!("expected Confirmed"),
        }
    }

    #[test]
    fn esc_cancels_without_requiring_the_typed_name() {
        let mut d = DeleteConfirm::new(PathBuf::from("/tmp/my-project"));
        assert!(matches!(
            d.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            DeleteOutcome::Cancel
        ));
    }
}
