//! Top-level application state and the event loop.
//!
//! One `App` owns everything the UI reads. Widgets are pure functions of it.
//! Phase 3: `buffers` is a `Vec` behind a tab strip; a `Config` loaded from
//! `~/.config/vulide/config.toml` drives editor behaviour.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::Duration;

use anyhow::Result;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Rect;

use crate::buffer::Buffer;
use crate::complete::Completion;
use crate::config::Config;
use crate::event::{AppEvent, EventSource};
use crate::run::{self, RunConsole};
use crate::search::{Field, Search, SearchAction};
use crate::theme::Theme;
use crate::ui;
use crate::ui::goto_line::{GotoLine, GotoOutcome};
use crate::ui::help::HelpOutcome;
use crate::ui::overlay::{
    Confirm, ConfirmAction, ConfirmOutcome, Overlay, PathPrompt, PromptKind, PromptOutcome,
    expand_tilde,
};
use crate::ui::palette::{Cmd, Entry, Palette, PaletteOutcome};
use crate::ui::project_search::ProjectSearchAction;
use crate::ui::projects::{DeleteConfirm, DeleteOutcome, ProjectsAction, ProjectsPicker};
use crate::ui::snippet_picker::SnippetOutcome;
use crate::ui::tabs::TabHit;
use crate::ui::theme_picker::{ThemePicker, ThemePickerOutcome};

/// Input poll granularity for the reader thread (not an output-latency bound).
const TICK: Duration = Duration::from_millis(100);

/// Which pane keystrokes go to when no overlay is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Editor,
    Output,
    /// The structure-outline sidebar (`F7`).
    Algo,
    /// The file-tree sidebar (`F2`).
    Files,
}

pub struct App {
    pub buffers: Vec<Buffer>,
    pub active: usize,
    pub config: Config,
    pub theme: Theme,
    pub themes: Vec<Theme>,
    pub theme_idx: usize,
    pub status: String,
    pub editor_rows: usize,
    pub overlay: Overlay,
    /// The live autocomplete popup, recomputed after every editing key.
    pub completion: Option<Completion>,
    /// The run-output console, present once the file has been run at least once
    /// this session (until explicitly closed).
    pub run: Option<RunConsole>,
    pub focus: Focus,
    /// User-dragged output-panel height (rows); `None` = the default third.
    pub panel_height: Option<u16>,
    pub dragging_splitter: bool,

    /// Undefined-variable warnings, recomputed each draw.
    pub diagnostics: Vec<crate::lint::Diagnostic>,

    // ---- structure outline (F7) ----
    pub show_algo: bool,
    pub algo_items: Vec<crate::algo::Item>,
    pub algo_selected: usize,
    /// Index of the first outline row drawn (kept so mouse clicks map to rows).
    pub algo_scroll: usize,
    pub algo_rect: Option<Rect>,

    // ---- file tree (F2) ----
    pub show_files: bool,
    /// Built when the sidebar first opens; never rebuilt per frame (it is disk
    /// I/O). Mutated on expand/collapse/refresh.
    pub file_tree: Option<crate::filetree::FileTree>,
    pub files_selected: usize,
    /// Index of the first tree row drawn (kept so mouse clicks map to rows).
    pub files_scroll: usize,
    pub files_rect: Option<Rect>,
    /// The path the tree last auto-revealed, so a file switch reveals once.
    pub files_revealed: Option<PathBuf>,

    // ---- find / replace ----
    /// The find/replace bar, present while it is open.
    pub search: Option<Search>,
    /// Every match of the current query, ordered; drives editor highlighting.
    pub search_matches: Vec<(crate::buffer::Position, crate::buffer::Position)>,
    /// Index into `search_matches` of the current (emphasised) match.
    pub search_idx: usize,
    /// Cursor position when the bar opened — incremental find anchors here so
    /// typing more of the query doesn't walk the selection forward.
    search_origin: crate::buffer::Position,

    // ---- mouse hit rects, refreshed every draw ----
    /// Screen rect of the status-bar ▶/■ button.
    pub run_button: Option<Rect>,
    pub editor_rect: Rect,
    pub status_rect: Rect,
    pub splitter_rect: Option<Rect>,
    /// Screen rect of the output panel (when shown).
    pub panel_rect: Option<Rect>,
    /// Screen rect of the panel's close-✕ button.
    pub panel_close_rect: Option<Rect>,
    /// Per-tab hit rects (index, tab, close-✕).
    pub tab_hits: Vec<TabHit>,
    /// Screen rect of the open overlay's box (for click-away dismiss).
    pub overlay_rect: Option<Rect>,
    /// Screen rect of the find/replace bar (when open).
    pub search_rect: Option<Rect>,

    // ---- hover state (mouse-move driven) ----
    pub hovered_tab: Option<usize>,
    pub hover_splitter: bool,
    pub hover_panel_close: bool,
    /// Channel the run console's reader threads push onto; set while `run()` owns
    /// the loop. `None` outside it (e.g. in tests, unless injected).
    run_tx: Option<Sender<AppEvent>>,
    should_quit: bool,
    /// The terminal window/tab title last written, so we only emit the OSC
    /// escape when it actually changes.
    title_shown: String,
    /// Opened once and kept alive for the app's lifetime. On X11, dropping an
    /// `arboard::Clipboard` right after `set_text` races the background
    /// thread that serves the paste request to other apps — arboard warns
    /// about this straight to stderr, which (with the terminal in raw / the
    /// alternate screen) corrupts the display instead of just logging.
    /// `None` after a failed open (no display server) so we don't retry
    /// every keystroke; `Err` == tried and failed, cached so `Ctrl+C`/`Ctrl+V`
    /// give one clean status message instead of hammering `Clipboard::new()`.
    clipboard: Option<Result<arboard::Clipboard, String>>,
}

impl App {
    pub fn new() -> Self {
        let (config, warning) = Config::load();
        let mut app = Self::with_config(config);
        if let Some(w) = warning {
            app.set_status(w);
        }
        app
    }

    /// Build an app around an explicit config, skipping the disk read. Tests use
    /// this so a developer's real `~/.config/vulide/config.toml` can't sway them.
    pub fn with_config(config: Config) -> Self {
        let themes = Theme::builtins();
        let theme_idx = themes
            .iter()
            .position(|t| t.name == config.theme)
            .unwrap_or(0);
        let theme = themes[theme_idx].clone();
        let show_algo = config.show_algo;
        let show_files = config.show_files;

        let mut app = Self {
            buffers: vec![Buffer::new()],
            active: 0,
            config,
            theme,
            themes,
            theme_idx,
            status: String::new(),
            editor_rows: 20,
            overlay: Overlay::None,
            completion: None,
            run: None,
            focus: Focus::Editor,
            panel_height: None,
            dragging_splitter: false,
            diagnostics: Vec::new(),
            show_algo,
            algo_items: Vec::new(),
            algo_selected: 0,
            algo_scroll: 0,
            algo_rect: None,
            show_files,
            file_tree: None,
            files_selected: 0,
            files_scroll: 0,
            files_rect: None,
            files_revealed: None,
            search: None,
            search_matches: Vec::new(),
            search_idx: 0,
            search_origin: crate::buffer::Position::default(),
            run_button: None,
            editor_rect: Rect::default(),
            status_rect: Rect::default(),
            splitter_rect: None,
            panel_rect: None,
            panel_close_rect: None,
            tab_hits: Vec::new(),
            overlay_rect: None,
            search_rect: None,
            hovered_tab: None,
            hover_splitter: false,
            hover_panel_close: false,
            run_tx: None,
            should_quit: false,
            title_shown: String::new(),
            clipboard: None,
        };
        app.apply_config();
        // The outline is rebuilt every draw, but the file tree is disk I/O and
        // is built on demand — so a persisted `show_files = true` has to build it
        // now, or the sidebar would open empty until toggled a few times.
        if app.show_files {
            let root = app.file_tree_root();
            app.file_tree = Some(crate::filetree::FileTree::new(&root));
        }
        app
    }

    // ---- buffer access ----

    pub fn buf(&self) -> &Buffer {
        &self.buffers[self.active]
    }

    pub fn buf_mut(&mut self) -> &mut Buffer {
        &mut self.buffers[self.active]
    }

    /// Push the config's editor settings onto every open buffer.
    fn apply_config(&mut self) {
        let c = &self.config;
        for b in &mut self.buffers {
            b.tab_width = c.tab_width;
            b.auto_close_brackets = c.auto_close_brackets;
            b.auto_indent = c.auto_indent;
        }
    }

    fn save_config(&mut self) {
        // Never write a real config file from a test run.
        if cfg!(test) {
            return;
        }
        if let Err(e) = self.config.save() {
            self.set_status(format!("config not saved: {e}"));
        }
    }

    // ---- theme ----

    /// Open the theme picker (Ctrl+T), selection starting on the active theme.
    pub fn open_theme_picker(&mut self) {
        let names = self.themes.iter().map(|t| t.name.clone()).collect();
        self.overlay = Overlay::ThemePicker(Box::new(ThemePicker::new(names, self.theme_idx)));
    }

    /// `F3` — snippets are Vulpin syntax, so this is a no-op (with a status
    /// message) everywhere else.
    fn open_snippet_picker(&mut self) {
        if self.buf().language() != crate::syntax::Language::Vulpin {
            self.set_status("snippets are Vulpin-only");
            return;
        }
        self.overlay = Overlay::SnippetPicker(Box::default());
    }

    /// `Ctrl+G` — jump the cursor to a 1-based line number.
    fn open_goto_line(&mut self) {
        self.overlay = Overlay::GotoLine(Box::new(GotoLine::new(self.buf().line_count())));
    }

