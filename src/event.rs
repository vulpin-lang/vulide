//! Input and background messages funnel into one `AppEvent` stream.
//!
//! A reader thread turns crossterm input into `AppEvent`s on an `mpsc` channel;
//! the run console's stdout/stderr reader threads (see `run.rs`) push onto the
//! same channel via [`EventSource::sender`]. The main loop just blocks on
//! `recv`, so nothing polls and nothing spins.

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use ratatui::crossterm::event::{self, Event as CtEvent, KeyEvent, KeyEventKind, MouseEvent};

/// Which child stream a line of output came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Paste(String),
    Resize(u16, u16),
    Tick,
    /// One line from the running child (newline already stripped).
    Output {
        stream: OutputStream,
        line: String,
    },
    /// A child stream hit EOF; when both have, the app reaps the exit code.
    StreamClosed(OutputStream),
    /// The input reader thread stopped (terminal went away).
    InputClosed,
}

pub struct EventSource {
    rx: Receiver<AppEvent>,
    tx: Sender<AppEvent>,
}

impl EventSource {
    /// `poll_rate` bounds how long the reader thread waits between input polls;
    /// it does not gate output events (those arrive as soon as they're sent).
    pub fn new(poll_rate: Duration) -> Self {
        let (tx, rx) = mpsc::channel();
        let input_tx = tx.clone();
        thread::Builder::new()
            .name("vulide-input".into())
            .spawn(move || input_loop(poll_rate, input_tx))
            .expect("spawn input thread");
        Self { rx, tx }
    }

    /// A sender for background producers (run console reader threads).
    pub fn sender(&self) -> Sender<AppEvent> {
        self.tx.clone()
    }

    /// Block for the next event. `None` only if every sender has dropped.
    pub fn next(&self) -> Result<Option<AppEvent>> {
        Ok(self.rx.recv().ok())
    }

    /// Non-blocking drain, for coalescing a burst before redrawing.
    pub fn try_next(&self) -> Option<AppEvent> {
        self.rx.try_recv().ok()
    }
}

fn input_loop(poll_rate: Duration, tx: Sender<AppEvent>) {
    let log = std::env::var_os("VULIDE_EVLOG").is_some();
    loop {
        match event::poll(poll_rate) {
            Ok(false) => continue,
            Ok(true) => {}
            Err(_) => {
                let _ = tx.send(AppEvent::InputClosed);
                return;
            }
        }
        let ev = match event::read() {
            Ok(ev) => ev,
            Err(_) => {
                let _ = tx.send(AppEvent::InputClosed);
                return;
            }
        };
        if log {
            log_event(&ev);
        }
        let mapped = match ev {
            CtEvent::Key(k) if k.kind == KeyEventKind::Press => Some(AppEvent::Key(k)),
            CtEvent::Key(_) => None,
            CtEvent::Mouse(m) => Some(AppEvent::Mouse(m)),
            CtEvent::Paste(s) => Some(AppEvent::Paste(s)),
            CtEvent::Resize(w, h) => Some(AppEvent::Resize(w, h)),
            _ => None,
        };
        if let Some(e) = mapped
            && tx.send(e).is_err()
        {
            return; // receiver gone — app is shutting down
        }
    }
}

fn log_event(ev: &CtEvent) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/vulide-events.log")
    {
        let _ = writeln!(f, "{ev:?}");
    }
}
