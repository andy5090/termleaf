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

Most terminals hand applications only the *finished* Hangul syllable, hiding
the composition. Tadak normally respects the operating-system IME, but offers
an optional live 2-set (두벌식) composer that puts the process front and center
in the big-pixel focus zone.

## Features

- **Big pixel font zone** — a short cursor-side phrase is drawn with
  [Galmuri9](https://quiple.dev/font/galmuri) pixels, including complete Hangul
  and distinct Latin uppercase/lowercase glyphs.
- **Optional live Hangul composition** — press `F2` to make one enlarged
  character slot evolve in place as `ㅎ` → `하` → `한`, matching normal Hangul,
  including complex vowels (ㅘ, ㅢ …), double tails (ㄳ, ㄺ …), and the ghost
  rule (닭 + ㅏ → 달가).
- **Mechanical feel** — choose `classic`, `deep`, or `soft` built-in typewriter
  strikes; deletion uses a short, gentle, rate-limited release, while Enter
  plays a brief lever contact followed by the carriage stop and a high margin
  bell. Master, deletion, and return effects can be controlled independently
  without blocking input.
- **Open and save in-app** — choose a filename on first save, reopen another
  document, or save under a new name without leaving Tadak.
- **Focus mode** — hides all chrome so only your words remain.
- **Themes** — `paper`, true-black `night`, phosphor-green `xt`, and `amber`.
- **English or Korean UI** — guidance defaults to English and toggles in-app;
  the last language, theme, sound, and display choices are restored on launch.
- **Built-in guidance** — the startup guide explains OS vs. Live Korean input;
  its “Don’t show again” checkbox is optional, and `F1` always reopens the full
  shortcut reference.
- **Save & autosave** — plain-text documents (`.md` by default), with
  time-based autosave.
- **Small footprint** — direct dependencies are `crossterm` and `rodio`.

## Install

### macOS and Linux (recommended)

The installer detects Apple Silicon, Intel macOS, or x86_64 Linux, verifies the
release archive, and installs `tadak` into Cargo's conventional binary
directory:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/andy5090/tadak/releases/latest/download/tadak-installer.sh | sh
```

Open a new terminal if the installer updates your `PATH`, then run:

```bash
tadak
tadak memo.md
tadak --help
```

Prebuilt archives and checksums are also available from the
[latest GitHub release](https://github.com/andy5090/tadak/releases/latest).
Release history is maintained in the [changelog](CHANGELOG.md).

### Update or uninstall

Existing curl installations can always be updated by running the installation
command again. Releases after `v0.1.0` also install a small updater:

```bash
tadak-update
```

To uninstall a curl installation, first print the exact installed paths:

```bash
command -v tadak
command -v tadak-update
```

Remove only the files printed by those commands. The default paths are
`$HOME/.cargo/bin/tadak` and `$HOME/.cargo/bin/tadak-update`. Keep Cargo's
shared `env`/`env.fish` files and the directory's `PATH` entry if other Cargo
tools are installed there.

Package-manager installations should be managed by the same package manager:

```bash
# Future Homebrew package
brew upgrade tadak
brew uninstall tadak

# Cargo installation
cargo install tadak --locked
cargo uninstall tadak
```

### Build from source

```bash
git clone https://github.com/andy5090/tadak.git
cd tadak
cargo install --path . --locked
```

Linux source builds require the ALSA development package, commonly
`libasound2-dev` on Debian and Ubuntu. Homebrew and native Linux packages are
planned as additional installation paths; they will reuse the same GitHub
release artifacts.

## Keybindings

| Key | Action |
| --- | --- |
| typing | insert text from the operating-system input method |
| `F1` | open the help and startup-guide settings |
| `F2` | toggle Tadak's optional `Live Korean` composer |
| `F3` | toggle focus mode |
| `F4` | toggle the big-font zone |
| `F5` | toggle system-audio keystroke sound |
| `F6` | cycle `paper` / `night` / `xt` / `amber` theme |
| `F7` / `F8` | decrease / increase big-font size across five effective levels |
| `F9` | toggle English / Korean interface guidance |
| `F10` | open detailed sound settings (master / delete / return / key style) |
| `F11` | cycle `classic` / `deep` / `soft` typewriter sound |
| `Ctrl+O` | open a file (a missing path starts a new file there) |
| `Ctrl+S` | save; the first save asks for a filename |
| `F12` | save as (reliable in terminals) |
| `Ctrl+Shift+S` | save as when the terminal preserves the Shift modifier |
| `Ctrl+Q` / `Ctrl+C` | quit |
| arrows / Home / End / Backspace / Delete / Enter | usual editing |

`Enter` plays a typewriter margin bell and carriage-return effect. In the
`F10` panel, use `↑`/`↓` to select an option, `Space` or `←`/`→` to change it,
and `Enter`/`Esc` to close.

When saving, a filename without an extension defaults to Markdown: `memo`
becomes `memo.md`. An explicit extension is preserved, so `memo.txt` remains
`memo.txt`.

The open prompt lists document files (`.txt`, `.md`, `.markdown`, `.rst`,
`.adoc`, `.asciidoc`, `.org`, and `.tex`) and subdirectories beside the current document. Use
`↑`/`↓` to select, `Tab` to complete the highlighted path, and `Enter` to open
the document or enter a directory. Typing filters the list by filename.

## Typing Korean

By default the status bar shows `IME:OS`; use the normal Linux/desktop 한/영
shortcut and Tadak accepts the committed Korean text from that IME. No Tadak
shortcut is required.

`F2` toggles the optional `Live Korean` (`IME:LIVE`) practice mode; it is not
the operating system's 한/영 switch. In this mode Tadak maps raw English-layout keys using standard
**두벌식**, e.g. `g k s` → **한**. The big zone updates one character slot in
place as `ㅎ` → `하` → `한`, and `Backspace` disassembles that same cluster one
step at a time. Set the OS keyboard to English while using `IME:LIVE`, since
Tadak needs the raw Latin key events to expose each intermediate step.

You can also open a document directly from the shell:

```bash
tadak memo.txt
```

An existing path is loaded; a missing path becomes a new document that will be
created there on save.

## Configuration

Settings live at `~/.config/tadak/config` (a simple `key = value` file), written
on exit and editable by hand:

```
live_composition = false
focus_mode = false
sound = true
backspace_sound = true
return_sound = true
sound_profile = classic
big_font = true
font_size = 2
# theme: paper, night, xt, or amber
theme = paper
language = en
show_welcome = true
autosave_secs = 30
```

These values are updated on normal exit, so the next run restores the last
interface language, theme, audio choices, input mode, and display settings.

## Architecture

```
src/
├── main.rs        # event loop, action dispatch, autosave
├── input/
│   ├── korean.rs  # 두벌식 composition automaton (well tested)
│   └── events.rs  # key → Action mapping
├── editor/        # text buffer, cursor, composer integration, save
├── renderer/
│   ├── font.rs    # embedded Galmuri9 ASCII + complete Hangul bitmap subset
│   └── terminal.rs# ANSI painting + RAII terminal guard
├── ui/            # layout regions, themes, char widths
└── config/        # settings load/save
```

## v0.1 scope notes

- The big-font view follows up to 12 characters around the cursor and trims the
  oldest characters to fit the current terminal width.
- The built-in typewriter PCM is mixed through one persistent, low-latency
  Rodio audio stream; no player process is started for individual keystrokes.

## License

Licensed under either of

- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

The embedded Galmuri9 font subset is separately licensed under the SIL Open Font
License 1.1. See [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).