    fn goto_line(&mut self, n: usize) {
        let line = n.saturating_sub(1);
        self.buf_mut()
            .set_cursor(crate::buffer::Position { line, col: 0 }, false);
        self.set_status(format!("line {n}"));
    }

    /// `F4` — find in files, rooted at the open file tree (or the active
    /// file's directory / cwd, same fallback as the file tree itself).
    fn open_project_search(&mut self) {
        self.overlay = Overlay::ProjectSearch(Box::default());
        self.set_status("find in files: type to search · Enter opens · Esc closes");
    }

    /// The directory a project-wide operation (search, later: "Projects")
    /// should root itself at — the open file tree if there is one, else
    /// wherever `F2` would open it.
    pub fn project_root(&self) -> PathBuf {
        self.file_tree
            .as_ref()
            .map(|t| t.root.clone())
            .unwrap_or_else(|| self.file_tree_root())
    }

    fn recompute_project_matches(&mut self) {
        let root = self.project_root();
        let Overlay::ProjectSearch(ps) = &mut self.overlay else {
            return;
        };
        let r = crate::ui::project_search::run(&root, &ps.query(), ps.case_sensitive, ps.regex);
        ps.selected = ps.selected.min(r.matches.len().saturating_sub(1));
        ps.matches = r.matches;
        ps.truncated = r.truncated;
        ps.error = r.error;
    }

    /// Open `path` (reusing its tab if already open) and place the cursor at
    /// a match's position — shared by `Enter` and a click in project search.
    fn jump_to_match(&mut self, path: PathBuf, line: usize, col: usize) {
        if let Err(e) = self.open_file(path) {
            self.set_status(format!("open failed: {e}"));
            return;
        }
        self.buf_mut()
            .set_cursor(crate::buffer::Position { line, col }, false);
    }

    /// Swap the active theme without touching the config (live preview).
    fn preview_theme(&mut self, idx: usize) {
        self.theme_idx = idx.min(self.themes.len() - 1);
        self.theme = self.themes[self.theme_idx].clone();
    }

    /// Preview + persist to config.
    fn set_theme(&mut self, idx: usize) {
        self.preview_theme(idx);
        self.config.theme = self.theme.name.clone();
        self.save_config();
    }

    fn set_theme_by_name(&mut self, name: &str) {
        if let Some(i) = self.themes.iter().position(|t| t.name == name) {
            self.set_theme(i);
            self.set_status(format!("theme: {}", self.theme.name));
        }
    }

    // ---- tabs / files ----

    pub fn open_path(&mut self, path: PathBuf) -> Result<()> {
        self.open_file(path)?;
        Ok(())
    }

    /// Reopen the files from `$XDG_STATE_HOME/vulide/session.toml`. Called from
    /// `main` only when no file was passed on the command line.
    pub fn restore_session(&mut self) {
        if !self.config.restore_session {
            return;
        }
        let saved = crate::session::Session::load();
        let mut opened = 0usize;
        for p in saved.files {
            if p.is_file() && self.open_file(p).is_ok() {
                opened += 1;
            }
        }
        if opened > 0 {
            self.active = saved.active.min(self.buffers.len() - 1);
            self.set_status(format!(
                "restored {opened} file{}",
                if opened == 1 { "" } else { "s" }
            ));
        }
    }

    /// The open files + active tab, as a `Session`. Pure — the caller persists.
    pub(crate) fn session_state(&self) -> crate::session::Session {
        let files: Vec<PathBuf> = self
            .buffers
            .iter()
            .filter_map(|b| b.path().map(Path::to_path_buf))
            .collect();
        // Active index counted among the saved (path-bearing) buffers only.
        let active = self.buffers[..self.active]
            .iter()
            .filter(|b| b.path().is_some())
            .count();
        crate::session::Session { files, active }
    }

    /// Write the current session to disk. Called once as the event loop exits.
    pub(crate) fn persist_session(&mut self) {
        if !self.config.restore_session {
            return;
        }
        if let Err(e) = self.session_state().save() {
            self.set_status(format!("session not saved: {e}"));
        }
    }

    fn open_file(&mut self, path: PathBuf) -> io::Result<()> {
        // A directory isn't a buffer — root the file tree there instead of
        // trying (and failing) to read it as text. Covers Ctrl+O, the CLI
        // file argument, and anything else that funnels through here.
        if path.is_dir() {
            self.open_dir(path);
            return Ok(());
        }

        let canonical = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());

        if let Some(i) = self
            .buffers
            .iter()
            .position(|b| b.path().map(|p| p == canonical) == Some(true))
        {
            self.active = i;
            self.set_status(format!("switched to {}", self.buffers[i].title()));
            return Ok(());
        }

        let buf = Buffer::open(&path)?;
        // Replace a pristine lone "untitled" buffer instead of stacking a tab.
        if self.buffers.len() == 1 && self.buf().path().is_none() && !self.buf().is_dirty() {
            self.buffers[0] = buf;
            self.active = 0;
        } else {
            self.buffers.push(buf);
            self.active = self.buffers.len() - 1;
        }
        self.apply_config();
        self.completion = None;
        self.dismiss_search_on_switch();
        self.set_status(format!("opened {}", self.buf().title()));

