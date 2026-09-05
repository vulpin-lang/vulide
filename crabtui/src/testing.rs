//! `Harness` — drive the real `App` headlessly against a `ratatui` `TestBackend`.
//!
//! Tests feed parsed events through `App::handle_event` (the same path the live
//! loop uses) and read back the rendered screen. Keystroke tests touch no real
//! terminal and never sleep. The run-console tests do spawn real short-lived
//! child processes (`printf`, `cat`, …) and `pump` their output off the channel.

#![cfg(test)]

use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;
use crate::buffer::Buffer;
use crate::config::Config;
use crate::event::AppEvent;

pub struct Harness {
    pub app: App,
    terminal: Terminal<TestBackend>,
    run_rx: Receiver<AppEvent>,
}

impl Harness {
    pub fn new(width: u16, height: u16) -> Self {
        let terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let (tx, run_rx) = mpsc::channel();
        let mut app = App::with_config(Config::default());
        app.inject_run_tx(tx);
        let mut h = Self {
            app,
            terminal,
            run_rx,
        };
        h.draw();
        h
    }

    /// Drain any background (run-console) events, then redraw.
    pub fn pump(&mut self) -> &mut Self {
        while let Ok(ev) = self.run_rx.try_recv() {
            self.app.handle_event(ev);
        }
        self.draw()
    }

