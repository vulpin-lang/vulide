# VulIDE (crabtui) — v0.2.0 review builds

**Source:** `crabtui/` in this repo (`vulpin-lang/vulide`), from `vulide-tui` commit `e45825e`.
**Language:** Rust (edition 2024). Deps: ratatui + crossterm + ropey + toml +
serde + anyhow + unicode-{segmentation,width} — the crate has no C dependencies.

| artifact | platform | download | unpacked | build |
|---|---|---|---|---|
| `VulIDE-0.2.0-ge45825e-x86_64.AppImage` | Linux x86-64 | 1.7 MB | — | static musl → appimagetool |
| `VulIDE-0.2.0-ge45825e-aarch64-linux.tar.gz` | Linux ARM64 (Pi / Asahi / ARM servers) | 756 KB | 1.6 MB | static musl, cross-compiled with zig |
| `VulIDE-0.2.0-ge45825e-x86_64-windows.zip` | Windows x86-64 | 704 KB | 1.55 MB | cross-compiled with zig (`windows-gnullvm`) |

`SHA256SUMS` has the checksums. All three build scripts are in
`vulide-tui/packaging/` — no sudo, no Docker; the cross builds need only `zig`
+ `cargo-zigbuild` (>= 0.23 for ARM64). Version is read from `Cargo.toml`.

Windows-on-ARM runs the x64 `.exe` via emulation, so there's no separate
Win-ARM build. No macOS build — cross-compiling Darwin from Linux needs the
Apple SDK.

## Run

**Linux x86-64:**
```sh
chmod +x VulIDE-0.2.0-ge45825e-x86_64.AppImage
./VulIDE-0.2.0-ge45825e-x86_64.AppImage [file]
```
Static-pie, no shared libs. Needs FUSE 2, or `--appimage-extract-and-run`.
Double-click relaunches it inside a terminal emulator.

**Linux ARM64:** `tar xzf …`, then `./VulIDE/vulide [file]`. Raw static
binary — no FUSE, no runtime.

**Windows** (10 1903+ / 11, Windows Terminal ideal): unzip, then from a
terminal `.\VulIDE\vulide.exe [file]`. Double-click works too.

## What's new since v0.1.0

- **Terminal window title** — `VulIDE — <file>`, `•` while unsaved; restores the
  shell's title on exit.
- **File-tree sidebar** (`F2`) — lazily-expanded directory view in the left
  column, stacked above the outline when both are open. `Enter` / click opens a
  file or toggles a folder; `→`/`←` expand/collapse, `r` refresh, `.` hidden.
  Title shows the root folder; opening a file reveals + selects its row.
- **Session restore** — reopens the last session's files + active tab on launch
  (no file argument). State lives in `$XDG_STATE_HOME/vulide/session.toml`, not
  the config file. Toggle in the palette.
- **Multi-language highlighting** — `Language` picked from the extension:
  Python, Rust, C get keyword/string/comment/number colouring; `.vul` is Vulpin;
  anything else renders plain. Grammar shows in the status bar. The Vulpin
  outline + undefined-var lint switch off for non-Vulpin buffers.
- **Sidebar scrollbars** — the tree and the outline get a scrollbar when they
  overflow.
- **Soft word-wrap** — the `word_wrap` toggle is now honoured; long lines flow
  onto extra visual rows instead of scrolling. (Up/Down still move by logical
  line — visual-row navigation is a follow-up.)

## Everything from v0.1.0 (still here)

Rope-backed editor (undo/redo coalescing, selection, auto-indent, bracket
match/close) · Vulpin syntax scanner matching `Vulpin/src/parser.c` · 40-role
theme system, 6 themes, `Ctrl+T` live picker · autocomplete (`$vars` +
functions + `.U/.L/.S/.T/.C` + command hints, Enter/Tab) · incremental
find/replace (`Ctrl+F`) · structure outline (`F7`) · undefined-`$name` lint ·
multi-buffer tabs · command palette (`Ctrl+P`) · run console (`F5`, streams
stdout/stderr, line-buffered stdin) · full optional mouse layer.

## Tests

137 headless tests (`cargo test`), 0 clippy warnings, `cargo fmt` clean.
Run-console tests spawn real `printf` / `cat` / `sh` / `sleep`.

## Not verified

The x86-64 Linux build was run and works (tty guard exits 2, renders). The
**ARM64 and Windows binaries compiled clean and pass ELF/PE validation but were
not executed** (no ARM box, no Windows box on this machine). The zig linker
prints a harmless "deprecated linker optimization" warning on the cross builds.

## Still open

- Modal / Vim-like editing — blocked on BatScript picking the non-Esc
  mode-switch key.
- Visual-row navigation under word-wrap (Up/Down still move by logical line).
- Cross-line block comments in Rust/C highlighting (single-line scanner).
- Embedded terminal, Blueprints, Visual Canvas — cut from the TUI.