        self.config.push_recent(&canonical);
        self.save_config();
        Ok(())
    }

    /// Point the file-tree sidebar at `path` and show it — what "opening" a
    /// directory means, as opposed to opening a file into a tab.
    fn open_dir(&mut self, path: PathBuf) {
        let canonical = std::fs::canonicalize(&path).unwrap_or(path);
        self.file_tree = Some(crate::filetree::FileTree::new(&canonical));
        self.files_selected = 0;
        self.files_scroll = 0;
        self.files_revealed = None;
        self.show_files = true;
        self.focus = Focus::Files;
        self.config.show_files = true;
        self.config.push_recent_project(&canonical);
        self.save_config();
        self.set_status(format!("file tree: {}", canonical.display()));
    }

    /// `F8` Projects picker › "+ New Project…". An existing directory is just
    /// opened (never scaffolded into — it might already hold real files); a
    /// path that doesn't exist yet is created with a starter `main.vul`.
    fn create_project(&mut self, path: &str) -> io::Result<()> {
        let target = PathBuf::from(path);
        if target.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "that path is a file, not a directory",
            ));
        }
        if !target.exists() {
            std::fs::create_dir_all(&target)?;
            let name = target
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "project".to_string());
            std::fs::write(
                target.join("main.vul"),
                format!("G\"Hello from {name}!\"\n"),
            )?;
        }
        self.open_dir(target);
        Ok(())
    }

    /// `F8` — the Projects picker: new / open / delete, seeded with the
    /// directories most recently opened as a project.
    fn open_projects(&mut self) {
        // Stale entries (moved or deleted since last time) are just noise —
        // and "opening" one would land on an empty, confusing file tree.
        self.config.recent_projects.retain(|p| p.is_dir());
        self.save_config();
        let recent = self.config.recent_projects.clone();
        self.overlay = Overlay::Projects(Box::new(ProjectsPicker::new(recent)));
    }

    /// A `Delete` in the Projects picker opens the typed-confirmation dialog
    /// — the actual deletion only happens if `DeleteOutcome::Confirmed` comes
    /// back from it (see `handle_overlay_key`).
    fn request_delete_project(&mut self, path: PathBuf) {
        self.overlay = Overlay::DeleteProject(Box::new(DeleteConfirm::new(path)));
    }

    /// Refuse to `remove_dir_all` a handful of catastrophic targets. Checked
    /// against the canonicalized path so `..`/symlinks can't sneak past it.
    fn refuse_delete(&self, canonical: &Path) -> Option<String> {
        if let Ok(cwd) = std::env::current_dir()
            && cwd.starts_with(canonical)
        {
            return Some(
                "refusing to delete: that's the current working directory (or contains it)"
                    .to_string(),
            );
        }
        if let Some(home) = std::env::var_os("HOME")
            && canonical == Path::new(&home)
        {
            return Some("refusing to delete your home directory".to_string());
        }
        if canonical.parent().is_none() {
            return Some("refusing to delete a filesystem root".to_string());
        }
        if self.file_tree.as_ref().is_some_and(|t| t.root == canonical) {
            return Some(
                "open a different file or folder before deleting this project".to_string(),
            );
        }
        None
    }

    /// The actual `rm -rf`, only ever reached after `DeleteConfirm` has
    /// already required typing the folder's own name.
    fn delete_project(&mut self, path: PathBuf) {
        let Ok(canonical) = std::fs::canonicalize(&path) else {
            // Already gone — just stop tracking it.
            self.config.recent_projects.retain(|p| p != &path);
            self.save_config();
            self.set_status("that project no longer exists — removed from the list");
            return;
        };
        if let Some(reason) = self.refuse_delete(&canonical) {
            self.set_status(reason);
            return;
        }
        match std::fs::remove_dir_all(&canonical) {
            Ok(()) => {
                self.config.recent_projects.retain(|p| p != &canonical);
                self.save_config();
                self.set_status(format!("deleted {}", canonical.display()));
            }
            Err(e) => self.set_status(format!("delete failed: {e}")),
        }
    }

    fn new_tab(&mut self) {
        self.buffers.push(Buffer::new());
        self.active = self.buffers.len() - 1;
        self.apply_config();
        self.completion = None;
        self.dismiss_search_on_switch();
        self.set_status("new buffer");
    }

    fn close_tab(&mut self, discard: bool) {
        self.close_tab_at(self.active, discard);
    }

    fn close_tab_at(&mut self, index: usize, discard: bool) {
        if index >= self.buffers.len() {
            return;
        }
        if self.buffers[index].is_dirty() && !discard {
            self.set_status("unsaved changes — save (Ctrl+S) or use palette › Close Tab (discard)");
            return;
        }
        self.buffers.remove(index);
        if self.buffers.is_empty() {
            self.buffers.push(Buffer::new());
            self.apply_config();
        }
        // Keep `active` pointing at the same buffer (or the nearest one).
        if self.active > index || self.active >= self.buffers.len() {
            self.active = self.active.saturating_sub(1).min(self.buffers.len() - 1);
        }
        self.completion = None;
        self.dismiss_search_on_switch();
        self.set_status("closed tab");
    }

    fn next_tab(&mut self) {
        self.active = (self.active + 1) % self.buffers.len();
        self.completion = None;
        self.dismiss_search_on_switch();
    }

    fn prev_tab(&mut self) {
        self.active = (self.active + self.buffers.len() - 1) % self.buffers.len();
        self.completion = None;
        self.dismiss_search_on_switch();
    }

    /// The find bar's match list is per-buffer — drop it when the active buffer
    /// changes (tab switch/close/open) so stale highlights can't linger.
    fn dismiss_search_on_switch(&mut self) {
        if self.search.take().is_some() {
            self.search_matches.clear();
            self.search_idx = 0;
        }
    }

    fn save_active(&mut self) {
        if self.buf().path().is_none() {
            self.overlay = Overlay::Prompt(Box::new(PathPrompt::save(&default_save_seed())));
            return;
        }
        let msg = match self.buf_mut().save() {
            Ok(()) => format!("saved {}", self.buf().title()),
            Err(e) => format!("save failed: {e}"),
        };
        self.set_status(msg);
    }

    // ---- quit ----

    /// Ctrl+Q / Ctrl+C / palette Quit route here. Quits straight away unless a
    /// buffer has unsaved changes, in which case it asks first.
    fn request_quit(&mut self) {
        let dirty = self.buffers.iter().filter(|b| b.is_dirty()).count();
        if dirty == 0 {
            self.should_quit = true;
            return;
        }
        let message = if dirty == 1 {
            "1 file has unsaved changes.".to_string()
        } else {
            format!("{dirty} files have unsaved changes.")
        };
        self.overlay = Overlay::Confirm(Box::new(Confirm::quit_unsaved(message)));
    }

    /// The quit-guard's "Y" (save then quit). Saves every buffer that already
    /// has a path; if an untitled buffer is still dirty it switches there and
    /// opens Save As instead of quitting.
    fn confirm_yes_quit(&mut self) {
        let mut untitled: Option<usize> = None;
        for i in 0..self.buffers.len() {
            if !self.buffers[i].is_dirty() {
                continue;
            }
            if self.buffers[i].path().is_some() {
                if let Err(e) = self.buffers[i].save() {
                    self.set_status(format!("save failed: {e} — not quitting"));
                    return;
                }
            } else if untitled.is_none() {
                untitled = Some(i);
            }
        }
        if let Some(i) = untitled {
            self.active = i;
            self.overlay = Overlay::Prompt(Box::new(PathPrompt::save(&default_save_seed())));
            self.set_status("name this file, then quit again");
            return;
        }
        self.should_quit = true;
    }

    // ---- command palette ----

    fn open_palette(&mut self) {
        let mut entries = vec![
            Entry::new("Save", Cmd::Save),
            Entry::new("Save As…", Cmd::SaveAs),
            Entry::new("Open File…", Cmd::OpenFile),
            Entry::new("Find…", Cmd::Find),
            Entry::new("Replace…", Cmd::Replace),
            Entry::new("Find in Files… (F4)", Cmd::FindInFiles),
            Entry::new("Go to Line… (Ctrl+G)", Cmd::GotoLine),
            Entry::new("Projects — New / Open / Delete… (F8)", Cmd::Projects),
            Entry::new("New Tab", Cmd::NewTab),
            Entry::new("Close Tab", Cmd::CloseTab),
            Entry::new("Close Tab (discard changes)", Cmd::CloseTabDiscard),
            Entry::new("Next Tab", Cmd::NextTab),
            Entry::new("Previous Tab", Cmd::PrevTab),
            Entry::new("Choose Theme…", Cmd::ChooseTheme),
            Entry::new("Toggle Line Numbers", Cmd::ToggleLineNumbers),
            Entry::new("Toggle Word Wrap", Cmd::ToggleWordWrap),
            Entry::new("Toggle Auto-close Brackets", Cmd::ToggleAutoClose),
            Entry::new("Toggle Structure Outline (F7)", Cmd::ToggleOutline),
            Entry::new("Toggle File Tree (F2)", Cmd::ToggleFileTree),
            Entry::new("Toggle Session Restore", Cmd::ToggleSessionRestore),
            Entry::new(
                "Toggle Mouse (for terminal text selection)",
                Cmd::ToggleMouse,
            ),
            Entry::new("Run File (F5)", Cmd::RunFile),
            Entry::new("Stop Run", Cmd::StopRun),
            Entry::new("Close Output Panel", Cmd::CloseOutput),
            Entry::new("Help — Keys & Shortcuts (F1)", Cmd::Help),
            Entry::new("Reload Config", Cmd::ReloadConfig),
            Entry::new("Quit", Cmd::Quit),
        ];
        for t in &self.themes {
            entries.push(Entry::new(
                format!("Theme: {}", t.name),
                Cmd::SetTheme(t.name.clone()),
            ));
        }
        for p in &self.config.recent_files {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.display().to_string());
            entries.push(Entry::new(
                format!("Open Recent: {name}"),
                Cmd::OpenRecent(p.clone()),
            ));
        }
        // Snippets are Vulpin syntax — only useful (and only offered) on a
        // Vulpin buffer.
        if self.buf().language() == crate::syntax::Language::Vulpin {
            for s in crate::snippets::SNIPPETS {
                entries.push(Entry::new(
                    format!("Snippet: {}", s.name),
                    Cmd::InsertSnippet(s.body),
                ));
            }
        }
        self.overlay = Overlay::Palette(Box::new(Palette::new(entries)));
    }

    fn run_command(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Quit => self.request_quit(),
            Cmd::Save => self.save_active(),
            Cmd::SaveAs => {
                self.overlay = Overlay::Prompt(Box::new(PathPrompt::save(&default_save_seed())));
            }
            Cmd::OpenFile => {
                self.overlay = Overlay::Prompt(Box::new(PathPrompt::open(&default_save_seed())));
            }
            Cmd::NewTab => self.new_tab(),
            Cmd::CloseTab => self.close_tab(false),
            Cmd::CloseTabDiscard => self.close_tab(true),
            Cmd::NextTab => self.next_tab(),
            Cmd::PrevTab => self.prev_tab(),
            Cmd::ChooseTheme => self.open_theme_picker(),
            Cmd::SetTheme(name) => self.set_theme_by_name(&name),
            Cmd::ToggleLineNumbers => {
                self.config.show_line_numbers = !self.config.show_line_numbers;
                self.save_config();
                self.set_status(format!(
                    "line numbers: {}",
                    on_off(self.config.show_line_numbers)
                ));
            }
            Cmd::ToggleWordWrap => {
                self.config.word_wrap = !self.config.word_wrap;
                self.save_config();
                self.set_status(format!("word wrap: {}", on_off(self.config.word_wrap)));
            }
            Cmd::ToggleSessionRestore => {
                self.config.restore_session = !self.config.restore_session;
                self.save_config();
                self.set_status(format!(
                    "restore session on launch: {}",
                    on_off(self.config.restore_session)
                ));
            }
            Cmd::ToggleAutoClose => {
                self.config.auto_close_brackets = !self.config.auto_close_brackets;
                self.apply_config();
                self.save_config();
                self.set_status(format!(
                    "auto-close brackets: {}",
                    on_off(self.config.auto_close_brackets)
                ));
            }
            Cmd::ToggleOutline => self.toggle_algo(),
            Cmd::ToggleFileTree => self.toggle_files(),
            Cmd::ToggleMouse => {
                self.config.mouse = !self.config.mouse;
                self.save_config();
                self.apply_mouse_capture();
                self.set_status(format!(
                    "mouse: {} (Shift bypasses for selection)",
                    on_off(self.config.mouse)
                ));
            }
            Cmd::ReloadConfig => {
                let (cfg, warning) = Config::load();
                self.config = cfg;
                self.apply_config();
                if let Some(name) = self.themes.iter().position(|t| t.name == self.config.theme) {
                    self.set_theme(name);
                }
                self.set_status(warning.unwrap_or_else(|| "config reloaded".to_string()));
            }
            Cmd::OpenRecent(path) => {
                if let Err(e) = self.open_file(path) {
                    self.set_status(format!("open failed: {e}"));
                }
            }
            Cmd::InsertSnippet(body) => {
                self.buf_mut().insert_str(body);
                self.clear_status();
            }
            Cmd::RunFile => self.start_run(),
            Cmd::StopRun => self.stop_run(),
            Cmd::CloseOutput => self.close_output(),
            Cmd::Find => self.open_search(),
            Cmd::Replace => {
                self.open_search();
                if let Some(s) = &mut self.search {
                    s.field = Field::Replace;
                }
            }
            Cmd::FindInFiles => self.open_project_search(),
            Cmd::GotoLine => self.open_goto_line(),
            Cmd::Projects => self.open_projects(),
            Cmd::Help => self.open_help(),
        }
    }

    // ---- run console ----

    /// F5: run the active buffer through the Vulpin interpreter.
    pub fn start_run(&mut self) {
        let Some(tx) = self.run_tx.clone() else {
            self.set_status("run needs the interactive event loop");
            return;
        };
        let Some(interp) = run::resolve_interpreter(&self.config.vulpin_path) else {
            self.set_status("vulpin interpreter not found — set vulpin_path in config");
            return;
        };
        let (file, workdir, temp) = match self.run_target() {
            Ok(t) => t,
            Err(e) => {
                self.set_status(format!("run: {e}"));
                return;
            }
        };
        let mut argv = interp;
        argv.push(file.to_string_lossy().into_owned());

        // Replacing `self.run` drops the previous console, which kills any child
        // still running and clears its temp file.
        match RunConsole::start(argv, &workdir, temp, &tx) {
            Ok(console) => {
                self.run = Some(console);
                self.focus = Focus::Output;
                self.set_status(format!("running {}", file.display()));
            }
            Err(e) => self.set_status(format!("run failed: {e}")),
        }
    }

    /// The file to hand the interpreter: the buffer's own path when saved and
    /// clean, otherwise a temp `.vul` written from the current contents.
    fn run_target(&self) -> io::Result<(PathBuf, PathBuf, Option<PathBuf>)> {
        let b = self.buf();
        if let Some(path) = b.path()
            && !b.is_dirty()
        {
            let workdir = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            return Ok((path.to_path_buf(), workdir, None));
        }

        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let mut tmp = std::env::temp_dir();
        tmp.push(format!("vulide-run-{}-{nanos}.vul", std::process::id()));
        let mut text = b.rope().to_string();
        if !text.ends_with('\n') {
            text.push('\n');
        }
        std::fs::write(&tmp, text)?;
        let workdir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Ok((tmp.clone(), workdir, Some(tmp)))
    }

    pub fn stop_run(&mut self) {
        match &mut self.run {
            Some(r) if r.is_running() => {
                r.stop();
                self.set_status("run stopped");
            }
            Some(_) => self.set_status("run already finished"),
            None => self.set_status("nothing running"),
        }
    }

    fn close_output(&mut self) {
        if self.run.is_some() {
            self.run = None; // Drop kills any child + removes the temp file
            self.focus = Focus::Editor;
            self.set_status("output panel closed");
        }
    }

    /// `F6`: run the file if nothing has run yet (there's no output panel to
    /// cycle to otherwise); once one exists, cycle focus through every
    /// visible pane — editor, file tree, outline, output — same as `Tab`
    /// already does from a sidebar, but this one works from the editor too,
    /// where bare `Tab` means indent.
    fn toggle_output_focus(&mut self) {
        if self.run.is_none() {
            self.start_run();
            return;
        }
        self.cycle_focus(true);
    }

    /// Panes currently on screen, in a fixed cycling order. The editor is
    /// always available; the others only when their sidebar/panel is shown.
    fn focus_cycle(&self) -> Vec<Focus> {
        let mut v = vec![Focus::Editor];
        if self.show_files {
            v.push(Focus::Files);
        }
        if self.show_algo {
            v.push(Focus::Algo);
        }
        if self.run.is_some() {
            v.push(Focus::Output);
        }
        v
    }

    /// Tab / Shift+Tab, from any pane but the editor (where Tab means indent):
    /// move focus to the next / previous visible pane.
    fn cycle_focus(&mut self, forward: bool) {
        let targets = self.focus_cycle();
        if targets.len() <= 1 {
            return;
        }
        let i = targets.iter().position(|&f| f == self.focus).unwrap_or(0);
        let next = if forward {
            (i + 1) % targets.len()
        } else {
            (i + targets.len() - 1) % targets.len()
        };
        self.focus = targets[next];
    }

    // ---- structure outline ----

    /// `F7` cycles the outline: hidden → shown+focused → (jump leaves it shown
    /// but unfocused) → focused again → hidden.
    fn toggle_algo(&mut self) {
        if !self.show_algo {
            self.show_algo = true;
            self.focus = Focus::Algo;
            self.set_status("outline — ↑↓ select · Enter jump · Esc editor · F7 hide");
        } else if self.focus != Focus::Algo {
            self.focus = Focus::Algo;
        } else {
            self.show_algo = false;
            self.focus = Focus::Editor;
            self.set_status("outline hidden");
        }
        self.config.show_algo = self.show_algo;
        self.save_config();
    }

    /// Keystrokes while the outline has focus: move the selection, Enter jumps
    /// the editor to that line, Esc/F7 leave.
    fn handle_algo_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('q') => {
                    self.request_quit();
                    return;
                }
                KeyCode::Char('w') => {
                    self.close_tab(false);
                    return;
                }
                KeyCode::Char('p') => return self.open_palette(),
                _ => return,
            }
        }
        let n = self.algo_items.len();
        match key.code {
            KeyCode::Esc => self.focus = Focus::Editor,
            KeyCode::Up | KeyCode::Char('k') => {
                self.algo_selected = self.algo_selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.algo_selected + 1 < n {
                    self.algo_selected += 1;
                }
            }
            KeyCode::Home => self.algo_selected = 0,
            KeyCode::End => self.algo_selected = n.saturating_sub(1),
            KeyCode::Enter => self.jump_to_selected_outline_item(),
            KeyCode::Tab => self.cycle_focus(true),
            KeyCode::BackTab => self.cycle_focus(false),
            _ => {}
        }
    }

    fn jump_to_selected_outline_item(&mut self) {
        if let Some(item) = self.algo_items.get(self.algo_selected) {
            let line = item.line;
            self.buf_mut()
                .set_cursor(crate::buffer::Position { line, col: 0 }, false);
        }
        self.focus = Focus::Editor;
    }

    /// The outline row under screen row `row`, if the click landed on one.
    fn algo_row_at(&self, row: u16) -> Option<usize> {
        let r = self.algo_rect?;
        let body_top = r.y + 1; // top border
        let body_bot = r.y + r.height.saturating_sub(1); // bottom border
        if row < body_top || row >= body_bot {
            return None;
        }
        let idx = self.algo_scroll + (row - body_top) as usize;
        (idx < self.algo_items.len()).then_some(idx)
    }

    // ---- file tree ----

    /// Where the tree roots: the active file's directory, else the working dir.
    fn file_tree_root(&self) -> PathBuf {
        self.buf()
            .path()
            .and_then(|p| p.parent())
            .map(Path::to_path_buf)
            .filter(|p| !p.as_os_str().is_empty())
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."))
    }

    /// `F2` cycles the file tree, same three states as the outline: hidden →
    /// shown+focused → shown+unfocused → focused → hidden.
    fn toggle_files(&mut self) {
        if self.file_tree.is_none() {
            let root = self.file_tree_root();
            self.file_tree = Some(crate::filetree::FileTree::new(&root));
            self.files_selected = 0;
        }
        if !self.show_files {
            self.show_files = true;
            self.focus = Focus::Files;
            self.set_status(
                "files — ↑↓ move · → ← expand/collapse · Enter open · r refresh · . hidden · F2 hide",
            );
        } else if self.focus != Focus::Files {
            self.focus = Focus::Files;
        } else {
            self.show_files = false;
            self.focus = Focus::Editor;
            self.set_status("file tree hidden");
        }
        self.config.show_files = self.show_files;
        self.save_config();
    }

    /// Keystrokes while the file tree has focus.
    fn handle_files_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('q') => self.request_quit(),
                KeyCode::Char('w') => self.close_tab(false),
                KeyCode::Char('p') => self.open_palette(),
                _ => {}
            }
            return;
        }
        match key.code {
            KeyCode::Esc => {
                self.focus = Focus::Editor;
                return;
            }
            KeyCode::Enter => {
                self.activate_selected_file();
                return;
            }
            KeyCode::Tab => {
                self.cycle_focus(true);
                return;
            }
            KeyCode::BackTab => {
                self.cycle_focus(false);
                return;
            }
            _ => {}
        }
        let Some(tree) = &mut self.file_tree else {
            return;
        };
        let n = tree.len();
        let sel = &mut self.files_selected;
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => *sel = sel.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => {
                if *sel + 1 < n {
                    *sel += 1;
                }
            }
            KeyCode::Home => *sel = 0,
            KeyCode::End => *sel = n.saturating_sub(1),
            KeyCode::Right | KeyCode::Char('l') => match tree.get(*sel) {
                Some(r) if r.is_dir && !r.expanded => tree.expand(*sel),
                // already-open dir: step into it
                Some(r) if r.is_dir && *sel + 1 < n => *sel += 1,
                _ => {}
            },
            KeyCode::Left | KeyCode::Char('h') => match tree.get(*sel) {
                Some(r) if r.is_dir && r.expanded => tree.collapse(*sel),
                _ => {
                    if let Some(p) = tree.parent_of(*sel) {
                        *sel = p;
                    }
                }
            },
            KeyCode::Char('r') => tree.refresh(),
            KeyCode::Char('.') => tree.toggle_hidden(),
            _ => {}
        }
    }

    /// Enter / click on the selected tree row: open a file, or expand/collapse a
    /// directory (staying in the tree).
    fn activate_selected_file(&mut self) {
        let (is_dir, expanded, path) = match self
            .file_tree
            .as_ref()
            .and_then(|t| t.get(self.files_selected))
        {
            Some(r) => (r.is_dir, r.expanded, r.path.clone()),
            None => return,
        };
        if is_dir {
            if let Some(t) = &mut self.file_tree {
                if expanded {
                    t.collapse(self.files_selected);
                } else {
                    t.expand(self.files_selected);
                }
            }
            return;
        }
        match self.open_file(path) {
            Ok(()) => self.focus = Focus::Editor,
            Err(e) => self.set_status(format!("open failed: {e}")),
        }
    }

    /// The tree row under screen row `row`, if the click landed on one.
    fn files_row_at(&self, row: u16) -> Option<usize> {
        let r = self.files_rect?;
        let body_top = r.y + 1; // top border
        let body_bot = r.y + r.height.saturating_sub(1); // bottom border
        if row < body_top || row >= body_bot {
            return None;
        }
        let idx = self.files_scroll + (row - body_top) as usize;
        let n = self.file_tree.as_ref().map(|t| t.len()).unwrap_or(0);
        (idx < n).then_some(idx)
    }

    /// Label for the status-bar button; `is_running` also decides its colour.
    pub fn run_button_label(&self) -> &'static str {
        if self.run.as_ref().is_some_and(RunConsole::is_running) {
            " ■ Stop "
        } else {
            " ▶ Run "
        }
    }

    /// The ▶/■ button: run when idle, stop when a run is in progress.
    fn toggle_run(&mut self) {
        if self.run.as_ref().is_some_and(RunConsole::is_running) {
            self.stop_run();
        } else {
            self.start_run();
        }
    }

    fn open_help(&mut self) {
        self.overlay = Overlay::Help(Box::default());
    }

    // ---- clipboard ----

    /// The shared clipboard handle, opened once and kept alive (see the
    /// field doc on `App::clipboard` for why: a short-lived one corrupts the
    /// display via a stderr warning while the terminal is in raw mode).
    /// Cheap to call repeatedly — only the first call after a failure retries
    /// `Clipboard::new()`.
    fn clipboard(&mut self) -> Result<&mut arboard::Clipboard, String> {
        if self.clipboard.is_none() {
            self.clipboard = Some(arboard::Clipboard::new().map_err(|e| e.to_string()));
        }
        match self.clipboard.as_mut().unwrap() {
            Ok(cb) => Ok(cb),
            Err(e) => Err(e.clone()),
        }
    }

    /// `Ctrl+C` in the editor: copy the selection to the system clipboard.
    /// Needs a real X11/Wayland/Win32/macOS clipboard to talk to — over a
    /// plain SSH session with no display this fails, which just shows up as
    /// a status message rather than a crash.
    fn copy_selection(&mut self) {
        let Some(text) = self.buf().selection_text().filter(|s| !s.is_empty()) else {
            self.set_status("nothing selected to copy");
            return;
        };
        match self
            .clipboard()
            .and_then(|cb| cb.set_text(text).map_err(|e| e.to_string()))
        {
            Ok(()) => self.set_status("copied"),
            Err(e) => self.set_status(format!("copy failed: {e}")),
        }
    }

    /// `Ctrl+V`: insert the system clipboard's text at the cursor, replacing
    /// the selection if there is one (same as typing would).
    fn paste(&mut self) {
        match self
            .clipboard()
            .and_then(|cb| cb.get_text().map_err(|e| e.to_string()))
        {
            Ok(text) => {
                self.buf_mut().insert_str(&text);
                self.clear_status();
            }
            Err(e) => self.set_status(format!("paste failed: {e}")),
        }
    }

    // ---- find / replace ----

    /// `Ctrl+F`: open the find/replace bar, seeded from the selection.
    fn open_search(&mut self) {
        let seed = self
            .buf()
            .selection_text()
            .filter(|s| !s.is_empty() && !s.contains('\n'))
            .unwrap_or_default();
        self.completion = None;
        self.search_origin = self.buf().cursor();
        self.search = Some(Search::new(&seed));
        self.recompute_matches();
        self.reset_match_to_origin();
        self.focus_current_match();
        self.set_status("find: type to search · Esc closes");
    }

    fn close_search(&mut self) {
        self.search = None;
        self.search_matches.clear();
        self.search_idx = 0;
        self.clear_status();
    }

    /// Refresh the match list for the current query, keeping `search_idx` in
    /// range (callers decide whether to re-anchor it).
    fn recompute_matches(&mut self) {
        let Some(s) = &self.search else {
            self.search_matches.clear();
            return;
        };
        let (q, cs, rx) = (s.query(), s.case_sensitive, s.regex);
        self.search_matches = crate::search::find_all(self.buf(), &q, cs, rx);
        if self.search_idx >= self.search_matches.len() {
            self.search_idx = 0;
        }
    }

    /// Point `search_idx` at the first match at or after where the bar opened.
    fn reset_match_to_origin(&mut self) {
        self.search_idx = self
            .search_matches
            .iter()
            .position(|(start, _)| *start >= self.search_origin)
            .unwrap_or(0);
    }

    /// Select the current match so the editor scrolls it into view.
    fn focus_current_match(&mut self) {
        if let Some(&(start, end)) = self.search_matches.get(self.search_idx) {
            self.buf_mut().set_cursor(start, false);
            self.buf_mut().set_cursor(end, true);
        }
    }

    fn step_match(&mut self, forward: bool) {
        let n = self.search_matches.len();
        if n == 0 {
            self.set_status("no matches");
            return;
        }
        self.search_idx = if forward {
            (self.search_idx + 1) % n
        } else {
            (self.search_idx + n - 1) % n
        };
        self.focus_current_match();
        self.set_status(format!("match {}/{}", self.search_idx + 1, n));
    }

    fn replace_one(&mut self) {
        let Some(&(start, end)) = self.search_matches.get(self.search_idx) else {
            self.set_status("no match to replace");
            return;
        };
        let with = self
            .search
            .as_ref()
            .map(Search::replacement)
            .unwrap_or_default();
        self.buf_mut().replace_ranges(&[(start, end)], &with);
        // The list shifts; keeping search_idx lands us on what was the next match.
        self.recompute_matches();
        self.focus_current_match();
        let n = self.search_matches.len();
        self.set_status(match n {
            0 => "replaced — no matches left".to_string(),
            1 => "replaced — 1 match left".to_string(),
            _ => format!("replaced — {n} matches left"),
        });
    }

    fn replace_all(&mut self) {
        if self.search_matches.is_empty() {
            self.set_status("no matches to replace");
            return;
        }
        let ranges = self.search_matches.clone();
        let with = self
            .search
            .as_ref()
            .map(Search::replacement)
            .unwrap_or_default();
        let count = self.buf_mut().replace_ranges(&ranges, &with);
        self.recompute_matches();
        self.set_status(format!("replaced {count}"));
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        let action = match &mut self.search {
            Some(s) => s.handle_key(key),
            None => return,
        };
        match action {
            SearchAction::Stay => {}
            SearchAction::Requery => {
                self.recompute_matches();
                self.reset_match_to_origin();
                self.focus_current_match();
            }
            SearchAction::Close => self.close_search(),
            SearchAction::Next => self.step_match(true),
            SearchAction::Prev => self.step_match(false),
            SearchAction::ReplaceOne => self.replace_one(),
            SearchAction::ReplaceAll => self.replace_all(),
        }
    }

    /// Push `config.mouse` to the terminal (live toggle). No-op under tests.
    fn apply_mouse_capture(&self) {
        if cfg!(test) {
            return;
        }
        use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture};
        let _ = if self.config.mouse {
            ratatui::crossterm::execute!(std::io::stdout(), EnableMouseCapture)
        } else {
            ratatui::crossterm::execute!(std::io::stdout(), DisableMouseCapture)
        };
    }

    /// Returns whether the screen needs a redraw (so idle mouse motion is free).
    fn handle_mouse(&mut self, ev: MouseEvent) -> bool {
        let (col, row) = (ev.column, ev.row);
        match ev.kind {
            MouseEventKind::Moved => return self.update_hover(col, row),
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.dragging_splitter {
                    self.resize_panel_to(row);
                    return true;
                }
                return false;
            }
            MouseEventKind::Up(MouseButton::Left) => {
                let was = self.dragging_splitter;
                self.dragging_splitter = false;
                return was;
            }
            MouseEventKind::ScrollUp if self.focus == Focus::Output => {
                if let Some(r) = &mut self.run {
                    r.scroll_up(3);
                }
                return true;
            }
            MouseEventKind::ScrollDown if self.focus == Focus::Output => {
                if let Some(r) = &mut self.run {
                    r.scroll_down(3);
                }
                return true;
            }
            MouseEventKind::ScrollUp
                if self.algo_rect.is_some_and(|r| hit(r, col, row))
                    && !self.algo_items.is_empty() =>
            {
                self.algo_selected = self.algo_selected.saturating_sub(1);
                return true;
            }
            MouseEventKind::ScrollDown
                if self.algo_rect.is_some_and(|r| hit(r, col, row))
                    && !self.algo_items.is_empty() =>
            {
                if self.algo_selected + 1 < self.algo_items.len() {
                    self.algo_selected += 1;
                }
                return true;
            }
            MouseEventKind::ScrollUp
                if self.files_rect.is_some_and(|r| hit(r, col, row))
                    && self.file_tree.as_ref().is_some_and(|t| !t.is_empty()) =>
            {
                self.files_selected = self.files_selected.saturating_sub(1);
                return true;
            }
            MouseEventKind::ScrollDown if self.files_rect.is_some_and(|r| hit(r, col, row)) => {
                let n = self.file_tree.as_ref().map(|t| t.len()).unwrap_or(0);
                if self.files_selected + 1 < n {
                    self.files_selected += 1;
                }
                return true;
            }
            MouseEventKind::ScrollUp if self.overlay.is_open() => {
                self.scroll_overlay(-1);
                return true;
            }
            MouseEventKind::ScrollDown if self.overlay.is_open() => {
                self.scroll_overlay(1);
                return true;
            }
            MouseEventKind::Down(MouseButton::Left) => {}
            _ => return false,
        }

        // ---- left click ----

        // A click outside an open overlay dismisses it (like Esc); a click
        // inside one selects/activates whatever row it landed on.
        if self.overlay.is_open() {
            let Some(rect) = self.overlay_rect else {
                self.dismiss_overlay();
                return true;
            };
            if !hit(rect, col, row) {
                self.dismiss_overlay();
                return true;
            }
            match &mut self.overlay {
                Overlay::Palette(p) => match p.click(rect, row) {
                    PaletteOutcome::Run(cmd) => {
                        self.overlay = Overlay::None;
                        self.run_command(cmd);
                    }
                    PaletteOutcome::Cancel => self.overlay = Overlay::None,
                    PaletteOutcome::Stay => {}
                },
                Overlay::SnippetPicker(p) => match p.click(rect, row) {
                    SnippetOutcome::Insert(body) => {
                        self.overlay = Overlay::None;
                        self.buf_mut().insert_str(body);
                    }
                    SnippetOutcome::Cancel => self.overlay = Overlay::None,
                    SnippetOutcome::Stay => {}
                },
                Overlay::ProjectSearch(ps) => match ps.click(rect, col, row) {
                    ProjectSearchAction::Open(path, line, col) => {
                        self.overlay = Overlay::None;
                        self.jump_to_match(path, line, col);
                    }
                    ProjectSearchAction::Cancel => self.overlay = Overlay::None,
                    ProjectSearchAction::Requery => self.recompute_project_matches(),
                    ProjectSearchAction::Stay => {}
                },
                Overlay::Projects(p) => match p.click(rect, row) {
                    ProjectsAction::NewProject => {
                        self.overlay = Overlay::Prompt(Box::new(PathPrompt::new_project(
                            &default_save_seed(),
                        )));
                    }
                    ProjectsAction::OpenProjectPrompt => {
                        self.overlay =
                            Overlay::Prompt(Box::new(PathPrompt::open(&default_save_seed())));
                    }
                    ProjectsAction::OpenRecent(path) => {
                        self.overlay = Overlay::None;
                        self.open_dir(path);
                    }
                    ProjectsAction::DeleteRecent(path) => self.request_delete_project(path),
                    ProjectsAction::Cancel | ProjectsAction::Stay => {}
                },
                _ => {}
            }
            return true;
        }

        // The find bar's [Aa] / [.*] toggle buttons.
        if let Some(s) = &mut self.search
            && s.click(col, row)
        {
            self.recompute_matches();
            self.reset_match_to_origin();
            self.focus_current_match();
            return true;
        }

        // Grab the splitter.
        if self.splitter_rect.is_some_and(|r| hit(r, col, row)) {
            self.dragging_splitter = true;
            return true;
        }

        // The panel's ✕ (checked before the panel body so the corner works).
        if self.panel_close_rect.is_some_and(|r| hit(r, col, row)) {
            self.close_output();
            return true;
        }

        // The status-bar ▶/■ button.
        if self.run_button.is_some_and(|r| hit(r, col, row)) {
            self.toggle_run();
            return true;
        }

        // Tabs: a click on a tab switches to it; on its ✕ closes it.
        let tab_action = self.tab_hits.iter().find_map(|t| {
            if hit(t.close, col, row) {
                Some((t.index, true))
            } else if hit(t.rect, col, row) {
                Some((t.index, false))
            } else {
                None
            }
        });
        if let Some((index, close)) = tab_action {
            if close {
                self.close_tab_at(index, false);
            } else {
                self.active = index;
                self.completion = None;
                self.dismiss_search_on_switch();
            }
            self.focus = Focus::Editor;
            return true;
        }

        // File tree: a click on a row opens the file / toggles the folder;
        // elsewhere in the panel just focuses it.
        if self.files_rect.is_some_and(|r| hit(r, col, row)) {
            match self.files_row_at(row) {
                Some(i) => {
                    self.files_selected = i;
                    self.activate_selected_file();
                }
                None => self.focus = Focus::Files,
            }
            return true;
        }

        // Outline sidebar: a click on a row jumps the editor there; elsewhere
        // in the panel just focuses it.
        if self.algo_rect.is_some_and(|r| hit(r, col, row)) {
            match self.algo_row_at(row) {
                Some(i) => {
                    self.algo_selected = i;
                    self.jump_to_selected_outline_item();
                }
                None => self.focus = Focus::Algo,
            }
            return true;
        }

        // Otherwise a click just moves focus between panes, so the keyboard
        // always goes where you're looking.
        if self.panel_rect.is_some_and(|r| hit(r, col, row)) && self.run.is_some() {
            self.focus = Focus::Output;
        } else if self.editor_rect.height > 0 && hit(self.editor_rect, col, row) {
            self.focus = Focus::Editor;
        }
        true
    }

    /// Recompute hover flags; returns whether any of them changed.
    fn update_hover(&mut self, col: u16, row: u16) -> bool {
        let tab = self
            .tab_hits
            .iter()
            .find(|t| hit(t.rect, col, row))
            .map(|t| t.index);
        let splitter = self.splitter_rect.is_some_and(|r| hit(r, col, row));
        let close = self.panel_close_rect.is_some_and(|r| hit(r, col, row));
        let changed = tab != self.hovered_tab
            || splitter != self.hover_splitter
            || close != self.hover_panel_close;
        self.hovered_tab = tab;
        self.hover_splitter = splitter;
        self.hover_panel_close = close;
        changed
    }

    /// Drag the editor/panel divider: the splitter row follows the cursor,
    /// keeping the editor at least [`ui::MIN_EDITOR_ROWS`] tall.
    fn resize_panel_to(&mut self, row: u16) {
        // The panel's bottom edge is whatever sits directly below it — the
        // search bar if it's open, otherwise the status bar.
        let bottom = self.search_rect.map(|r| r.y).unwrap_or(self.status_rect.y);
        let editor_top = self.editor_rect.y;
        if bottom == 0 {
            return;
        }
        // panel occupies (row+1 ..= bottom-1)  →  height = bottom - row - 1
        let want = bottom.saturating_sub(row).saturating_sub(1);
        let max = bottom.saturating_sub(editor_top + crate::ui::MIN_EDITOR_ROWS + 1);
        self.panel_height = Some(want.clamp(3, max.max(3)));
    }

    /// `F11` / `F12`: grow / shrink the output panel a few rows at a time —
    /// dragging the splitter is otherwise the *only* way to resize it, with
    /// no keyboard path at all (and mouse support can be turned off entirely
    /// via `Cmd::ToggleMouse`, so this isn't just a no-mouse-hardware case).
    fn resize_panel_by(&mut self, rows: i32) {
        let Some(splitter) = self.splitter_rect else {
            self.set_status("no output panel to resize");
            return;
        };
        let row = (i32::from(splitter.y) - rows).max(0) as u16;
        self.resize_panel_to(row);
    }

    /// Close the overlay the way its own Esc would (reverting a theme preview).
    fn dismiss_overlay(&mut self) {
        if let Overlay::ThemePicker(p) = &self.overlay {
            let original = p.original;
            self.preview_theme(original);
        }
        self.overlay = Overlay::None;
    }

    /// Mouse wheel while an overlay with a scrollable list is open.
    fn scroll_overlay(&mut self, delta: isize) {
        match &mut self.overlay {
            Overlay::Palette(p) => p.scroll(delta),
            Overlay::SnippetPicker(p) => p.scroll(delta),
            Overlay::ProjectSearch(p) => p.scroll(delta),
            Overlay::Projects(p) => p.scroll(delta),
            Overlay::Help(h) => {
                h.scroll = if delta < 0 {
                    h.scroll.saturating_sub(1)
                } else {
                    h.scroll + 1
                };
            }
            _ => {}
        }
    }

    #[cfg(test)]
    pub fn inject_run_tx(&mut self, tx: Sender<AppEvent>) {
        self.run_tx = Some(tx);
    }

    /// Test hook: start a run from an explicit argv, skipping interpreter
    /// resolution and temp-file handling (workdir = cwd).
    #[cfg(test)]
    pub fn start_run_argv(&mut self, argv: Vec<String>) {
        let tx = self.run_tx.clone().expect("inject_run_tx first");
        let workdir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        match RunConsole::start(argv, &workdir, None, &tx) {
            Ok(console) => {
                self.run = Some(console);
                self.focus = Focus::Output;
            }
            Err(e) => self.set_status(format!("run failed: {e}")),
        }
    }

    // ---- misc ----

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status = msg.into();
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// The terminal window/tab title: `VulIDE — <file>` (`•` when unsaved).
    pub(crate) fn window_title(&self) -> String {
        let b = self.buf();
        let name = b
            .path()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".to_string());
        let mark = if b.is_dirty() { " •" } else { "" };
        format!("VulIDE — {name}{mark}")
    }

    /// Push the current title to the terminal, but only when it changed.
    fn sync_window_title(&mut self) {
        if cfg!(test) {
            return;
        }
        let want = self.window_title();
        if want != self.title_shown {
            let _ = ratatui::crossterm::execute!(
                io::stdout(),
                ratatui::crossterm::terminal::SetTitle(&want)
            );
            self.title_shown = want;
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let events = EventSource::new(TICK);
        self.run_tx = Some(events.sender());
        self.sync_window_title();
        terminal.draw(|f| ui::draw(f, self))?;

        while !self.should_quit {
            let Some(first) = events.next()? else {
                break; // every sender dropped — shouldn't happen, but exit cleanly
            };
            let mut dirty = self.handle_event(first);
            // Coalesce a burst (e.g. a flood of output lines, or mouse motion)
            // into a single redraw — and skip it entirely if nothing changed.
            while let Some(ev) = events.try_next() {
                if self.should_quit {
                    break;
                }
                dirty |= self.handle_event(ev);
            }
            if dirty {
                self.sync_window_title();
                terminal.draw(|f| ui::draw(f, self))?;
            }
        }
        self.run_tx = None;
        self.persist_session();
        Ok(())
    }

    /// Dispatch one event; returns whether the screen needs to be redrawn.
    pub fn handle_event(&mut self, ev: AppEvent) -> bool {
        match ev {
            AppEvent::Key(key) => self.handle_key(key),
            AppEvent::Mouse(m) => return self.handle_mouse(m),
            AppEvent::Paste(text) if self.focus == Focus::Output => {
                if let Some(r) = &mut self.run {
                    r.input.insert_str(&text);
                }
            }
            AppEvent::Paste(text) => {
                self.buf_mut().insert_str(&text);
                self.clear_status();
            }
            AppEvent::Output { stream, line } => {
                if let Some(r) = &mut self.run {
                    r.on_output(stream, line);
                }
            }
            AppEvent::StreamClosed(_) => {
                let done = if let Some(r) = &mut self.run {
                    if r.on_stream_closed() {
                        r.reap();
                        r.close_stdin();
                        Some((r.stopped, r.exit_code))
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some((stopped, code)) = done {
                    self.set_status(match (stopped, code) {
                        (true, _) => "run stopped".to_string(),
                        (_, Some(0)) => "run finished (exit 0)".to_string(),
                        (_, Some(c)) => format!("run finished (exit {c})"),
                        (_, None) => "run finished".to_string(),
                    });
                    // The program is done — hand the keyboard back to the editor
                    // unless the user is scrolled up reading the output.
                    if self.focus == Focus::Output
                        && self.run.as_ref().is_some_and(|r| r.scroll == 0)
                    {
                        self.focus = Focus::Editor;
                    }
                }
            }
            AppEvent::InputClosed => {
                self.set_status("terminal input closed");
                self.should_quit = true;
            }
            AppEvent::Resize(..) => {}
            AppEvent::Tick => return false,
        }
        true
    }

    fn clear_status(&mut self) {
        self.status.clear();
    }

    /// Route a key to whatever overlay is open. Returns `true` if the overlay
    /// consumed the key (the editor must not also see it).
    fn handle_overlay_key(&mut self, key: KeyEvent) -> bool {
        match &mut self.overlay {
            Overlay::None => false,
            Overlay::Prompt(prompt) => {
                let outcome = prompt.handle_key(key);
                self.resolve_prompt(outcome);
                true
            }
            Overlay::Palette(palette) => {
                match palette.handle_key(key) {
                    PaletteOutcome::Stay => {}
                    PaletteOutcome::Cancel => self.overlay = Overlay::None,
                    PaletteOutcome::Run(cmd) => {
                        self.overlay = Overlay::None;
                        self.run_command(cmd);
                    }
                }
                true
            }
            Overlay::Help(h) => {
                if let HelpOutcome::Close = h.handle_key(key) {
                    self.overlay = Overlay::None;
                }
                true
            }
            Overlay::Confirm(c) => {
                let action = c.action;
                match c.handle_key(key) {
                    ConfirmOutcome::Stay => {}
                    ConfirmOutcome::Cancel => self.overlay = Overlay::None,
                    ConfirmOutcome::Yes => {
                        self.overlay = Overlay::None;
                        match action {
                            ConfirmAction::QuitUnsaved => self.confirm_yes_quit(),
                        }
                    }
                    ConfirmOutcome::No => {
                        self.overlay = Overlay::None;
                        match action {
                            ConfirmAction::QuitUnsaved => self.should_quit = true,
                        }
                    }
                }
                true
            }
            Overlay::SnippetPicker(p) => {
                match p.handle_key(key) {
                    SnippetOutcome::Stay => {}
                    SnippetOutcome::Cancel => self.overlay = Overlay::None,
                    SnippetOutcome::Insert(body) => {
                        self.overlay = Overlay::None;
                        self.buf_mut().insert_str(body);
                    }
                }
                true
            }
            Overlay::GotoLine(g) => {
                match g.handle_key(key) {
                    GotoOutcome::Stay => {}
                    GotoOutcome::Cancel => self.overlay = Overlay::None,
                    GotoOutcome::Submit(n) => {
                        self.overlay = Overlay::None;
                        self.goto_line(n);
                    }
                }
                true
            }
            Overlay::ProjectSearch(ps) => {
                match ps.handle_key(key) {
                    ProjectSearchAction::Stay => {}
                    ProjectSearchAction::Requery => self.recompute_project_matches(),
                    ProjectSearchAction::Cancel => self.overlay = Overlay::None,
                    ProjectSearchAction::Open(path, line, col) => {
                        self.overlay = Overlay::None;
                        self.jump_to_match(path, line, col);
                    }
                }
                true
            }
            Overlay::Projects(p) => {
                match p.handle_key(key) {
                    ProjectsAction::Stay => {}
                    ProjectsAction::Cancel => self.overlay = Overlay::None,
                    ProjectsAction::NewProject => {
                        self.overlay = Overlay::Prompt(Box::new(PathPrompt::new_project(
                            &default_save_seed(),
                        )));
                    }
                    ProjectsAction::OpenProjectPrompt => {
                        self.overlay =
                            Overlay::Prompt(Box::new(PathPrompt::open(&default_save_seed())));
                    }
                    ProjectsAction::OpenRecent(path) => {
                        self.overlay = Overlay::None;
                        self.open_dir(path);
                    }
                    ProjectsAction::DeleteRecent(path) => self.request_delete_project(path),
                }
                true
            }
            Overlay::DeleteProject(d) => {
                match d.handle_key(key) {
                    DeleteOutcome::Stay => {}
                    DeleteOutcome::Cancel => self.overlay = Overlay::None,
                    DeleteOutcome::Confirmed(path) => {
                        self.overlay = Overlay::None;
                        self.delete_project(path);
                    }
                }
                true
            }
            Overlay::ThemePicker(picker) => {
                match picker.handle_key(key) {
                    ThemePickerOutcome::Preview(i) => self.preview_theme(i),
                    ThemePickerOutcome::Commit(i) => {
                        self.set_theme(i);
                        self.overlay = Overlay::None;
                        self.set_status(format!("theme: {}", self.theme.name));
                    }
                    ThemePickerOutcome::Cancel => {
                        let original = match &self.overlay {
                            Overlay::ThemePicker(p) => p.original,
                            _ => self.theme_idx,
                        };
                        self.preview_theme(original);
                        self.overlay = Overlay::None;
                    }
                }
                true
            }
        }
    }

    fn resolve_prompt(&mut self, outcome: PromptOutcome) {
        let kind = match &self.overlay {
            Overlay::Prompt(p) => p.kind,
            _ => return,
        };
        match outcome {
            PromptOutcome::Stay => {}
            PromptOutcome::Cancel => {
                self.overlay = Overlay::None;
                self.set_status(match kind {
                    PromptKind::Save => "save cancelled",
                    PromptKind::Open => "open cancelled",
                    PromptKind::NewProject => "new project cancelled",
                });
            }
            PromptOutcome::Submit(path) if path.is_empty() => {
                if let Overlay::Prompt(prompt) = &mut self.overlay {
                    prompt.error = Some("enter a path".to_string());
                }
            }
            PromptOutcome::Submit(path) => {
                let path = expand_tilde(&path);
                let result: io::Result<String> = match kind {
                    PromptKind::Save => {
                        let r = self.buf_mut().save_as(&path);
                        r.map(|()| format!("saved {}", self.buf().title()))
                    }
                    // `open_file` sets its own status; keep it on success.
                    PromptKind::Open => {
                        self.open_file(PathBuf::from(&path)).map(|()| String::new())
                    }
                    // `create_project` (via `open_dir`) sets its own status too.
                    PromptKind::NewProject => self.create_project(&path).map(|()| String::new()),
                };
                match result {
                    Ok(msg) => {
                        self.overlay = Overlay::None;
                        if !msg.is_empty() {
                            self.set_status(msg);
                        }
                    }
                    Err(e) => {
                        if let Overlay::Prompt(prompt) = &mut self.overlay {
                            prompt.error = Some(e.to_string());
                        }
                    }
                }
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.handle_overlay_key(key) {
            return;
        }
        // These work from either pane.
        match key.code {
            KeyCode::F(5) => return self.start_run(),
            KeyCode::F(6) => return self.toggle_output_focus(),
            KeyCode::F(7) => return self.toggle_algo(),
            KeyCode::F(2) => return self.toggle_files(),
            KeyCode::F(3) => return self.open_snippet_picker(),
            KeyCode::F(4) => return self.open_project_search(),
            KeyCode::F(8) => return self.open_projects(),
            KeyCode::F(9) => return self.close_output(),
            KeyCode::F(11) => return self.resize_panel_by(3),
            KeyCode::F(12) => return self.resize_panel_by(-3),
            KeyCode::F(1) => return self.open_help(),
            _ => {}
        }
        // File/view management shortcuts also work from any pane — previously
        // these only fired from `handle_key_inner`, so e.g. Ctrl+O did nothing
        // at all while focus was on the file tree or output panel. (Ctrl+C and
        // Ctrl+W are deliberately NOT here: Ctrl+C copies the editor's
        // selection in `handle_key_inner` but stops a running child in the
        // output panel, and Ctrl+W closes the active tab everywhere except
        // the output panel, where it closes *that* instead — a global
        // binding would pick the wrong one in both cases.)
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('o') => {
                    self.overlay =
                        Overlay::Prompt(Box::new(PathPrompt::open(&default_save_seed())));
                    return;
                }
                KeyCode::Char('n') => return self.new_tab(),
                KeyCode::Char('s') => return self.save_active(),
                KeyCode::Char('t') => return self.open_theme_picker(),
                KeyCode::Char('f') => return self.open_search(),
                KeyCode::Char('g') => return self.open_goto_line(),
                _ => {}
            }
        }
        if self.focus == Focus::Output {
            self.handle_output_key(key);
            return;
        }
        if self.focus == Focus::Algo {
            self.handle_algo_key(key);
            return;
        }
        if self.focus == Focus::Files {
            self.handle_files_key(key);
            return;
        }
        if self.search.is_some() {
            self.handle_search_key(key);
            return;
        }
        if self.completion.is_some() && self.handle_completion_key(key) {
            return;
        }
        self.handle_key_inner(key);
        // The `$word` context under the cursor may have changed — re-scan.
        // Autocomplete is Vulpin-specific ($vars, string methods, command
        // hints), so it stays off for Python / Rust / C / … buffers.
        self.completion = if self.config.show_autocomplete
            && self.buf().language() == crate::syntax::Language::Vulpin
        {
            Completion::detect(self.buf())
        } else {
            None
        };
    }

    /// Keystrokes while the output panel has focus: scrollback nav, a stdin line,
    /// Esc back to the editor. `Ctrl+C` stops a running child, same as a real
    /// terminal's SIGINT — it no longer quits the app (that's `Ctrl+Q` only).
    /// `Ctrl+W` closes the panel itself here, rather than the active file tab
    /// like it does everywhere else — closing "this" makes more sense than
    /// reaching past it to a tab you can't currently see.
    fn handle_output_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        if ctrl {
            match key.code {
                KeyCode::Char('q') => {
                    self.request_quit();
                    return;
                }
                KeyCode::Char('c') => {
                    if self.run.as_ref().is_some_and(RunConsole::is_running) {
                        self.stop_run();
                    }
                    return;
                }
                KeyCode::Char('w') => {
                    self.close_output();
                    return;
                }
                KeyCode::Char('p') => return self.open_palette(),
                KeyCode::Char('d') => {
                    if let Some(r) = &mut self.run {
                        r.close_stdin();
                        self.set_status("stdin closed (EOF)");
                    }
                    return;
                }
                _ => {}
            }
        }
        let Some(r) = &mut self.run else {
            self.focus = Focus::Editor;
            return;
        };
        let running = r.is_running();
        match key.code {
            KeyCode::Esc => self.focus = Focus::Editor,
            KeyCode::Tab => self.cycle_focus(true),
            KeyCode::BackTab => self.cycle_focus(false),
            KeyCode::Up => r.scroll_up(1),
            KeyCode::Down => r.scroll_down(1),
            KeyCode::PageUp => r.scroll_up(10),
            KeyCode::PageDown => r.scroll_down(10),
            KeyCode::Home => r.scroll_up(usize::MAX),
            KeyCode::End => r.scroll_to_bottom(),
            KeyCode::Enter if running => {
                let line = r.input.rope().to_string();
                r.input = Buffer::new();
                r.send_stdin(&line);
                r.scroll_to_bottom();
            }
            KeyCode::Backspace if running => r.input.delete_backward(),
            KeyCode::Char(c) if running && !ctrl && !alt => r.input.insert_char(c),
            _ => {}
        }
    }

    /// Route a key to the autocomplete popup. Returns `true` if it was consumed
    /// (navigation / accept / dismiss); `false` lets the key edit as normal and
    /// the popup refreshes afterwards.
    fn handle_completion_key(&mut self, key: KeyEvent) -> bool {
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return false;
        }
        let Some(c) = &mut self.completion else {
            return false;
        };
        match key.code {
            KeyCode::Up => c.move_up(),
            KeyCode::Down => c.move_down(),
            KeyCode::Esc => self.completion = None,
            // Enter and Tab both accept. When there's nothing left to insert
            // (a command reminder, or a fully-typed method) Enter falls through
            // so it still makes a newline.
            KeyCode::Enter | KeyCode::Tab => {
                let tail = c.completion_tail().to_string();
                self.completion = None;
                if tail.is_empty() {
                    return key.code != KeyCode::Enter;
                }
                self.buf_mut().insert_str(&tail);
            }
            _ => return false,
        }
        true
    }

    fn handle_key_inner(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let rows = self.editor_rows.max(1);

        // ---- app-level shortcuts (must not hold a &mut buffer) ----
        if ctrl {
            match key.code {
                // Ctrl+Q quits (asks first if a buffer is unsaved). A modal
                // `:` command line (BatScript wants Vim-like) is still Phase
                // 1.5. Ctrl+C is copy — the near-universal convention — not
                // a second quit key.
                KeyCode::Char('q') => {
                    self.request_quit();
                    return;
                }
                KeyCode::Char('c') => {
                    self.copy_selection();
                    return;
                }
                KeyCode::Char('v') => {
                    self.paste();
                    return;
                }
                KeyCode::Char('d') => {
                    self.buf_mut().duplicate_line();
                    return;
                }
                KeyCode::Char('p') => {
                    self.open_palette();
                    return;
                }
                // Many terminals send Ctrl+H as Backspace; where it arrives as a
                // real Ctrl+H it opens the help card (F1 is the reliable key).
                KeyCode::Char('h') => {
                    self.open_help();
                    return;
                }
                KeyCode::Char('f') => {
                    self.open_search();
                    return;
                }
                KeyCode::Char('o') => {
                    self.overlay =
                        Overlay::Prompt(Box::new(PathPrompt::open(&default_save_seed())));
                    return;
                }
                KeyCode::Char('n') => {
                    self.new_tab();
                    return;
                }
                KeyCode::Char('w') => {
                    self.close_tab(false);
                    return;
                }
                // Ctrl+Tab / Ctrl+Shift+Tab only — dropped the Ctrl+PgUp/PgDn
                // aliases, which just duplicated this with no real upside.
                KeyCode::Tab => {
                    self.next_tab();
                    return;
                }
                KeyCode::BackTab => {
                    self.prev_tab();
                    return;
                }
                KeyCode::Char('t') => {
                    self.open_theme_picker();
                    return;
                }
                KeyCode::Char('s') => {
                    self.save_active();
                    return;
                }
                KeyCode::Char('z') if !shift => {
                    let ok = self.buf_mut().undo();
                    self.set_status(if ok { "" } else { "nothing to undo" });
                    return;
                }
                KeyCode::Char('z') if shift => {
                    self.buf_mut().redo();
                    self.clear_status();
                    return;
                }
                KeyCode::Char('y') => {
                    let ok = self.buf_mut().redo();
                    self.set_status(if ok { "" } else { "nothing to redo" });
                    return;
                }
                KeyCode::Char('a') => {
                    self.buf_mut().select_all();
                    return;
                }
                _ => {}
            }
        }

        let b = &mut self.buffers[self.active];
        match key.code {
            // ---- motion ----
            KeyCode::Left if ctrl => b.move_word_left(shift),
            KeyCode::Right if ctrl => b.move_word_right(shift),
            KeyCode::Left => b.move_left(shift),
            KeyCode::Right => b.move_right(shift),
            KeyCode::Up if alt => b.move_line_up(),
            KeyCode::Down if alt => b.move_line_down(),
            KeyCode::Up => b.move_up(shift),
            KeyCode::Down => b.move_down(shift),
            KeyCode::Home if ctrl => b.move_doc_start(shift),
            KeyCode::End if ctrl => b.move_doc_end(shift),
            KeyCode::Home => b.move_home(shift),
            KeyCode::End => b.move_end(shift),
            KeyCode::PageUp => b.move_page_up(rows, shift),
            KeyCode::PageDown => b.move_page_down(rows, shift),
            KeyCode::Esc => {
                let c = b.cursor();
                b.set_cursor(c, false);
            }

            // ---- edits ----
            KeyCode::Char(c) if !ctrl && !alt => b.insert_char(c),
            KeyCode::Enter => b.insert_char('\n'),
            KeyCode::Backspace if ctrl => b.delete_word_backward(),
            KeyCode::Delete if ctrl => b.delete_word_forward(),
            KeyCode::Backspace => b.delete_backward(),
            KeyCode::Delete => b.delete_forward(),
            KeyCode::Tab => {
                if b.selection().is_some() {
                    b.indent(false);
                } else {
                    let pad: String = " ".repeat(b.tab_width);
                    b.insert_str(&pad);
                }
            }
            KeyCode::BackTab => b.indent(true),

            _ => {}
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience for tests / callers wanting a buffer preloaded from disk.
pub fn app_with_file(path: &Path) -> Result<App> {
    let mut app = App::new();
    app.open_path(path.to_path_buf())?;
    Ok(app)
}

fn on_off(b: bool) -> &'static str {
    if b { "on" } else { "off" }
}

/// Is the cell `(col, row)` inside `r`?
fn hit(r: Rect, col: u16, row: u16) -> bool {
    col >= r.x && col < r.x + r.width && row >= r.y && row < r.y + r.height
}

/// Prefill for the path field: the current working directory with a trailing
/// separator, so the user only types a filename.
fn default_save_seed() -> String {
    match std::env::current_dir() {
        Ok(dir) => format!("{}/", dir.display()),
        Err(_) => "~/".to_string(),
    }
}
