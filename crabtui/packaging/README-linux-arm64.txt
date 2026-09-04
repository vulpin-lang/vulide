VulIDE (TUI) — Linux ARM64 test build
=====================================

A terminal IDE for the Vulpin language. Rust, static musl binary,
cross-compiled from x86-64 with zig. No shared libraries, no FUSE.

RUN IT
------
    chmod +x vulide
    ./vulide            open an empty buffer
    ./vulide prog.vul   open a file

Put it on PATH if you like:  cp vulide ~/.local/bin/

REQUIREMENTS
------------
- aarch64 Linux (Raspberry Pi 3/4/5 64-bit OS, Asahi, ARM servers, pinebook…).
- A terminal that does 256-colour / truecolour + the alternate screen — any
  modern one (foot, alacritty, kitty, konsole, gnome-terminal, xterm).

THE "RUN" FEATURE (F5)
---------------------
F5 runs the current file through the Vulpin interpreter, searched as:
  vulpin on PATH  (it is a C program — no python fallback)
If none is found the status bar just says so; the editor still works.
Set an explicit path in ~/.config/vulide/config.toml (key: vulpin_path).

KEYS:  F1 help · Ctrl+P palette · Ctrl+T theme · Ctrl+S/O save/open ·
       Ctrl+N/W tab · Ctrl+Q quit · F5 run

FEEDBACK
--------
Discord. Not done yet: algorithm viewer, search/replace, Vim modal editing,
soft word-wrap. This binary was compiled + ELF-checked on x86-64 but not run
on real aarch64 — that's what this is for.