    /// Pump until `pred` holds or `timeout` elapses. Returns whether it held.
    pub fn pump_until(&mut self, timeout: Duration, mut pred: impl FnMut(&App) -> bool) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            self.pump();
            if pred(&self.app) {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    pub fn with_text(text: &str, width: u16, height: u16) -> Self {
        let mut h = Self::new(width, height);
        h.app.buffers[0] = Buffer::from_str(text);
        h.draw();
        h
    }

    pub fn draw(&mut self) -> &mut Self {
        let app = &mut self.app;
        self.terminal.draw(|f| crate::ui::draw(f, app)).unwrap();
        self
    }

    pub fn key(&mut self, code: KeyCode) -> &mut Self {
        self.key_mods(code, KeyModifiers::NONE)
    }

    pub fn key_mods(&mut self, code: KeyCode, mods: KeyModifiers) -> &mut Self {
        self.app
            .handle_event(AppEvent::Key(KeyEvent::new(code, mods)));
        self.draw()
    }

    pub fn ctrl(&mut self, ch: char) -> &mut Self {
        self.key_mods(KeyCode::Char(ch), KeyModifiers::CONTROL)
    }

    fn mouse(
        &mut self,
        kind: ratatui::crossterm::event::MouseEventKind,
        col: u16,
        row: u16,
    ) -> &mut Self {
        use ratatui::crossterm::event::MouseEvent;
        self.app.handle_event(AppEvent::Mouse(MouseEvent {
            kind,
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        }));
        self.draw()
    }

    /// A left-button press at `(col, row)`.
    pub fn click(&mut self, col: u16, row: u16) -> &mut Self {
        use ratatui::crossterm::event::{MouseButton, MouseEventKind};
        self.mouse(MouseEventKind::Down(MouseButton::Left), col, row)
    }

    pub fn mouse_move(&mut self, col: u16, row: u16) -> &mut Self {
        self.mouse(ratatui::crossterm::event::MouseEventKind::Moved, col, row)
    }

    /// A full press → drag → release from `(x0,y0)` to `(x1,y1)`.
    pub fn drag(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> &mut Self {
        use ratatui::crossterm::event::{MouseButton, MouseEventKind};
        self.mouse(MouseEventKind::Down(MouseButton::Left), x0, y0);
        self.mouse(MouseEventKind::Drag(MouseButton::Left), x1, y1);
        self.mouse(MouseEventKind::Up(MouseButton::Left), x1, y1)
    }

    pub fn type_str(&mut self, s: &str) -> &mut Self {
        for ch in s.chars() {
            let code = if ch == '\n' {
                KeyCode::Enter
            } else {
                KeyCode::Char(ch)
            };
            self.app
                .handle_event(AppEvent::Key(KeyEvent::new(code, KeyModifiers::NONE)));
        }
        self.draw()
    }

    pub fn screen(&self) -> String {
        let buf = self.terminal.backend().buffer();
        let area = *buf.area();
        let mut out = String::new();
        for y in 0..area.height {
            for x in 0..area.width {
                if let Some(cell) = buf.cell((x, y)) {
                    out.push_str(cell.symbol());
                }
            }
            out.push('\n');
        }
        out
    }

    pub fn line(&self, row: u16) -> String {
        self.screen()
            .lines()
            .nth(row as usize)
            .unwrap_or_default()
            .trim_end()
            .to_string()
    }

    pub fn contains(&self, needle: &str) -> bool {
        self.screen().contains(needle)
    }

    /// The terminal cursor position `(x, y)` after the last draw.
    pub fn cursor_xy(&self) -> (u16, u16) {
        let p = self.terminal.backend().cursor_position();
        (p.x, p.y)
    }

    /// The rendered cell at `(x, y)` — for asserting on colour/style.
    pub fn cell(&self, x: u16, y: u16) -> ratatui::buffer::Cell {
        self.terminal
            .backend()
            .buffer()
            .cell((x, y))
            .cloned()
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::buffer::Position;

    #[test]
    fn status_bar_renders() {
        let h = Harness::new(40, 6);
        assert!(h.contains("untitled"));
        assert!(h.contains("Ln 1, Col 1"));
    }

    #[test]
    fn typing_shows_on_screen() {
        let mut h = Harness::new(40, 6);
        h.type_str("G\"hello\"");
        assert!(h.contains("G\"hello\""));
        assert!(h.contains("Ln 1, Col 9"));
    }

    #[test]
    fn ctrl_t_theme_picker_previews_commits_and_reverts() {
        use ratatui::style::Color;

        let mut h = Harness::with_text("G\"hi\"", 44, 12);
        let g_mocha = h.cell(4, 0).fg; // the 'G', coloured by theme.command
        assert_eq!(h.app.theme.name, "Dark (Catppuccin Mocha)");

        h.ctrl('t');
        assert!(h.contains("Theme"));
        assert_eq!(h.app.theme.name, "Dark (Catppuccin Mocha)"); // not changed yet

        // Down previews the next theme live on the editor behind the overlay.
        h.key(KeyCode::Down);
        assert_eq!(h.app.theme.name, "Light (Catppuccin Latte)");
        assert_ne!(h.cell(4, 0).fg, g_mocha, "'G' recoloured on preview");
        assert_ne!(h.cell(4, 0).fg, Color::Reset);

        // Esc reverts to the theme that was active when the picker opened.
        h.key(KeyCode::Esc);
        assert!(!h.app.overlay.is_open());
        assert_eq!(h.app.theme.name, "Dark (Catppuccin Mocha)");
        assert_eq!(h.cell(4, 0).fg, g_mocha);

        // This time keep the previewed theme.
        h.ctrl('t');
        h.key(KeyCode::Down);
        h.key(KeyCode::Enter);
        assert!(!h.app.overlay.is_open());
        assert_eq!(h.app.theme.name, "Light (Catppuccin Latte)");
        assert_eq!(h.app.config.theme, "Light (Catppuccin Latte)");
        assert!(h.contains("theme: Light"));
    }

    #[test]
    fn default_theme_is_dark() {
        let h = Harness::new(40, 6);
        assert_eq!(h.app.theme.name, "Dark (Catppuccin Mocha)");
        assert_eq!(
            crate::theme::Theme::default().name,
            "Dark (Catppuccin Mocha)"
        );
    }

    #[test]
    fn autocomplete_popup_appears_and_accepts() {
        let mut h = Harness::with_text("counter = 0\n", 40, 10);
        h.app
            .buf_mut()
            .set_cursor(Position { line: 1, col: 0 }, false);
        h.draw();

        h.type_str("G $c");
        assert!(h.app.completion.is_some(), "popup should be open on `$c`");
        assert!(h.contains("counter"), "candidate shown:\n{}", h.screen());

        h.key(KeyCode::Enter); // Enter accepts (Tab also works)
        assert!(h.app.completion.is_none(), "popup closes on accept");
        assert_eq!(h.app.buf().line_text(1), "G $counter");
        assert_eq!(
            h.app.buf().line_count(),
            2,
            "Enter accepted, did not add a line"
        );
    }

    #[test]
    fn undefined_variable_is_flagged_in_the_status_bar() {
        let mut h = Harness::with_text("name = 1\nG $name\n", 60, 8);
        assert!(
            !h.contains("undefined"),
            "clean file, no warning:\n{}",
            h.screen()
        );

        // introduce a typo
        h.app.buffers[0] = Buffer::from_str("name = 1\nG $naem\n");
        h.draw();
        assert_eq!(h.app.diagnostics.len(), 1);
        assert!(h.contains("⚠ 1 undefined var"), "status:\n{}", h.screen());

        // the flagged span is red in the editor (col 3 + 5-wide gutter,
        // which now reserves 1 extra column for the hex-color swatch)
        let cell = h.cell(8, 1); // the 'n' of `naem` on row 1
        assert_eq!(cell.fg, h.app.theme.output_err);
        assert!(cell.modifier.contains(ratatui::style::Modifier::UNDERLINED));
    }

    #[test]
    fn enter_makes_a_newline_when_the_hint_has_nothing_to_insert() {
        let mut h = Harness::with_text("", 40, 8);
        h.type_str("K");
        assert!(h.app.completion.is_some());
        h.key(KeyCode::Enter);
        assert!(h.app.completion.is_none());
        assert_eq!(
            h.app.buf().rope().to_string(),
            "K\n",
            "Enter still broke the line"
        );
    }

    #[test]
    fn autocomplete_string_methods_and_functions() {
        let mut h = Harness::with_text("F shout(msg)\n  G $msg.U\n~\n", 50, 14);
        h.app
            .buf_mut()
            .set_cursor(Position { line: 3, col: 0 }, false);
        h.draw();

        // function name completes with its signature shown
        h.type_str("G $sh");
        assert!(h.app.completion.is_some());
        assert!(h.contains("shout"), "fn candidate:\n{}", h.screen());
        assert!(h.contains("fn(msg)"), "signature shown:\n{}", h.screen());
        h.key(KeyCode::Enter);
        assert_eq!(h.app.buf().line_text(3), "G $shout");

        // `.` offers the string methods
        h.type_str(".");
        assert!(h.app.completion.is_some());
        assert!(h.contains("UPPERCASE"), "methods:\n{}", h.screen());
        h.type_str("L");
        assert_eq!(h.app.completion.as_ref().unwrap().items.len(), 1);
        h.key(KeyCode::Enter);
        assert_eq!(h.app.buf().line_text(3), "G $shout.L");
    }

    #[test]
    fn autocomplete_command_hint() {
        let mut h = Harness::with_text("", 50, 10);
        h.type_str("K");
        assert!(h.app.completion.is_some(), "hint on a lone command letter");
        assert!(
            h.contains("read a line of input"),
            "screen:\n{}",
            h.screen()
        );
        // adding an expression ends the hint
        h.type_str(" $x");
        assert!(h.app.completion.is_none());
    }

    #[test]
    fn autocomplete_dismisses_on_esc_without_editing() {
        let mut h = Harness::with_text("value = 1\n", 40, 10);
        h.app
            .buf_mut()
            .set_cursor(Position { line: 1, col: 0 }, false);
        h.draw();

        h.type_str("$va");
        assert!(h.app.completion.is_some());
        h.key(KeyCode::Esc);
        assert!(h.app.completion.is_none());
        assert_eq!(h.app.buf().line_text(1), "$va");
        assert!(
            !h.contains("value = 1\nvalue"),
            "no completion was inserted"
        );
    }

    #[test]
    fn enter_and_autoindent_render() {
        let mut h = Harness::new(40, 8);
        h.type_str("? $x > 1\n");
        assert_eq!(h.app.buf().cursor(), Position { line: 1, col: 4 });
        h.type_str("G\"big\"");
        assert!(h.line(1).ends_with("    G\"big\""));
    }

    #[test]
    fn backspace_and_undo_via_keys() {
        let mut h = Harness::new(40, 6);
        h.type_str("abcd");
        h.key(KeyCode::Backspace);
        assert_eq!(h.app.buf().rope().to_string(), "abc");
        h.ctrl('z'); // undo the backspace
        assert_eq!(h.app.buf().rope().to_string(), "abcd");
        h.ctrl('z'); // undo the typing group
        assert_eq!(h.app.buf().rope().to_string(), "");
        h.ctrl('y');
        assert_eq!(h.app.buf().rope().to_string(), "abcd");
    }

    #[test]
    fn shift_arrow_builds_selection() {
        let mut h = Harness::with_text("hello world", 40, 6);
        h.key(KeyCode::End);
        for _ in 0..5 {
            h.key_mods(KeyCode::Left, KeyModifiers::SHIFT);
        }
        assert_eq!(
            h.app.buf().selection(),
            Some((Position { line: 0, col: 6 }, Position { line: 0, col: 11 }))
        );
        h.key(KeyCode::Backspace);
        assert_eq!(h.app.buf().rope().to_string(), "hello ");
    }

    #[test]
    fn cursor_stays_visible_when_scrolling_down() {
        let mut h = Harness::new(40, 6); // ~4 text rows + status
        for i in 0..20 {
            h.type_str(&format!("line{i}\n"));
        }
        h.type_str("LAST");
        assert!(
            h.contains("LAST"),
            "cursor line must be on screen:\n{}",
            h.screen()
        );
        assert!(!h.contains("line0"), "top should have scrolled away");
    }

    #[test]
    fn horizontal_scroll_follows_cursor() {
        let mut h = Harness::new(24, 5);
        h.type_str(&"x".repeat(60));
        assert_eq!(h.app.buf().cursor().col, 60);
        // the line is far wider than the viewport, so it must be clipped
        assert!(h.screen().matches('x').count() < 60);
        // and the cursor end of the line stays visible
        assert!(h.line(0).ends_with('x'));
    }

    #[test]
    fn ctrl_s_on_untitled_opens_save_as() {
        let mut h = Harness::new(70, 12);
        h.type_str("G\"hi\"");
        h.ctrl('s');
        assert!(h.contains("Save As"));
        assert!(matches!(
            h.app.overlay,
            crate::ui::overlay::Overlay::Prompt(_)
        ));
        // editor keystrokes are captured by the overlay now
        h.type_str("abc");
        assert_eq!(h.app.buf().rope().to_string(), "G\"hi\"");
        h.key(KeyCode::Esc);
        assert!(!h.app.overlay.is_open());
        assert!(h.contains("save cancelled"));
    }

    #[test]
    fn save_as_writes_file_and_closes() {
        let path = std::env::temp_dir().join(format!("vulide_saveas_{}.vp", std::process::id()));
        let _ = std::fs::remove_file(&path);

        let mut h = Harness::new(80, 12);
        h.type_str("G\"saved\"");
        h.ctrl('s');
        // clear the prefilled path, type our own
        for _ in 0..200 {
            h.key(KeyCode::Backspace);
        }
        h.type_str(path.to_str().unwrap());
        h.key(KeyCode::Enter);

        assert!(!h.app.overlay.is_open());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "G\"saved\"\n");
        assert!(!h.app.buf().is_dirty());
        assert_eq!(h.app.buf().path(), Some(path.as_path()));
        std::fs::remove_file(&path).ok();

        // a second Ctrl+S now writes straight through, no overlay
        h.type_str("!");
        h.ctrl('s');
        assert!(!h.app.overlay.is_open());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "G\"saved\"!\n");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn tabs_open_switch_and_close() {
        let mut h = Harness::new(60, 12);
        h.type_str("first");
        h.ctrl('n'); // new tab
        assert_eq!(h.app.buffers.len(), 2);
        assert_eq!(h.app.active, 1);
        h.type_str("second");
        assert!(h.contains("[2/2]")); // status bar shows the tab count

        h.key_mods(KeyCode::BackTab, KeyModifiers::CONTROL); // prev tab
        assert_eq!(h.app.active, 0);
        assert_eq!(h.app.buf().rope().to_string(), "first");

        h.key_mods(KeyCode::Tab, KeyModifiers::CONTROL); // next tab
        assert_eq!(h.app.active, 1);

        // dirty tab won't close without discard
        h.ctrl('w');
        assert_eq!(h.app.buffers.len(), 2);
        assert!(h.contains("unsaved changes"));
    }

    #[test]
    fn command_palette_filters_and_runs() {
        let mut h = Harness::new(80, 16);
        assert_eq!(h.app.buffers.len(), 1);

        h.ctrl('p');
        assert!(h.contains("Commands"));
        h.type_str("new tab"); // fuzzy filter
        h.key(KeyCode::Enter);

        assert!(!h.app.overlay.is_open());
        assert_eq!(h.app.buffers.len(), 2, "palette ran 'New Tab'");
    }

    #[test]
    fn palette_sets_theme_directly() {
        let mut h = Harness::new(80, 16);
        assert_eq!(h.app.theme.name, "Dark (Catppuccin Mocha)");
        h.ctrl('p');
        h.type_str("theme nord"); // matches the "Theme: Nord" entry
        h.key(KeyCode::Enter);
        assert_eq!(h.app.theme.name, "Nord");
        assert_eq!(h.app.config.theme, "Nord");
    }

    #[test]
    fn ctrl_o_opens_a_file_in_a_new_tab() {
        let path = std::env::temp_dir().join(format!("vulide_open_{}.vul", std::process::id()));
        std::fs::write(&path, "G\"from disk\"\n").unwrap();

        let mut h = Harness::new(80, 14);
        h.type_str("scratch");
        h.ctrl('o');
        for _ in 0..300 {
            h.key(KeyCode::Backspace);
        }
        h.type_str(path.to_str().unwrap());
        h.key(KeyCode::Enter);

        assert!(!h.app.overlay.is_open());
        assert_eq!(h.app.buffers.len(), 2);
        assert_eq!(h.app.buf().rope().to_string(), "G\"from disk\"");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn ctrl_o_on_a_directory_shows_the_file_tree_instead_of_erroring() {
        let dir = std::env::temp_dir().join(format!("vulide_opendir_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.vul"), "Q\n").unwrap();

        let mut h = Harness::new(90, 20);
        h.ctrl('o');
        for _ in 0..300 {
            h.key(KeyCode::Backspace);
        }
        h.type_str(dir.to_str().unwrap());
        h.key(KeyCode::Enter);

        assert!(!h.app.overlay.is_open(), "prompt closed, not an error");
        assert_eq!(
            h.app.buffers.len(),
            1,
            "no tab was opened for the directory"
        );
        assert!(h.app.show_files);
        assert_eq!(h.app.focus, crate::app::Focus::Files);
        assert!(h.contains("a.vul"), "tree shows the directory's contents");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn word_wrap_flows_a_long_line_onto_several_rows() {
        let long = "G \"".to_string() + &"ab ".repeat(20) + "\"";
        // +1 vs. the old fixture width: the gutter grew by one column for
        // the hex-color swatch, so this keeps the same text width (and so
        // the same wrap boundaries) as before that change.
        let mut h = Harness::with_text(&long, 31, 10);

        // wrap off: the tail is not on screen
        assert!(!h.contains("ab \""), "no wrap yet:\n{}", h.screen());

        h.ctrl('p');
        h.type_str("word wrap");
        h.key(KeyCode::Enter);
        assert!(h.app.config.word_wrap);

        // now the whole line is visible across rows, gutter blank on continuations
        assert!(h.contains("ab \""), "wrapped tail shown:\n{}", h.screen());
        assert_eq!(
            h.line(1).trim_start().chars().next(),
            Some('a'),
            "cont row 2:\n{}",
            h.screen()
        );
        assert!(h.line(1).starts_with("     "), "continuation gutter blank");
    }

    #[test]
    fn word_wrap_keeps_the_cursor_on_its_visual_row() {
        let long = "x".repeat(60);
        let mut h = Harness::with_text(&long, 30, 8); // text width ~26
        h.app.config.word_wrap = true;
        h.draw();
        // cursor at end of the (wrapped) line
        h.key(KeyCode::End);
        h.draw();
        let (_cx, cy) = h.cursor_xy();
        assert!(cy >= 2, "cursor rode the wrap down to row {cy}");
    }

    // ---- hex-color gutter swatch ----

    #[test]
    fn a_hex_color_shows_a_matching_swatch_in_the_gutter() {
        let h = Harness::with_text("hex = \"#00FF00\"\nplain line\n", 60, 8);
        // The swatch sits at the leftmost gutter column, before the number.
        assert_eq!(h.cell(0, 0).fg, ratatui::style::Color::Rgb(0, 255, 0));
        assert_eq!(h.line(0).chars().next(), Some('●'));
        // A line with no hex color gets a blank swatch column, not a stale one.
        assert_ne!(h.line(1).chars().next(), Some('●'));
    }

    #[test]
    fn the_swatch_column_does_not_depend_on_line_number_padding() {
        // Enough lines that later ones have no spare leading padding in the
        // line-number field — the swatch must still show up regardless.
        let mut text = String::new();
        for i in 1..=15 {
            text.push_str(&format!("line {i}\n"));
        }
        text.push_str("color here #ABCDEF\n");
        let h = Harness::with_text(&text, 60, 20);
        let row = 15u16; // 0-indexed row of the "color here" line
        assert_eq!(
            h.cell(0, row).fg,
            ratatui::style::Color::Rgb(0xAB, 0xCD, 0xEF)
        );
    }

    #[test]
    fn word_wrap_can_scroll_to_the_end_of_a_giant_line() {
        // One line longer than a whole screen of wrapped rows.
        let long = "abcdefghij".repeat(30); // 300 chars
        let mut h = Harness::with_text(&long, 24, 6); // ~20 wide, ~4 body rows
        h.app.config.word_wrap = true;
        h.key(KeyCode::End); // jump to the end of the giant line
        h.type_str("ZEND"); // ...then mark it
        h.draw();

        assert!(
            h.contains("ZEND"),
            "tail reachable under wrap:\n{}",
            h.screen()
        );
        let (_cx, cy) = h.cursor_xy();
        assert!(cy < 6, "cursor stays on screen (row {cy})");
        assert!(h.app.buf().scroll_subrow > 0, "scrolled into the line");
    }

    #[test]
    fn window_title_tracks_file_and_dirty_state() {
        let mut h = Harness::new(80, 10);
        assert_eq!(h.app.window_title(), "VulIDE — Untitled");

        h.type_str("G\"hi\"");
        assert_eq!(
            h.app.window_title(),
            "VulIDE — Untitled •",
            "unsaved edits get the dot"
        );

        let path = std::env::temp_dir().join(format!("vulide_title_{}.vul", std::process::id()));
        std::fs::write(&path, "G\"disk\"\n").unwrap();
        h.app.open_path(path.clone()).unwrap();
        assert_eq!(
            h.app.window_title(),
            format!("VulIDE — {}", path.file_name().unwrap().to_string_lossy())
        );
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn toggle_line_numbers_via_palette() {
        let mut h = Harness::with_text("G\"x\"", 40, 8);
        assert!(
            h.line(0).starts_with("   1 "),
            "gutter present: {:?}",
            h.line(0)
        );
        h.ctrl('p');
        h.type_str("line numbers");
        h.key(KeyCode::Enter);
        assert!(!h.app.config.show_line_numbers);
        assert_eq!(h.line(0), "G\"x\"", "gutter gone:\n{}", h.screen());
    }

    // ---- Phase 4: run + output console ----

    fn wait_for_exit(h: &mut Harness) {
        let done = h.pump_until(Duration::from_secs(5), |a| {
            a.run.as_ref().is_some_and(|r| !r.is_running())
        });
        assert!(done, "process did not finish:\n{}", h.screen());
    }

    #[test]
    fn run_streams_stdout_into_the_panel() {
        let mut h = Harness::new(60, 20);
        h.app
            .start_run_argv(vec!["printf".into(), "line one\nline two\n".into()]);
        h.draw();
        assert!(h.contains("printf"), "panel title missing:\n{}", h.screen()); // panel title shows the command
        wait_for_exit(&mut h);

        assert!(h.contains("line one"), "stdout not shown:\n{}", h.screen());
        assert!(h.contains("line two"));
        assert_eq!(h.app.run.as_ref().unwrap().exit_code, Some(0));
        assert!(h.contains("exit 0"));
    }

    #[test]
    fn run_shows_stderr_and_nonzero_exit() {
        let mut h = Harness::new(60, 20);
        h.app.start_run_argv(vec![
            "sh".into(),
            "-c".into(),
            "echo good; echo bad 1>&2; exit 3".into(),
        ]);
        wait_for_exit(&mut h);

        let console = h.app.run.as_ref().unwrap();
        assert_eq!(console.exit_code, Some(3));
        assert!(console.rows.iter().any(|r| r.text == "good"));
        assert!(
            console
                .rows
                .iter()
                .any(|r| { r.text == "bad" && r.stream == crate::event::OutputStream::Stderr })
        );
        assert!(h.contains("exit 3"));
    }

    #[test]
    fn stdin_round_trips_through_cat() {
        let mut h = Harness::new(60, 20);
        h.app.start_run_argv(vec!["cat".into()]);
        assert_eq!(h.app.focus, crate::app::Focus::Output);

        h.type_str("ping"); // goes to the panel's stdin line
        h.key(KeyCode::Enter);

        let echoed = h.pump_until(Duration::from_secs(5), |a| {
            a.run
                .as_ref()
                .unwrap()
                .rows
                .iter()
                .any(|r| r.text == "ping" && r.stream == crate::event::OutputStream::Stdout)
        });
        assert!(echoed, "cat did not echo stdin:\n{}", h.screen());

        h.ctrl('d'); // close stdin → cat exits
        wait_for_exit(&mut h);
        assert_eq!(h.app.run.as_ref().unwrap().exit_code, Some(0));
    }

    #[test]
    fn stop_terminates_a_running_process() {
        let mut h = Harness::new(60, 20);
        h.app.start_run_argv(vec!["sleep".into(), "30".into()]);
        h.pump();
        assert!(h.app.run.as_ref().unwrap().is_running());

        h.app.stop_run();
        assert!(h.app.run.as_ref().unwrap().stopped);
        assert!(!h.app.run.as_ref().unwrap().is_running());
        assert_eq!(h.app.run.as_ref().unwrap().exit_code, None);

        // drain the StreamClosed events from the killed pipes; state stays stopped
        for _ in 0..20 {
            h.pump();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(h.app.run.as_ref().unwrap().stopped);
        assert!(h.contains("stopped"));
    }

    #[test]
    fn f5_runs_the_saved_file() {
        // A real interpreter probably isn't on PATH in CI; point vulpin_path at
        // a stub that echoes its file argument.
        let dir = std::env::temp_dir();
        let stub = dir.join(format!("vulide_stub_{}.sh", std::process::id()));
        std::fs::write(&stub, "#!/bin/sh\necho \"ran: $1\"\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let src = dir.join(format!("vulide_prog_{}.vul", std::process::id()));
        std::fs::write(&src, "G\"hi\"\n").unwrap();

        let mut h = Harness::new(70, 20);
        h.app.config.vulpin_path = stub.to_string_lossy().into_owned();
        h.app.open_path(src.clone()).unwrap();
        h.draw();

        h.key(KeyCode::F(5));
        wait_for_exit(&mut h);
        assert!(
            h.contains(&format!("ran: {}", src.display())),
            "stub output missing:\n{}",
            h.screen()
        );

        std::fs::remove_file(&stub).ok();
        std::fs::remove_file(&src).ok();
    }

    #[test]
    fn run_button_is_visible_and_clickable() {
        let dir = std::env::temp_dir();
        let stub = dir.join(format!("vulide_btn_stub_{}.sh", std::process::id()));
        std::fs::write(&stub, "#!/bin/sh\necho clicked-run\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let src = dir.join(format!("vulide_btn_prog_{}.vul", std::process::id()));
        std::fs::write(&src, "G\"x\"\n").unwrap();

        let mut h = Harness::new(70, 16);
        h.app.config.vulpin_path = stub.to_string_lossy().into_owned();
        h.app.open_path(src.clone()).unwrap();
        h.draw();

        assert!(h.contains("▶ Run"), "button missing:\n{}", h.screen());
        let btn = h.app.run_button.expect("button rect recorded");
        h.click(btn.x + 2, btn.y); // click on the "▶"
        wait_for_exit(&mut h);
        assert!(
            h.contains("clicked-run"),
            "click didn't run:\n{}",
            h.screen()
        );

        std::fs::remove_file(&stub).ok();
        std::fs::remove_file(&src).ok();
    }

    #[test]
    fn f1_opens_help_and_esc_closes() {
        let mut h = Harness::new(90, 44);
        h.key(KeyCode::F(1));
        assert!(h.contains("Keys & Shortcuts"));
        assert!(h.contains("save (Save As if untitled)"));
        assert!(h.contains("run the current file"));
        // The card has grown past what a 44-row terminal shows in one page —
        // "command palette" is further down, in "View & commands".
        for _ in 0..200 {
            h.key(KeyCode::Down);
        }
        assert!(
            h.contains("command palette"),
            "scrolled into view:\n{}",
            h.screen()
        );
        h.key(KeyCode::Esc);
        assert!(!h.app.overlay.is_open());

        // short terminal: scrolls instead of overflowing
        let mut h = Harness::new(90, 14);
        h.key(KeyCode::F(1));
        assert!(h.contains("Keys & Shortcuts"));
        for _ in 0..200 {
            h.key(KeyCode::Down);
        }
        assert!(h.contains("Shift + drag"), "scroll reached the last line");
    }

    #[test]
    fn palette_has_a_help_entry() {
        let mut h = Harness::new(80, 24);
        h.ctrl('p');
        h.type_str("help");
        h.key(KeyCode::Enter);
        assert!(matches!(
            h.app.overlay,
            crate::ui::overlay::Overlay::Help(_)
        ));
        assert!(h.contains("Keys & Shortcuts"));
    }

    #[test]
    fn click_outside_an_overlay_dismisses_it() {
        let mut h = Harness::new(90, 30);
        h.key(KeyCode::F(1));
        assert!(h.app.overlay.is_open());
        let r = h.app.overlay_rect.expect("overlay rect recorded");

        // a click inside the box does not close it
        h.click(r.x + 1, r.y + 1);
        assert!(h.app.overlay.is_open());

        // a click in the corner (outside) does
        h.click(0, 0);
        assert!(!h.app.overlay.is_open());
    }

    #[test]
    fn click_outside_theme_picker_reverts_preview() {
        let mut h = Harness::new(90, 30);
        h.ctrl('t');
        h.key(KeyCode::Down); // preview Latte
        assert_eq!(h.app.theme.name, "Light (Catppuccin Latte)");
        h.click(0, 0); // click away
        assert!(!h.app.overlay.is_open());
        assert_eq!(h.app.theme.name, "Dark (Catppuccin Mocha)");
    }

    #[test]
    fn clicking_between_panes_moves_focus() {
        let mut h = Harness::new(70, 20);
        h.app.start_run_argv(vec!["sleep".into(), "30".into()]);
        h.pump();
        assert_eq!(h.app.focus, crate::app::Focus::Output);

        let editor_row = 2;
        h.click(5, editor_row);
        assert_eq!(h.app.focus, crate::app::Focus::Editor, "click in editor");

        let pr = h.app.panel_rect.expect("panel rect");
        h.click(pr.x + 2, pr.y + 1);
        assert_eq!(h.app.focus, crate::app::Focus::Output, "click in panel");

        h.app.stop_run();
    }

    #[test]
    fn focus_returns_to_editor_when_the_run_finishes() {
        let mut h = Harness::new(70, 20);
        h.app.start_run_argv(vec!["printf".into(), "done\n".into()]);
        assert_eq!(h.app.focus, crate::app::Focus::Output);
        wait_for_exit(&mut h);
        assert_eq!(
            h.app.focus,
            crate::app::Focus::Editor,
            "keyboard should be back in the editor after the run"
        );
    }

    #[test]
    fn splitter_drag_resizes_the_panel() {
        let mut h = Harness::new(80, 30);
        h.app.start_run_argv(vec!["sleep".into(), "30".into()]);
        h.pump();
        let start_h = h.app.panel_rect.expect("panel").height;
        let sp = h.app.splitter_rect.expect("splitter rect");

        // drag the splitter up by 5 rows → panel grows
        h.drag(sp.x + sp.width / 2, sp.y, sp.x + sp.width / 2, sp.y - 5);
        let new_h = h.app.panel_rect.expect("panel").height;
        assert!(new_h > start_h, "panel {start_h} -> {new_h}");
        assert!(!h.app.dragging_splitter, "drag released");

        h.app.stop_run();
    }

    #[test]
    fn panel_close_button_closes_the_output() {
        let mut h = Harness::new(70, 20);
        h.app.start_run_argv(vec!["sleep".into(), "30".into()]);
        h.pump();
        let x = h.app.panel_close_rect.expect("close rect");
        h.click(x.x, x.y);
        assert!(h.app.run.is_none(), "output panel closed");
        assert_eq!(h.app.focus, crate::app::Focus::Editor);
    }

    #[test]
    fn ctrl_w_in_the_output_panel_closes_the_panel_not_a_tab() {
        let mut h = Harness::with_text("first", 70, 20);
        h.ctrl('n'); // second tab, so a wrong close would be visible
        assert_eq!(h.app.buffers.len(), 2);
        h.app.start_run_argv(vec!["sleep".into(), "30".into()]);
        h.pump();
        h.app.focus = crate::app::Focus::Output;

        h.ctrl('w');
        assert!(h.app.run.is_none(), "the panel closed");
        assert_eq!(h.app.buffers.len(), 2, "no tab was closed");
        assert_eq!(h.app.focus, crate::app::Focus::Editor);
    }

    #[test]
    fn ctrl_w_still_closes_the_active_tab_from_files_and_outline_focus() {
        let dir = std::env::temp_dir().join(format!("vulide_ctrlw_files_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut h = Harness::new(80, 20);
        h.app.open_path(dir.clone()).unwrap(); // opens the file tree, one tab
        h.ctrl('n'); // a second, clean tab so Ctrl+W has something safe to close
        assert_eq!(h.app.buffers.len(), 2);
        h.app.focus = crate::app::Focus::Files;

        h.ctrl('w');
        assert_eq!(
            h.app.buffers.len(),
            1,
            "Ctrl+W still closed a tab from Files focus"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn f9_closes_the_output_panel_with_no_mouse() {
        let mut h = Harness::new(70, 20);
        h.app.start_run_argv(vec!["sleep".into(), "30".into()]);
        h.pump();
        assert!(h.app.run.is_some());
        h.key(KeyCode::F(9));
        assert!(h.app.run.is_none(), "output panel closed");
        assert_eq!(h.app.focus, crate::app::Focus::Editor);
    }

    #[test]
    fn f9_with_no_output_panel_is_a_harmless_no_op() {
        let mut h = Harness::new(70, 20);
        h.key(KeyCode::F(9));
        assert!(h.app.run.is_none());
        assert!(!h.app.should_quit());
    }

    #[test]
    fn f11_and_f12_resize_the_output_panel_with_no_mouse() {
        let mut h = Harness::new(80, 30);
        h.app.start_run_argv(vec!["sleep".into(), "30".into()]);
        h.pump();
        let start_h = h.app.panel_rect.expect("panel").height;

        h.key(KeyCode::F(11)); // grow
        let grown = h.app.panel_rect.expect("panel").height;
        assert!(grown > start_h, "panel {start_h} -> {grown}");

        h.key(KeyCode::F(12)); // shrink
        h.key(KeyCode::F(12));
        let shrunk = h.app.panel_rect.expect("panel").height;
        assert!(shrunk < grown, "panel {grown} -> {shrunk}");

        h.app.stop_run();
    }

    #[test]
    fn f11_with_no_output_panel_says_so_instead_of_panicking() {
        let mut h = Harness::new(80, 30);
        h.key(KeyCode::F(11));
        assert!(h.contains("no output panel to resize"));
    }

    #[test]
    fn clicking_a_tab_switches_and_its_x_closes_it() {
        let mut h = Harness::new(70, 14);
        h.ctrl('n');
        h.ctrl('n'); // three tabs, active = 2
        assert_eq!(h.app.buffers.len(), 3);
        assert_eq!(h.app.active, 2);

        let first = h.app.tab_hits[0];
        h.click(first.rect.x + 1, first.rect.y);
        assert_eq!(h.app.active, 0, "clicked tab 0");

        // close the (now) middle tab via its ✕
        let mid = h.app.tab_hits[1];
        h.click(mid.close.x, mid.close.y);
        assert_eq!(h.app.buffers.len(), 2);
    }

    // ---- Phase 5: find / replace ----

    #[test]
    fn find_bar_highlights_and_navigates() {
        let mut h = Harness::with_text("alpha beta alpha gamma alpha", 60, 12);
        h.ctrl('f');
        assert!(h.app.search.is_some());
        h.type_str("alpha");
        assert_eq!(h.app.search_matches.len(), 3);
        assert!(h.contains("1/3"), "counter shown:\n{}", h.screen());
        // incremental: cursor jumped to (the end of) the first match
        assert_eq!(h.app.buf().cursor(), Position { line: 0, col: 5 });

        h.key(KeyCode::Enter); // next
        assert_eq!(h.app.search_idx, 1);
        assert_eq!(h.app.buf().cursor(), Position { line: 0, col: 16 });

        h.key(KeyCode::Enter);
        h.key(KeyCode::Enter); // wraps 2 -> 0
        assert_eq!(h.app.search_idx, 0);

        h.key_mods(KeyCode::Enter, KeyModifiers::SHIFT); // prev, wraps 0 -> 2
        assert_eq!(h.app.search_idx, 2);

        h.key(KeyCode::Esc);
        assert!(h.app.search.is_none());
        assert!(h.app.search_matches.is_empty());
    }

    #[test]
    fn find_is_case_insensitive_until_toggled() {
        let mut h = Harness::with_text("Foo foo FOO", 50, 10);
        h.ctrl('f');
        h.type_str("foo");
        assert_eq!(h.app.search_matches.len(), 3);
        h.key_mods(KeyCode::Char('c'), KeyModifiers::ALT); // Alt+C
        assert_eq!(h.app.search_matches.len(), 1);
        assert!(h.app.search.as_ref().unwrap().case_sensitive);
        assert!(h.contains("[Aa]"));
    }

    #[test]
    fn replace_one_then_replace_all() {
        let mut h = Harness::with_text("foo foo foo", 60, 12);
        h.ctrl('f');
        h.type_str("foo");
        assert_eq!(h.app.search_matches.len(), 3);

        h.key(KeyCode::Tab); // -> Replace field
        h.type_str("bar");
        h.ctrl('r'); // replace current + advance
        assert_eq!(h.app.buf().line_text(0), "bar foo foo");
        assert_eq!(h.app.search_matches.len(), 2);

        h.key_mods(KeyCode::Char('a'), KeyModifiers::ALT); // Alt+A replace all
        assert_eq!(h.app.buf().line_text(0), "bar bar bar");
        assert!(h.contains("replaced 2"));

        // one undo step per operation
        h.key(KeyCode::Esc);
        h.ctrl('z');
        assert_eq!(h.app.buf().line_text(0), "bar foo foo");
        h.ctrl('z');
        assert_eq!(h.app.buf().line_text(0), "foo foo foo");
    }

    #[test]
    fn find_seeds_from_selection() {
        let mut h = Harness::with_text("needle here and needle there", 60, 12);
        h.key(KeyCode::End);
        for _ in 0..5 {
            h.key_mods(KeyCode::Left, KeyModifiers::SHIFT); // select "there"
        }
        h.ctrl('f');
        assert_eq!(h.app.search.as_ref().unwrap().query(), "there");
    }

    #[test]
    fn clicking_another_tab_dismisses_the_find_bar() {
        // The bar captures the keyboard, so a tab switch only reaches here via
        // the mouse or the palette — either way its per-buffer matches must go.
        let mut h = Harness::new(70, 14);
        h.type_str("alpha alpha");
        h.ctrl('n');
        h.type_str("alpha");
        h.ctrl('f');
        h.type_str("alpha");
        assert!(h.app.search.is_some());

        let first = h.app.tab_hits[0];
        h.click(first.rect.x + 1, first.rect.y);
        assert_eq!(h.app.active, 0);
        assert!(h.app.search.is_none(), "find bar dropped on tab switch");
        assert!(h.app.search_matches.is_empty());
    }

    #[test]
    fn find_bar_shares_the_screen_with_the_output_panel() {
        let mut h = Harness::new(80, 24);
        h.app.start_run_argv(vec!["sleep".into(), "30".into()]);
        h.pump();
        h.app.focus = crate::app::Focus::Editor;
        h.ctrl('f');
        h.draw();
        // editor keeps at least the minimum height with tabs off + panel + bar
        assert!(h.app.editor_rect.height >= crate::ui::MIN_EDITOR_ROWS);
        assert!(h.app.panel_rect.is_some());
        assert!(h.app.search_rect.is_some());
        h.app.stop_run();
    }

    // ---- Phase 5: structure outline ----

    #[test]
    fn f7_shows_outline_and_enter_jumps_to_the_line() {
        let src = "G \"start\"\nF greet(name)\n  ? $name\n    R \"hi\"\n  ;\n~\nL loop\nJ loop\n";
        let mut h = Harness::with_text(src, 80, 20);

        h.key(KeyCode::F(7));
        assert!(h.app.show_algo);
        assert_eq!(h.app.focus, crate::app::Focus::Algo);
        assert!(h.contains("Outline"), "sidebar:\n{}", h.screen());
        assert!(h.contains("greet(name)"), "fn shown:\n{}", h.screen());
        assert!(h.contains("loop"), "label shown:\n{}", h.screen());

        // first item is the function on line 1 (0-based)
        assert_eq!(h.app.algo_items[0].line, 1);
        h.key(KeyCode::Enter);
        assert_eq!(h.app.focus, crate::app::Focus::Editor);
        assert_eq!(h.app.buf().cursor().line, 1);

        // the outline stays visible after a jump — F7 now just re-focuses it
        h.key(KeyCode::F(7));
        assert_eq!(h.app.focus, crate::app::Focus::Algo);
        assert!(h.app.show_algo);
        h.key(KeyCode::End); // last item = the jump on line 7
        assert_eq!(
            h.app.algo_items[h.app.algo_selected].kind,
            crate::algo::Kind::Jump
        );
        h.key(KeyCode::Enter);
        assert_eq!(h.app.buf().cursor().line, 7);

        // F7 (re-focus) then F7 (hide)
        h.key(KeyCode::F(7));
        h.key(KeyCode::F(7));
        assert!(!h.app.show_algo);
        assert!(h.app.algo_rect.is_none());
    }

    #[test]
    fn outline_click_jumps_and_narrow_terminal_hides_it() {
        let src = "F a()\n~\nF b()\n~\n";
        let mut h = Harness::with_text(src, 80, 16);
        h.key(KeyCode::F(7));
        let ar = h.app.algo_rect.expect("sidebar rect");
        // click the second row (F b) inside the panel body
        h.click(ar.x + 2, ar.y + 2);
        assert_eq!(h.app.buf().cursor().line, 2);
        assert_eq!(h.app.focus, crate::app::Focus::Editor);

        // a too-narrow terminal doesn't lay the sidebar out even when enabled
        let mut narrow = Harness::new(40, 16);
        narrow.app.show_algo = true;
        narrow.draw();
        assert!(narrow.app.algo_rect.is_none(), "hidden below min width");
        assert!(narrow.app.editor_rect.width > 0);
    }

    // ---- file tree (F2) ----

    fn tree_fixture(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("vulide_ft_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("lib")).unwrap();
        std::fs::write(dir.join("main.vul"), "G\"hi\"\n").unwrap();
        std::fs::write(dir.join("readme.txt"), "x").unwrap();
        std::fs::write(dir.join("lib").join("util.vul"), "Q\n").unwrap();
        dir
    }

    #[test]
    fn f2_shows_the_file_tree_with_directory_contents() {
        let dir = tree_fixture("show");
        let mut h = Harness::new(80, 20);
        h.app.file_tree = Some(crate::filetree::FileTree::new(&dir));

        h.key(KeyCode::F(2));
        assert!(h.app.show_files);
        assert_eq!(h.app.focus, crate::app::Focus::Files);
        assert!(h.contains("Files"), "sidebar title:\n{}", h.screen());
        assert!(h.contains("lib"), "dir listed:\n{}", h.screen());
        assert!(h.contains("main.vul"), "file listed:\n{}", h.screen());
        assert!(h.contains("readme.txt"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn opening_a_python_file_switches_grammar_and_status() {
        let path = std::env::temp_dir().join(format!("vulide_py_{}.py", std::process::id()));
        std::fs::write(&path, "def greet(name):\n    return $undef\n").unwrap();
        let mut h = Harness::new(80, 12);
        h.app.open_path(path.clone()).unwrap();
        h.draw();

        assert!(h.contains(" Python "), "status bar:\n{}", h.line(11));
        // `def` is a keyword → theme.keyword colour on the first cell
        assert_eq!(h.cell(5, 0).fg, h.app.theme.keyword); // past the 5-col gutter
        // the Vulpin `$undef` lint must NOT fire on a Python buffer
        assert!(h.app.diagnostics.is_empty(), "no Vulpin lint off-grammar");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn unknown_extension_renders_plain() {
        let path = std::env::temp_dir().join(format!("vulide_x_{}.ps1", std::process::id()));
        std::fs::write(&path, "if true { echo hi }\n").unwrap();
        let mut h = Harness::new(80, 10);
        h.app.open_path(path.clone()).unwrap();
        h.draw();
        assert!(h.contains(" Plain "));
        // `if` gets no keyword colour — falls back to plain fg
        assert_eq!(h.cell(5, 0).fg, h.app.theme.fg);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn ctrl_q_quits_immediately_when_nothing_is_unsaved() {
        let mut h = Harness::with_text("G\"hi\"", 60, 10); // from_str => not dirty
        h.ctrl('q');
        assert!(h.app.should_quit());
        assert!(!h.app.overlay.is_open());
    }

    #[test]
    fn ctrl_q_on_unsaved_asks_first() {
        let mut h = Harness::new(60, 10);
        h.type_str("G\"hi\""); // now dirty
        h.ctrl('q');
        assert!(!h.app.should_quit(), "held back by the guard");
        assert!(matches!(
            h.app.overlay,
            crate::ui::overlay::Overlay::Confirm(_)
        ));
        assert!(h.contains("unsaved changes"));

        h.key(KeyCode::Esc); // cancel
        assert!(!h.app.should_quit());
        assert!(!h.app.overlay.is_open());

        h.ctrl('q');
        h.key(KeyCode::Char('n')); // discard & quit
        assert!(h.app.should_quit());
    }

    #[test]
    fn quit_guard_y_saves_a_titled_buffer_then_quits() {
        let path = std::env::temp_dir().join(format!("vulide_qg_{}.vul", std::process::id()));
        std::fs::write(&path, "G\"a\"\n").unwrap();
        let mut h = Harness::new(60, 10);
        h.app.open_path(path.clone()).unwrap();
        h.type_str("G\"b\"\n");
        assert!(h.app.buf().is_dirty());

        h.ctrl('q');
        h.key(KeyCode::Char('y'));
        assert!(h.app.should_quit());
        assert!(std::fs::read_to_string(&path).unwrap().contains("G\"b\""));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn quit_guard_y_on_untitled_opens_save_as_instead_of_quitting() {
        let mut h = Harness::new(60, 10);
        h.type_str("G\"x\""); // dirty + untitled
        h.ctrl('q');
        h.key(KeyCode::Char('y'));
        assert!(!h.app.should_quit(), "can't quit without naming the file");
        assert!(matches!(
            h.app.overlay,
            crate::ui::overlay::Overlay::Prompt(_)
        ));
        assert!(h.contains("Save As"));
    }

    #[test]
    fn session_persists_open_files_and_restores_them() {
        let a = std::env::temp_dir().join(format!("vulide_sess_a_{}.vul", std::process::id()));
        let b = std::env::temp_dir().join(format!("vulide_sess_b_{}.vul", std::process::id()));
        std::fs::write(&a, "G\"a\"\n").unwrap();
        std::fs::write(&b, "G\"b\"\n").unwrap();

        // Session 1: open both, leave the second active.
        let mut h1 = Harness::new(80, 12);
        h1.app.open_path(a.clone()).unwrap();
        h1.app.open_path(b.clone()).unwrap();
        assert_eq!(h1.app.active, 1);
        let state = h1.app.session_state();
        assert_eq!(state.files.len(), 2);
        assert_eq!(state.active, 1);

        // Round-trip the state file (Session::save() is a no-op under cfg!(test)).
        let sf =
            std::env::temp_dir().join(format!("vulide_sess_state_{}.toml", std::process::id()));
        state.save_to(&sf).unwrap();
        let reloaded = crate::session::Session::load_from(&sf).unwrap();
        assert_eq!(reloaded, state);
        std::fs::remove_file(&sf).ok();

        // Session 2: a fresh app reopens what `reloaded` names.
        let mut h2 = Harness::new(80, 12);
        for p in &reloaded.files {
            h2.app.open_path(p.clone()).unwrap();
        }
        h2.app.active = reloaded.active.min(h2.app.buffers.len() - 1);
        assert_eq!(h2.app.buffers.len(), 2);
        assert!(
            h2.app
                .buf()
                .path()
                .unwrap()
                .ends_with(b.file_name().unwrap().to_str().unwrap())
        );

        std::fs::remove_file(&a).ok();
        std::fs::remove_file(&b).ok();
    }

    #[test]
    fn config_show_files_builds_the_tree_at_startup() {
        // A persisted `show_files = true` must have a tree ready on the first
        // frame — not an empty box that needs toggling to populate.
        let cfg = Config {
            show_files: true,
            ..Config::default()
        };
        let app = App::with_config(cfg);
        assert!(app.show_files);
        assert!(app.file_tree.is_some(), "tree built at startup");
    }

    #[test]
    fn enter_on_a_vul_file_opens_it_in_the_editor() {
        let dir = tree_fixture("open");
        let mut h = Harness::new(80, 20);
        h.app.file_tree = Some(crate::filetree::FileTree::new(&dir));
        h.key(KeyCode::F(2));

        // rows: "lib" (dir), "main.vul", "readme.txt" — step past the dir.
        h.key(KeyCode::Down);
        assert_eq!(h.app.file_tree.as_ref().unwrap().rows()[1].name, "main.vul");
        h.key(KeyCode::Enter);

        assert_eq!(h.app.focus, crate::app::Focus::Editor);
        assert!(
            h.app.buf().path().unwrap().ends_with("main.vul"),
            "opened: {:?}",
            h.app.buf().path()
        );
        assert_eq!(h.app.buf().rope().to_string(), "G\"hi\"");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn right_arrow_expands_a_directory_then_left_collapses() {
        let dir = tree_fixture("expand");
        let mut h = Harness::new(80, 20);
        h.app.file_tree = Some(crate::filetree::FileTree::new(&dir));
        h.key(KeyCode::F(2));

        assert_eq!(h.app.file_tree.as_ref().unwrap().len(), 3); // lib, main.vul, readme.txt
        h.key(KeyCode::Right); // expand "lib"
        let names: Vec<String> = h
            .app
            .file_tree
            .as_ref()
            .unwrap()
            .rows()
            .iter()
            .map(|r| r.name.clone())
            .collect();
        assert_eq!(names, vec!["lib", "util.vul", "main.vul", "readme.txt"]);
        assert!(h.contains("util.vul"), "child on screen:\n{}", h.screen());

        h.key(KeyCode::Left); // collapse "lib"
        assert_eq!(h.app.file_tree.as_ref().unwrap().len(), 3);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn f2_and_f7_stack_in_the_left_column() {
        let dir = tree_fixture("stack");
        let mut h = Harness::with_text("F a()\n~\n", 90, 24);
        h.app.file_tree = Some(crate::filetree::FileTree::new(&dir));

        h.key(KeyCode::F(2));
        h.key(KeyCode::F(7));
        h.draw();

        let fr = h.app.files_rect.expect("file tree rect");
        let ar = h.app.algo_rect.expect("outline rect");
        assert_eq!(fr.x, ar.x, "same column");
        assert!(fr.y < ar.y, "file tree above the outline");
        assert!(fr.height >= 5);
        assert!(h.contains("Files") && h.contains("Outline"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn tree_shows_a_scrollbar_when_it_overflows() {
        let dir = std::env::temp_dir().join(format!("vulide_ftsb_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for i in 0..40 {
            std::fs::write(dir.join(format!("f{i:02}.vul")), "Q\n").unwrap();
        }
        let mut h = Harness::new(80, 10); // ~6 body rows for 40 entries
        h.app.file_tree = Some(crate::filetree::FileTree::new(&dir));
        h.key(KeyCode::F(2));
        let r = h.app.files_rect.expect("tree rect");
        // some cell on the right border column is a scrollbar glyph, not the box rule
        let col = r.x + r.width - 1;
        let has_bar = (r.y + 1..r.y + r.height - 1).any(|y| {
            let s = h.cell(col, y).symbol().to_string();
            s == "\u{2588}" || s == "\u{2502}"
        });
        assert!(has_bar, "scrollbar drawn:\n{}", h.screen());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn narrow_terminal_hides_the_file_tree() {
        let dir = tree_fixture("narrow");
        let mut narrow = Harness::new(40, 16);
        narrow.app.file_tree = Some(crate::filetree::FileTree::new(&dir));
        narrow.app.show_files = true;
        narrow.draw();
        assert!(narrow.app.files_rect.is_none(), "hidden below min width");
        assert!(narrow.app.editor_rect.width > 0);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn tree_title_shows_the_root_folder_and_reveals_the_active_file() {
        let dir = tree_fixture("reveal");
        // a nested file to open
        std::fs::write(dir.join("lib").join("deep.vul"), "Q\n").unwrap();
        let mut h = Harness::new(90, 20);
        h.app.file_tree = Some(crate::filetree::FileTree::new(&dir));
        h.key(KeyCode::F(2));

        // the title carries the root folder name (tail-clipped to the column)
        assert!(h.line(0).contains("Files — "), "title:\n{}", h.line(0));
        let root_name = dir.file_name().unwrap().to_string_lossy().into_owned();
        let tail: String = root_name.chars().rev().take(6).collect();
        let tail: String = tail.chars().rev().collect();
        assert!(
            h.line(0).contains(&tail),
            "root tail in title:\n{}",
            h.line(0)
        );

        // open lib/deep.vul from disk — the tree should expand "lib" and select it
        h.app.open_path(dir.join("lib").join("deep.vul")).unwrap();
        h.draw();
        assert!(h.contains("deep.vul"), "revealed:\n{}", h.screen());
        let sel = &h.app.file_tree.as_ref().unwrap().rows()[h.app.files_selected];
        assert_eq!(sel.name, "deep.vul");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn clicking_a_tree_row_opens_that_file() {
        let dir = tree_fixture("click");
        let mut h = Harness::new(80, 20);
        h.app.file_tree = Some(crate::filetree::FileTree::new(&dir));
        h.key(KeyCode::F(2));

        let r = h.app.files_rect.expect("tree rect");
        // body rows start at r.y + 1: lib, main.vul, readme.txt
        h.click(r.x + 2, r.y + 2); // "main.vul"
        assert!(h.app.buf().path().unwrap().ends_with("main.vul"));
        assert_eq!(h.app.focus, crate::app::Focus::Editor);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn hovering_a_tab_marks_it() {
        let mut h = Harness::new(70, 14);
        h.ctrl('n'); // two tabs
        assert_eq!(h.app.hovered_tab, None);
        let t0 = h.app.tab_hits[0];
        h.mouse_move(t0.rect.x + 1, t0.rect.y);
        assert_eq!(h.app.hovered_tab, Some(0));
        h.mouse_move(0, 10); // move away (into the editor)
        assert_eq!(h.app.hovered_tab, None);
    }

    // ---- Ctrl+C is copy, not a second quit key ----

    #[test]
    fn ctrl_c_in_the_editor_never_quits() {
        let mut h = Harness::with_text("hello world", 60, 10);
        h.key(KeyCode::Right); // no selection
        h.ctrl('c');
        assert!(!h.app.should_quit());
        assert!(!h.app.overlay.is_open(), "no quit-confirm popped up either");
    }

    #[test]
    fn ctrl_c_with_no_selection_says_so_instead_of_copying() {
        let mut h = Harness::with_text("hello world", 60, 10);
        h.ctrl('c');
        assert!(h.contains("nothing selected"));
    }

    #[test]
    fn ctrl_c_with_a_selection_attempts_a_copy_and_never_quits() {
        let mut h = Harness::with_text("hello world", 60, 10);
        h.key_mods(KeyCode::Right, KeyModifiers::SHIFT);
        h.key_mods(KeyCode::Right, KeyModifiers::SHIFT);
        assert!(h.app.buf().selection_text().is_some());
        h.ctrl('c');
        assert!(!h.app.should_quit());
        // No display server in a test sandbox, so this usually fails — but it
        // must fail *softly* (a status message), never quit or panic.
        assert!(h.contains("copied") || h.contains("copy failed"));
    }

    #[test]
    fn ctrl_v_pastes_and_never_quits() {
        let mut h = Harness::with_text("hello", 60, 10);
        h.ctrl('v');
        assert!(!h.app.should_quit());
        // No display server in a test sandbox, so this usually has nothing
        // to paste — it must fail *softly* (a status message), never panic.
        assert!(h.contains("paste failed") || h.app.buf().rope() != "hello");
    }

    #[test]
    fn ctrl_d_duplicates_the_current_line() {
        let mut h = Harness::with_text("one\ntwo\nthree", 60, 10);
        h.key(KeyCode::Down); // cursor onto "two"
        h.ctrl('d');
        assert_eq!(h.app.buf().rope().to_string(), "one\ntwo\ntwo\nthree");
        assert_eq!(h.app.buf().cursor().line, 2);
    }

    #[test]
    fn alt_up_and_down_move_the_current_line() {
        let mut h = Harness::with_text("one\ntwo\nthree", 60, 10);
        h.key(KeyCode::Down); // cursor onto "two"
        h.key_mods(KeyCode::Up, KeyModifiers::ALT);
        assert_eq!(h.app.buf().rope().to_string(), "two\none\nthree");
        assert_eq!(h.app.buf().cursor().line, 0);

        h.key_mods(KeyCode::Down, KeyModifiers::ALT);
        assert_eq!(h.app.buf().rope().to_string(), "one\ntwo\nthree");
        assert_eq!(h.app.buf().cursor().line, 1);
    }

    #[test]
    fn ctrl_backspace_and_delete_remove_whole_words() {
        let mut h = Harness::with_text("foo bar baz", 60, 10);
        h.app
            .buf_mut()
            .set_cursor(Position { line: 0, col: 11 }, false);
        h.key_mods(KeyCode::Backspace, KeyModifiers::CONTROL);
        assert_eq!(h.app.buf().rope().to_string(), "foo bar ");

        h.app
            .buf_mut()
            .set_cursor(Position { line: 0, col: 0 }, false);
        h.key_mods(KeyCode::Delete, KeyModifiers::CONTROL);
        assert_eq!(h.app.buf().rope().to_string(), "bar ");
    }

    #[test]
    fn ctrl_c_in_the_output_panel_without_a_running_process_does_not_quit() {
        let mut h = Harness::with_text("G\"hi\"\n", 60, 10);
        h.app.focus = crate::app::Focus::Output; // no process ever started
        h.ctrl('c');
        assert!(!h.app.should_quit());
    }

    #[test]
    fn ctrl_q_still_quits() {
        let mut h = Harness::with_text("hello", 60, 10);
        h.ctrl('q');
        h.key(KeyCode::Char('n')); // discard & quit (no unsaved-file save prompt)
        assert!(h.app.should_quit());
    }

    #[test]
    fn the_active_tab_is_legible_not_dark_text_on_a_near_black_background() {
        let mut h = Harness::new(70, 8);
        h.ctrl('n');
        let active = h.app.tab_hits[h.app.active];
        let cell = h.cell(active.rect.x + 2, active.rect.y);
        assert_eq!(
            cell.fg, h.app.theme.fg,
            "bright text, not a background tint"
        );
        assert_eq!(
            cell.bg, h.app.theme.tab_active,
            "the tab's own highlighted bg"
        );
        assert_ne!(
            cell.fg, cell.bg,
            "foreground and background must not collapse to the same color"
        );
    }

    // ---- go to line (Ctrl+G) ----

    #[test]
    fn ctrl_g_jumps_the_cursor_to_a_line() {
        let text = (1..=20)
            .map(|n| format!("line {n}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut h = Harness::with_text(&text, 80, 14);
        h.ctrl('g');
        assert!(h.contains("Go to Line"));
        h.type_str("7");
        h.key(KeyCode::Enter);
        assert!(!h.app.overlay.is_open());
        assert_eq!(h.app.buf().cursor().line, 6); // 1-based 7 -> 0-based 6
        assert!(h.contains("line 7"));
    }

    #[test]
    fn ctrl_g_clamps_a_too_large_line_to_the_last_line() {
        let text = "a\nb\nc";
        let mut h = Harness::with_text(text, 80, 14);
        h.ctrl('g');
        h.type_str("999");
        h.key(KeyCode::Enter);
        assert_eq!(h.app.buf().cursor().line, 2); // clamped to the last line
    }

    #[test]
    fn ctrl_g_rejects_non_numeric_input_and_stays_open() {
        let mut h = Harness::with_text("only line", 80, 14);
        h.ctrl('g');
        h.key(KeyCode::Enter); // empty field
        assert!(h.app.overlay.is_open(), "stays open on a bad entry");
        assert!(h.contains("enter a line number"));
        h.key(KeyCode::Esc);
        assert!(!h.app.overlay.is_open());
    }

    // ---- project-wide search (F4) ----

    fn psearch_fixture(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("vulide_psearch_it_{tag}_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.vul"), "G\"hello\"\nP\"needle_marker\"\n").unwrap();
        std::fs::write(dir.join("other.vul"), "K x\n").unwrap();
        dir
    }

    #[test]
    fn f4_finds_a_match_and_enter_jumps_to_it() {
        let dir = psearch_fixture("jump");
        let mut h = Harness::new(90, 20);
        h.app.open_path(dir.join("other.vul")).unwrap();

        h.key(KeyCode::F(4));
        assert!(h.contains("Find in Files"));
        h.type_str("needle_marker");
        assert!(
            h.contains("main.vul:2"),
            "shows the match location:\n{}",
            h.screen()
        );

        h.key(KeyCode::Enter);
        assert!(!h.app.overlay.is_open());
        assert!(h.app.buf().path().unwrap().ends_with("main.vul"));
        assert_eq!(h.app.buf().cursor().line, 1);
        assert_eq!(
            h.app.buffers.len(),
            2,
            "opened in a new tab, other.vul stayed open"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn f4_search_is_case_insensitive_and_reuses_an_open_tab() {
        let dir = psearch_fixture("reuse");
        let mut h = Harness::new(90, 20);
        h.app.open_path(dir.join("main.vul")).unwrap();

        h.key(KeyCode::F(4));
        h.type_str("NEEDLE_MARKER");
        h.key(KeyCode::Enter);

        assert!(!h.app.overlay.is_open());
        assert_eq!(
            h.app.buffers.len(),
            1,
            "main.vul was already open — no new tab"
        );
        assert_eq!(h.app.buf().cursor().line, 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn f4_on_a_non_matching_query_shows_no_matches() {
        let dir = psearch_fixture("nomatch");
        let mut h = Harness::new(90, 20);
        h.app.open_path(dir.join("main.vul")).unwrap();

        h.key(KeyCode::F(4));
        h.type_str("xyzzy_not_here");
        assert!(h.contains("no matches"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn f4_esc_cancels_without_moving_the_cursor() {
        let dir = psearch_fixture("cancel");
        let mut h = Harness::new(90, 20);
        h.app.open_path(dir.join("main.vul")).unwrap();

        h.key(KeyCode::F(4));
        h.type_str("needle_marker");
        h.key(KeyCode::Esc);
        assert!(!h.app.overlay.is_open());
        assert_eq!(h.app.buf().cursor().line, 0, "cursor untouched by cancel");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn find_bar_regex_mode_matches_a_pattern() {
        let mut h = Harness::with_text("a1 b22 c333", 60, 10);
        h.ctrl('f');
        h.type_str("foo"); // not a valid match yet — just checking the toggle works either order
        h.key_mods(KeyCode::Char('x'), KeyModifiers::ALT); // Alt+X: regex on
        assert!(h.app.search.as_ref().unwrap().regex);
        assert!(h.contains("[.*]"));
        for _ in 0..3 {
            h.key(KeyCode::Backspace);
        }
        h.type_str(r"\d+");
        assert_eq!(h.app.search_matches.len(), 3);
    }

    #[test]
    fn find_bar_shows_the_regex_compiler_error_inline() {
        let mut h = Harness::with_text("anything", 60, 10);
        h.ctrl('f');
        h.key_mods(KeyCode::Char('x'), KeyModifiers::ALT); // Alt+X: regex on
        h.type_str("(unclosed");
        assert!(h.contains("regex error"));
        assert!(h.app.search_matches.is_empty());
    }

    #[test]
    fn clicking_the_find_bar_buttons_toggles_case_and_regex() {
        let mut h = Harness::with_text("Foo foo", 60, 10);
        h.ctrl('f');
        h.type_str("foo");
        assert_eq!(h.app.search_matches.len(), 2);

        let case_rect = h.app.search.as_ref().unwrap().case_rect.unwrap();
        h.click(case_rect.x, case_rect.y);
        assert!(h.app.search.as_ref().unwrap().case_sensitive);
        assert_eq!(h.app.search_matches.len(), 1, "clicking [Aa] requeried");

        let regex_rect = h.app.search.as_ref().unwrap().regex_rect.unwrap();
        h.click(regex_rect.x, regex_rect.y);
        assert!(h.app.search.as_ref().unwrap().regex);
    }

    #[test]
    fn f4_regex_mode_matches_a_pattern_across_files() {
        let dir = psearch_fixture("regex");
        let mut h = Harness::new(90, 20);
        h.app.open_path(dir.join("main.vul")).unwrap();

        h.key(KeyCode::F(4));
        h.key_mods(KeyCode::Char('x'), KeyModifiers::ALT); // Alt+X: regex on
        match &h.app.overlay {
            crate::ui::overlay::Overlay::ProjectSearch(ps) => assert!(ps.regex),
            _ => panic!("expected the project search overlay to stay open"),
        }
        assert!(h.contains("[.*]"));
        h.type_str("need.e_marker");
        assert!(h.contains("main.vul:2"), "regex matched:\n{}", h.screen());

        h.key(KeyCode::Enter);
        assert!(!h.app.overlay.is_open());
        assert_eq!(h.app.buf().cursor().line, 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn f4_shows_the_regex_compiler_error_inline() {
        let dir = psearch_fixture("regex_err");
        let mut h = Harness::new(90, 20);
        h.app.open_path(dir.join("main.vul")).unwrap();

        h.key(KeyCode::F(4));
        h.key_mods(KeyCode::Char('x'), KeyModifiers::ALT); // Alt+X: regex on
        h.type_str("(unclosed");
        assert!(h.contains("regex error"));

        std::fs::remove_dir_all(&dir).ok();
    }

    // ---- Projects (F8): new / open / delete ----

    fn fresh_temp_dir(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("vulide_projects_it_{tag}_{}", std::process::id()))
    }

    #[test]
    fn f8_new_project_scaffolds_a_starter_file_and_opens_it() {
        let mut h = Harness::new(80, 20);
        let dir = fresh_temp_dir("new");
        std::fs::remove_dir_all(&dir).ok(); // must not already exist

        h.key(KeyCode::F(8));
        assert!(h.contains("Projects"));
        h.key(KeyCode::Enter); // row 0: "+ New Project..."
        assert!(h.contains("New Project"));
        for _ in 0..300 {
            h.key(KeyCode::Backspace); // clear the cwd/ seed
        }
        h.type_str(dir.to_str().unwrap());
        h.key(KeyCode::Enter);

        assert!(!h.app.overlay.is_open());
        assert!(dir.join("main.vul").is_file(), "scaffolded a starter file");
        assert_eq!(h.app.focus, crate::app::Focus::Files);
        assert_eq!(
            h.app.file_tree.as_ref().unwrap().root,
            dir.canonicalize().unwrap()
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn f8_new_project_on_an_existing_directory_does_not_scaffold_over_it() {
        let mut h = Harness::new(80, 20);
        let dir = fresh_temp_dir("existing");
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("keepme.txt"), "do not touch").unwrap();

        h.key(KeyCode::F(8));
        h.key(KeyCode::Enter);
        for _ in 0..300 {
            h.key(KeyCode::Backspace);
        }
        h.type_str(dir.to_str().unwrap());
        h.key(KeyCode::Enter);

        assert!(!h.app.overlay.is_open());
        assert!(
            !dir.join("main.vul").exists(),
            "no scaffold on a real folder"
        );
        assert!(dir.join("keepme.txt").exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn f8_lists_and_opens_a_recent_project() {
        let mut h = Harness::new(80, 20);
        let dir = fresh_temp_dir("recent");
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        h.app
            .config
            .recent_projects
            .push(dir.canonicalize().unwrap());

        h.key(KeyCode::F(8));
        assert!(h.contains(&dir.display().to_string()));
        h.key(KeyCode::Down); // -> "+ Open Project..."
        h.key(KeyCode::Down); // -> the recent entry
        h.key(KeyCode::Enter);

        assert!(!h.app.overlay.is_open());
        assert_eq!(
            h.app.file_tree.as_ref().unwrap().root,
            dir.canonicalize().unwrap()
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn f8_delete_refuses_the_currently_open_project() {
        let mut h = Harness::new(80, 20);
        let dir = fresh_temp_dir("refuse_open");
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        h.app.open_path(dir.clone()).unwrap(); // opens it as the file tree root — also
        // registers it as recent_projects[0]

        h.key(KeyCode::F(8));
        h.key(KeyCode::Down);
        h.key(KeyCode::Down);
        h.key(KeyCode::Delete);
        assert!(h.contains("Delete Project"));
        let name = dir.file_name().unwrap().to_str().unwrap();
        h.type_str(name);
        h.key(KeyCode::Enter);

        // The typed name matched, so the dialog closes — but the actual
        // rm -rf is still refused, and that shows up as a status message.
        assert!(!h.app.overlay.is_open());
        assert!(h.contains("open a different file or folder"));
        assert!(dir.exists(), "refused: directory must survive");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn f8_delete_requires_typing_the_exact_name_then_removes_the_directory() {
        let mut h = Harness::new(80, 20);
        let victim = fresh_temp_dir("victim");
        std::fs::remove_dir_all(&victim).ok();
        std::fs::create_dir_all(&victim).unwrap();
        // Keep a different directory as the open project so the "currently
        // open" guard doesn't fire for the one we're deleting. Opening it
        // registers it as recent_projects[0] — put the victim at the front
        // instead, so "Down, Down" from the picker's action rows lands on it.
        let other = fresh_temp_dir("other_open");
        std::fs::remove_dir_all(&other).ok();
        std::fs::create_dir_all(&other).unwrap();
        h.app.open_path(other.clone()).unwrap();
        h.app
            .config
            .recent_projects
            .insert(0, victim.canonicalize().unwrap());

        h.key(KeyCode::F(8));
        h.key(KeyCode::Down);
        h.key(KeyCode::Down);
        h.key(KeyCode::Delete);
        assert!(h.contains("Delete Project"));

        // Wrong name: stays open, shows an error, directory untouched.
        h.type_str("not-the-right-name");
        h.key(KeyCode::Enter);
        assert!(h.app.overlay.is_open());
        assert!(h.contains("exactly"));
        assert!(victim.exists());

        for _ in 0..40 {
            h.key(KeyCode::Backspace);
        }
        let name = victim.file_name().unwrap().to_str().unwrap();
        h.type_str(name);
        h.key(KeyCode::Enter);

        assert!(!h.app.overlay.is_open());
        assert!(!victim.exists(), "confirmed delete actually removed it");
        assert!(
            !h.app
                .config
                .recent_projects
                .contains(&victim.canonicalize().unwrap_or_else(|_| victim.clone()))
        );

        std::fs::remove_dir_all(&other).ok();
    }

    #[test]
    fn f8_esc_cancels_the_delete_dialog_without_touching_the_directory() {
        let mut h = Harness::new(80, 20);
        let dir = fresh_temp_dir("esc_cancel");
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        h.app
            .config
            .recent_projects
            .push(dir.canonicalize().unwrap());

        h.key(KeyCode::F(8));
        h.key(KeyCode::Down);
        h.key(KeyCode::Down);
        h.key(KeyCode::Delete);
        h.key(KeyCode::Esc);

        assert!(!h.app.overlay.is_open());
        assert!(dir.exists());

        std::fs::remove_dir_all(&dir).ok();
    }
}
