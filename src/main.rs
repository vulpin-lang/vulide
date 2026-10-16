//! VulIDE — terminal UI for the Vulpin language.
//!
//! Phase 0/1: a single-buffer editor. Tabs, files, run console, and the
//! algorithm viewer arrive in later phases — see `PLAN.md`.

// Several buffer/theme/event APIs exist ahead of the phase that consumes them
// (multi-buffer tabs, the 40-role theme, resize handling). Drop this as the
// later phases land.
#![allow(dead_code)]

mod algo;
mod app;
mod buffer;
mod colorswatch;
mod complete;
mod config;
mod event;
mod filetree;
mod lint;
mod matcher;
mod run;
mod search;
mod session;
mod snippets;
mod syntax;
mod theme;
mod ui;

#[cfg(test)]
mod testing;

use std::io::IsTerminal;
use std::path::PathBuf;

use anyhow::Result;
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use ratatui::crossterm::execute;

fn main() -> Result<()> {
    // A TUI needs a real terminal on both ends. IDE "Run" panels, pipes, and
    // detached processes give a non-tty stdin/stdout — the app would render but
    // never receive a keystroke. Fail loudly instead.
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        eprintln!(
            "vulide needs an interactive terminal.\n\
             Launch it yourself in a terminal emulator (Konsole, Alacritty, kitty, …):\n\
             \n    cd vulide-tui && cargo run -- <file.vul>\n\n\
             Not from an IDE 'Run' button/panel, a pipe, `nohup`, or a background job."
        );
        std::process::exit(2);
    }

    let arg = std::env::args().nth(1);

    // Save the terminal's current window title so we can put it back on exit
    // (terminals that support the title stack, xterm CSI 22/23 t).
    {
        use std::io::Write;
        let mut out = std::io::stdout();
        let _ = out.write_all(b"\x1b[22;2t");
        let _ = out.flush();
    }

    let mut terminal = ratatui::init();
    let mut app = app::App::new();
    // Mouse reporting powers the status-bar ▶ button and click-to-focus. Hold
    // Shift for the terminal's own text selection while it's on; disable it
    // entirely with `mouse = false` in the config or the palette.
    if app.config.mouse {
        let _ = execute!(std::io::stdout(), EnableMouseCapture);
    }

    if let Some(arg) = arg {
        if let Err(e) = app.open_path(PathBuf::from(arg)) {
            app.set_status(format!("open failed: {e}"));
        }
    } else {
        app.restore_session();
    }

    let result = app.run(&mut terminal);
    let _ = execute!(std::io::stdout(), DisableMouseCapture);
    ratatui::restore();
    {
        use std::io::Write;
        let mut out = std::io::stdout();
        let _ = out.write_all(b"\x1b[23;2t"); // restore the saved window title
        let _ = out.flush();
    }
    result
}
