VulIDE (TUI) — Windows test build
=================================

A terminal IDE for the Vulpin language. Rust, cross-compiled from Linux with
zig; no runtime to install.

RUN IT
------
- Best: open Windows Terminal (or PowerShell), cd to this folder, then:
      .\vulide.exe               open an empty buffer
      .\vulide.exe prog.vul      open a file

- Double-clicking vulide.exe also works — Windows gives it a console window.
  If it flashes and closes, launch it from a terminal instead (above) so you
  can read the message.

REQUIREMENTS
------------
- Windows 10 1903+ / Windows 11 (needs the modern console / ConPTY for colours
  and the alternate screen). Windows Terminal is ideal; the legacy conhost
  works but looks rougher.
- 64-bit.

THE "RUN" FEATURE (F5)
---------------------
F5 runs the current file through the Vulpin interpreter. It looks for, in order:
  vulpin.exe on PATH  (it is a C program — no python fallback)
If none is found it just says so on the status bar — the editor still works.
You can also set an explicit path in %APPDATA%\..\.config\vulide\config.toml
(key: vulpin_path), or wherever XDG_CONFIG_HOME points.

KEYS
----
F1                     keys & shortcuts card
Ctrl+P                 command palette
Ctrl+T                 theme picker
Ctrl+S / Ctrl+O        save / open
Ctrl+N / Ctrl+W        new / close tab
Ctrl+Q or Ctrl+C       quit
F5 / the ▶ button      run the file

Mouse works too (click tabs, drag the panel splitter, ✕ buttons) — it's all
optional, the editor is keyboard-first.

FEEDBACK
--------
Drop notes in the Discord. Known-not-done: algorithm viewer, search/replace,
Vim-style modal editing, soft word-wrap.
