//! Running the current file through the Vulpin interpreter and streaming its
//! output into a console panel.
//!
//! Vulpin's `K` (input) command reads line-buffered stdin (`getline`), so piped
//! stdio is enough — no PTY. One reader thread per stream forwards lines onto
//! the shared [`AppEvent`] channel; the app reaps the exit code once both
//! streams hit EOF.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Instant;

use crate::event::{AppEvent, OutputStream};

/// Hard cap on retained output lines (oldest dropped past this).
const SCROLLBACK: usize = 5000;

/// Resolve the interpreter command (argv prefix, before the file argument):
///   1. an explicit, existing `config.vulpin_path`
///   2. a `vulpin` binary on `$PATH`
///
/// Vulpin is a C program (`Vulpin/src`, built with tcc/gcc) — there is no
/// `python -m vulpin` module, so there's no Python fallback.
pub fn resolve_interpreter(config_path: &str) -> Option<Vec<String>> {
    if !config_path.trim().is_empty() {
        let p = Path::new(config_path.trim());
        if p.is_file() {
            return Some(vec![p.to_string_lossy().into_owned()]);
        }
    }
    for name in ["vulpin", "vulpin.exe"] {
        if let Some(p) = which(name) {
            return Some(vec![p.to_string_lossy().into_owned()]);
        }
    }
    None
}

/// First executable named `name` on `$PATH`.
pub fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|cand| is_executable(cand))
}

#[cfg(unix)]
fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(p: &Path) -> bool {
    p.is_file()
}

/// A spawned child plus its stdin pipe and any temp file backing an unsaved
/// buffer. Dropping it kills a still-running child and cleans the temp file.
struct Runner {
    child: Child,
    stdin: Option<ChildStdin>,
    temp_file: Option<PathBuf>,
}

impl Runner {
    fn spawn(
        argv: &[String],
        workdir: &Path,
        temp_file: Option<PathBuf>,
        tx: &Sender<AppEvent>,
    ) -> std::io::Result<Runner> {
        let (cmd, args) = argv.split_first().expect("argv is never empty");
        let mut child = Command::new(cmd)
            .args(args)
            .current_dir(workdir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        let stdin = child.stdin.take();
        spawn_reader(stdout, OutputStream::Stdout, tx.clone());
        spawn_reader(stderr, OutputStream::Stderr, tx.clone());

        Ok(Runner {
            child,
            stdin,
            temp_file,
        })
    }

    fn send_line(&mut self, line: &str) -> std::io::Result<()> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| std::io::Error::other("child stdin is closed"))?;
        stdin.write_all(line.as_bytes())?;
        stdin.write_all(b"\n")?;
        stdin.flush()
    }

    /// Close stdin so a child blocked on `K` at EOF can finish.
    fn close_stdin(&mut self) {
        self.stdin = None;
    }

    fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    fn try_reap(&mut self) -> Option<i32> {
        self.child.wait().ok().map(|s| s.code().unwrap_or(-1))
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            self.kill();
        }
        if let Some(path) = &self.temp_file {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn spawn_reader<R: std::io::Read + Send + 'static>(
    src: R,
    stream: OutputStream,
    tx: Sender<AppEvent>,
) {
    thread::Builder::new()
        .name(format!("vulide-{stream:?}"))
        .spawn(move || {
            let mut reader = BufReader::new(src);
            let mut buf = Vec::new();
            loop {
                buf.clear();
                match reader.read_until(b'\n', &mut buf) {
                    Ok(0) => break,
                    Ok(_) => {
                        while matches!(buf.last(), Some(b'\n' | b'\r')) {
                            buf.pop();
                        }
                        let line = String::from_utf8_lossy(&buf).into_owned();
                        if tx.send(AppEvent::Output { stream, line }).is_err() {
                            return;
                        }
                    }
                    Err(_) => break,
                }
            }
            let _ = tx.send(AppEvent::StreamClosed(stream));
        })
        .expect("spawn reader thread");
}

#[derive(Clone)]
pub struct OutputRow {
    pub stream: OutputStream,
    pub text: String,
}

/// State + rendering data for the output console panel.
pub struct RunConsole {
    runner: Option<Runner>,
    pub rows: VecDeque<OutputRow>,
    open_streams: u8,
    /// `Some(code)` once the process has exited; `None` while running.
    pub exit_code: Option<i32>,
    /// `true` if the run was stopped by the user.
    pub stopped: bool,
    /// Lines scrolled up from the bottom; 0 means pinned to the tail.
    pub scroll: usize,
    pub input: crate::buffer::Buffer,
    pub command: String,
    started: Instant,
    elapsed: Option<std::time::Duration>,
}

impl RunConsole {
    pub fn start(
        argv: Vec<String>,
        workdir: &Path,
        temp_file: Option<PathBuf>,
        tx: &Sender<AppEvent>,
    ) -> std::io::Result<RunConsole> {
        let runner = Runner::spawn(&argv, workdir, temp_file, tx)?;
        Ok(RunConsole {
            runner: Some(runner),
            rows: VecDeque::new(),
            open_streams: 2,
            exit_code: None,
            stopped: false,
            scroll: 0,
            input: crate::buffer::Buffer::new(),
            // one-line display form (args may themselves contain newlines)
            command: argv
                .join(" ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
            started: Instant::now(),
            elapsed: None,
        })
    }

    pub fn is_running(&self) -> bool {
        self.runner.is_some() && self.exit_code.is_none() && !self.stopped
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.elapsed.unwrap_or_else(|| self.started.elapsed())
    }

    fn push_row(&mut self, stream: OutputStream, text: String) {
        // Preserve the scroll offset in absolute terms when appending.
        let pinned = self.scroll == 0;
        self.rows.push_back(OutputRow { stream, text });
        while self.rows.len() > SCROLLBACK {
            self.rows.pop_front();
        }
        if !pinned {
            self.scroll = self.scroll.saturating_add(1).min(self.rows.len());
        }
    }

    pub fn on_output(&mut self, stream: OutputStream, line: String) {
        self.push_row(stream, line);
    }

    /// A stream closed; returns `true` when the process has now fully finished
    /// (both streams done) so the caller can reap the exit code.
    pub fn on_stream_closed(&mut self) -> bool {
        self.open_streams = self.open_streams.saturating_sub(1);
        self.open_streams == 0
    }

    pub fn reap(&mut self) {
        if let Some(runner) = &mut self.runner {
            let code = runner.try_reap().unwrap_or(-1);
            if !self.stopped {
                self.exit_code = Some(code);
            }
            self.elapsed = Some(self.started.elapsed());
        }
    }

    pub fn send_stdin(&mut self, line: &str) {
        let echoed = format!("< {line}");
        if let Some(runner) = &mut self.runner {
            match runner.send_line(line) {
                Ok(()) => self.push_row(OutputStream::Stdout, echoed),
                Err(e) => self.push_row(OutputStream::Stderr, format!("stdin: {e}")),
            }
        }
    }

    pub fn close_stdin(&mut self) {
        if let Some(runner) = &mut self.runner {
            runner.close_stdin();
        }
    }

    pub fn stop(&mut self) {
        if let Some(runner) = &mut self.runner {
            runner.kill();
        }
        self.stopped = true;
        self.exit_code = None;
        self.elapsed = Some(self.started.elapsed());
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll = (self.scroll + n).min(self.rows.len());
    }

    pub fn scroll_down(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll = 0;
    }
}
