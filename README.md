# Tadak (타닥)

A distraction-free **terminal writing app** written in Rust — big pixel fonts, a
mechanical-typewriter feel, and **live Hangul composition** that shows each
jamo assembling one piece at a time (예: `ㅎ` → `하` → `한`).

The name comes from the Korean onomatopoeia *타닥타닥* — the clatter of keys.

[![CI](https://github.com/andy5090/tadak/actions/workflows/ci.yml/badge.svg)](https://github.com/andy5090/tadak/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

> Status: v0.1 — a working foundation. Rendering is ANSI-only (via
> [`crossterm`](https://crates.io/crates/crossterm)) so it runs on virtually any
> terminal.

## Why

Most terminals hand your program only the *finished* Hangul syllable, hiding the
composition. Tadak drives composition itself (a 2-set / 두벌식 automaton), so it
can put the process front and center in a big-pixel focus zone — great for
writing practice, screencasts, or just enjoying the act of typing.

## Features

- **Big pixel font zone** — the character (or the jamo you're composing) is drawn
  large with block pixels, scalable live.
- **Live Hangul jamo-by-jamo composition** — lead consonant → vowel → tail
  consonant appear step by step, including complex vowels (ㅘ, ㅢ …), double
  tails (ㄳ, ㄺ …), and the ghost rule (닭 + ㅏ → 달가).
- **Mechanical feel** — a soft "clack" (terminal bell) on each keystroke.
- **Focus mode** — hides all chrome so only your words remain.
- **Themes** — `paper`, `night`, `amber`.
- **Save & autosave** — plain `.txt`, with time-based autosave.
- **Tiny footprint** — one dependency (`crossterm`).

## Install / run

```bash
# from the repo
cargo run --release            # start with an empty buffer
cargo run --release -- note.txt  # open (or create on save) a file
```

## Keybindings

| Key | Action |
| --- | --- |
| typing | insert text (or compose Hangul in 한 mode) |
| `F2` | toggle 한글 / English input |
| `F3` | toggle focus mode |
| `F4` | toggle the big-font zone |
| `F5` | toggle keystroke sound |
| `F6` | cycle theme |
| `F7` / `F8` | decrease / increase big-font size |
| `Ctrl+S` | save |
| `Ctrl+Q` / `Ctrl+C` | quit |
| arrows / Home / End / Backspace / Enter | usual editing |

## Typing Korean

Press `F2` to switch to 한 mode. Tadak uses the standard **두벌식** layout, e.g.
`g k s` → `ㅎ ㅏ ㄴ` → **한**. Because Tadak composes the syllable itself, the
big zone shows each jamo appearing in turn, and `Backspace` disassembles the
cluster one jamo at a time.

## Configuration

Settings live at `~/.config/tadak/config` (a simple `key = value` file), written
on exit and editable by hand:

```
hangul_mode = false
focus_mode = false
sound = true
big_font = true
font_size = 2
theme = paper
autosave_secs = 30
```

## Architecture

```
src/
├── main.rs        # event loop, action dispatch, autosave
├── input/
│   ├── korean.rs  # 두벌식 composition automaton (well tested)
│   └── events.rs  # key → Action mapping
├── editor/        # text buffer, cursor, composer integration, save
├── renderer/
│   ├── font.rs    # 5x7 ASCII + 8x8 jamo bitmap font
│   └── terminal.rs# ANSI painting + RAII terminal guard
├── ui/            # layout regions, themes, char widths
└── config/        # settings load/save
```

## v0.1 scope notes

- The big-font Hangul view shows the composing **jamo left-to-right** (the
  assembly), not a fully laid-out 2D syllable block. Composed jamo (ㅘ, ㄳ …)
  fold to a base glyph for the big view.
- Sound is the terminal bell; no audio files yet.

## License

Licensed under either of

- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
