# VulIDE (crabtui)

A terminal UI for the [Vulpin](https://github.com/vulpin-lang/Vulpin) language —
editor, syntax highlighting, autocomplete, find/replace, a structure outline, an
undefined-variable linter, and a run console, all keyboard-first with an optional
mouse layer.

This is the **Rust track** of the VulIDE rewrite (the original is a PyQt5 desktop
app). It's built on [ratatui](https://ratatui.rs) and is pure Rust — no C
toolchain, no ncurses, one static binary per platform.

## Build

```sh
cargo build --release
./target/release/vulide [file.vul]
```

Needs a real terminal on both ends — launch it from a terminal emulator, not an
IDE "Run" panel or a pipe.

Requires a recent stable Rust (edition 2024). The Vulpin interpreter is found via
`vulpin_path` in the config, then `vulpin` / `vulpin.exe` on `PATH`.

## Keys

| | |
|---|---|
| `Ctrl+S` / `Ctrl+O` | save (Save As if untitled) / open |
| `Ctrl+N` / `Ctrl+W` | new tab / close tab |
| `Ctrl+PgUp` / `Ctrl+PgDn` | previous / next tab |
| `Ctrl+Z` / `Ctrl+Y` | undo / redo |
| `Ctrl+F` | find / replace bar (`^R` replace-one, `Alt+A` replace-all, `Alt+C` case, `Esc` close) |
| `Tab` / `Shift+Tab` | indent / dedent |
| autocomplete | `$vars` · functions · `.U/.L/.S/.T/.C` · command hints — `Enter` or `Tab` accepts |
| `F5` | run the current file |
| `F6` | toggle focus editor ↔ output |
| `F2` | file tree — `↑↓` select, `→`/`←` expand/collapse, `Enter` opens, `r` refresh, `.` hidden |
| `F7` | structure outline — `↑↓` select, `Enter` jumps to the line |
| `Ctrl+P` | command palette |
| `Ctrl+T` | theme picker (live preview) |
| `F1` / `Ctrl+H` | full keys card |
| `Ctrl+Q` | quit |

## Features

- **Editor** — rope-backed buffer, grapheme/display-column movement, undo/redo
  with coalescing, selection, auto-indent on Vulpin block openers, bracket
  matching and auto-close.
- **Syntax highlighting** — a hand-written Vulpin scanner matching
  `Vulpin/src/parser.c` (includes `O` FOR, the real operator set, no string
  escapes), plus keyword/string/comment highlighting for Python, Rust, C, Shell,
  TOML, JSON, and Markdown, picked by extension. Unknown extensions render plain.
  Current grammar shows in the status bar. The Vulpin outline, lint, and
  autocomplete switch off in non-Vulpin buffers.
- **Themes** — a 40-role theme system, 6 bundled themes, live-preview picker.
  Config at `~/.config/vulide/config.toml`.
- **Autocomplete** — pops as you type: `$name` → variables and user functions
  (with an `fn(a, b)` signature), `.` → the string methods, a lone command char →
  a one-line reminder.
- **Find / replace** — incremental, highlights every match, wraps, one-undo-step
  replace-all. Docks below the editor.
- **File tree** (`F2`) — a lazily-expanded directory view in the left column,
  rooted at the open file's folder (or the working dir). `Enter` / click opens a
  file in a tab or toggles a folder. Stacks above the outline when both are on.
- **Structure outline** (`F7`) — functions, loops, conditionals, switch/try
  blocks, labels, jumps, returns, indented by nesting; tolerant of half-typed
  code. `Enter` or click jumps the editor there.
- **Undefined-variable lint** — Vulpin reads an undefined `$name` as `None`
  *silently*. Every such reference is underlined and counted in the status bar.
- **Tabs**, a **command palette**, and a **run console** (`F5`) that streams
  stdout/stderr with line-buffered stdin.
- **Mouse layer** — all optional: run button, draggable splitter, tab
  click/close, click-to-focus, click-away-to-dismiss.
- Sets the **terminal window title** to `VulIDE — <file>` (`•` while unsaved).
- **Unsaved-changes guard** — `Ctrl+Q` / `Ctrl+C` asks before quitting if a
  buffer has unsaved edits (`Y` save & quit · `n` discard · `Esc` cancel).

## Packaging

`packaging/` has no-sudo, no-Docker build scripts for a Linux x86-64 AppImage, a
static Linux ARM64 tarball, and a Windows zip (cross-compiled with `zig`). See
the scripts for the toolchain.

## Status

v1 (`v0.1.0`) is complete — code editor, tabs, Vulpin syntax, run console, and the
algorithm/structure viewer. Post-v1 work on this branch: terminal title, file
tree, session restore, multi-language highlighting, soft word-wrap (palette ›
Word Wrap; Up/Down still move by logical line). Still open: modal editing.

## License

MIT — see [LICENSE](LICENSE).
