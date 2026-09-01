# Termleaf

A distraction-free **terminal editor for prose**, with cursor-side big type and
optional typewriter sound.
Termleaf keeps files, shortcuts, and the writing surface close while leaving
the rest of the desktop out of the way.

> Just you, your words, and the terminal.

[![CI](https://github.com/andy5090/termleaf/actions/workflows/ci.yml/badge.svg)](https://github.com/andy5090/termleaf/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

**[Website](https://termleaf.com) · [Latest release](https://github.com/andy5090/termleaf/releases/latest)**

> Status: v0.6 — a working foundation. Rendering is ANSI-only (via
> [`crossterm`](https://crates.io/crates/crossterm)) so it runs on virtually any
> terminal.

## Why

General-purpose editors are excellent at managing code and complex projects,
but their panels, modes, and notifications can compete with the act of
writing. Termleaf is deliberately narrower: open a plain-text document, focus
on the words, and save without leaving the terminal.

## Features

- **Focus mode** — hides all chrome so only your words remain.
- **Readable page width** — optionally centers writing in an 80-column page on
  wide terminals.
- **Non-destructive soft wrapping** — long paragraphs wrap at the screen or
  page edge without adding newline characters to the saved document.
- **Relaxed line spacing** — defaults to a comfortable one-row gap and cycles
  through three persisted spacing levels.
- **Open and save in-app** — choose a filename on first save, reopen another
  document, or save under a new name without leaving Termleaf.
- **Save & autosave** — plain-text documents (`.md` by default), with
  time-based autosave.
- **Big pixel font zone** — a short cursor-side phrase is drawn with
  [Galmuri9](https://quiple.dev/font/galmuri) pixels. English is built in;
  optional Korean and Japanese packs add Hangul, kana, and CJK glyphs.
- **Optional in-app input modes** — `F2` cycles through the installed input
  languages. Live Korean makes one enlarged character slot evolve in place as
  `ㅎ` → `하` → `한`, matching normal Hangul,
  including complex vowels (ㅘ, ㅢ …), double tails (ㄳ, ㄺ …), and the ghost
  rule (닭 + ㅏ → 달가).
- **Themes** — `paper`, true-black `night`, phosphor-green `xt`, and `amber`.
- **Installable language support** — English is built in, while Korean and
  Japanese UI, guidance, enlarged-text glyphs, and the Japanese offline
  conversion model are managed from `F9`.
- **Built-in guidance** — the startup guide explains OS vs. Live Korean input;
  its “Don’t show again” checkbox is optional, and `F1` always reopens the full
  shortcut reference.
- **Mobile touch controls** — Termux starts with a localized command bar for
  narrow screens and software keyboards, exposing help, input, file, display,
  language, sound, and focus controls without function keys.
- **Optional writing ambience** — choose `classic`, `deep`, or `soft` key
  sounds, with independently configurable delete and return effects. Audio can
  be disabled completely.
- **Small core** — optional glyphs and language models stay in separately
  installed language packs.

## Install

### macOS, Linux, and Termux (recommended)

The installer detects Apple Silicon, Intel macOS, x86_64 Linux, or 32-bit x86
Linux (`i686`), as well as ARM64 Android devices running Termux. It verifies the
release archive and installs `termleaf` into Cargo's conventional binary
directory. On Debian and Ubuntu, it also installs any missing ALSA runtime
packages. This includes the PulseAudio plugin used by WSLg to route sound to
Windows. On Termux, the installer also installs `play-audio` and Termleaf uses
it from a non-blocking worker for typewriter sound. Termux installs are made
available immediately through its existing command path, regardless of whether
the active shell is bash, zsh, or fish:

```bash
curl -fsSL https://termleaf.com/install | sh
```

When building from source in Termux instead, install the audio helper once:

```bash
pkg install play-audio
```

The localized website installers add the matching language pack and select it
for the first run:

```bash
curl -fsSL https://termleaf.com/install/ko | sh
curl -fsSL https://termleaf.com/install/ja | sh
```

On macOS or desktop Linux, open a new terminal if the installer updates your
`PATH`. Termux can run the command immediately:

```bash
termleaf
termleaf memo.md
termleaf --help
```

Prebuilt archives and checksums are also available from the
[latest GitHub release](https://github.com/andy5090/termleaf/releases/latest).
Release history is maintained in the [changelog](CHANGELOG.md).

### Update or uninstall

Termleaf checks the latest published release and updates itself with one command:

```bash
termleaf update
```

To repair an incomplete installation or reinstall the current release:

```bash
termleaf update --force
```

To open a document literally named `update`, separate it from commands with
`termleaf -- update`.

To uninstall a curl installation, first print the exact installed paths:

```bash
command -v termleaf
readlink "$(command -v termleaf)" 2>/dev/null || true
```

The default executable is `$HOME/.cargo/bin/termleaf`. Termux also prints its
immediate-access link at `$PREFIX/bin/termleaf`; remove that link and the
executable when uninstalling there. Keep Cargo's shared `env`/`env.fish` files
and the directory's `PATH` entry if other Cargo tools are installed there.

### Build from source

On Debian and Ubuntu, install the build and audio runtime dependencies first:

```bash
sudo apt-get update
sudo apt-get install --yes pkg-config libasound2-dev libasound2-plugins
```

Then build Termleaf:

```bash
git clone https://github.com/andy5090/termleaf.git
cd termleaf
cargo install --path . --locked
```

Other Linux distributions need their equivalent ALSA development package and,
when the desktop audio server is PulseAudio or PipeWire, the corresponding ALSA
plugin.

## Keybindings

| Key | Action |
| --- | --- |
| typing | insert text from the operating-system input method |
| `F1` | open the help and startup-guide settings |
| `F2` | cycle installed input modes forward (`OS → Korean → Japanese`) |
| `Shift+F2` | cycle installed in-app input modes in reverse |
| `Ctrl+K` | switch Hiragana/Katakana while Live Japanese is active |
| `F3` | toggle focus mode |
| `F4` | toggle the big-font zone |
| `F5` | toggle the centered page-width (paper) mode |
| `Shift+F5` | cycle document line spacing through levels 1–3 |
| `F6` | cycle `paper` / `night` / `xt` / `amber` theme |
| `F7` / `F8` | decrease / increase big-font size across five effective levels |
| `Option+L` (macOS) / `Alt+L` (Windows/Linux) | alternate line-spacing shortcut |
| `Option+P` (macOS) / `Alt+P` (Windows/Linux) | alternate page-width shortcut |
| `F9` | open the language install/select/remove panel |
| `F10` | open sound settings (typing / delete / return / key style) |
| `F11` | cycle `classic` / `deep` / `soft` typewriter sound |
| `Ctrl+O` | open a file (a missing path starts a new file there) |
| `Ctrl+S` | save; the first save asks for a filename |
| `Ctrl+T` | toggle the touch command bar and terminal mouse capture |
| `F12` | save as (reliable in terminals) |
| `Ctrl+Shift+S` | save as when the terminal preserves the Shift modifier |
| `Ctrl+Q` / `Ctrl+C` | quit |
| arrows / Home / End / Backspace / Delete / Enter | usual editing |

`Ctrl` means the Control key on every operating system, not macOS Command or
the Windows key. macOS Option is the equivalent of Alt in terminal input;
Termleaf does not bind Command or the Windows key. Depending on macOS keyboard
settings, function-key shortcuts may require `Fn` as well—for example,
`Fn+Shift+F5` for line spacing.

On Termux, touch mode is enabled by default. Tap the segmented bottom bar to
open files, save, change input mode, and reach display or sound settings. The
buttons change to relevant actions inside dialogs, such as Previous, Use, and
Close. The final Tools page contains **Touch off**; `Ctrl+T` can turn the bar
back on. Disabling it also restores normal terminal mouse selection behavior.

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

## Language packs

English and its Latin big-text glyphs are always available. Manage the
data-only Korean and Japanese packs from `F9` or the command line:

```bash
termleaf language list
termleaf language install ko --use
termleaf language install ja --use
termleaf language use en
termleaf language remove ja
```

Language packs are stored under `${XDG_DATA_HOME:-~/.local/share}/termleaf/languages`.
They contain a manifest, Galmuri glyph data, licenses, and optional language
models—never executable plugin code. Documents continue to accept Unicode input
from the operating system regardless of the selected interface language.
Japanese packs installed before contextual conversion are shown as **Update
needed** in `F9`; select Japanese and press `Enter` to replace the old pack.

## Typing Korean and Japanese

By default the status bar shows `IME:OS`; Termleaf accepts text committed by
the operating-system input method. Switch input sources with `Control+Space`
on macOS (or a configured Globe/Fn key), `Win+Space` on Windows, or the desktop
shortcut on Linux (`Super+Space` by default in GNOME). The input source must be
installed in the operating system first.

`F2` cycles forward through the installed in-app input modes; `Shift+F2`
cycles in reverse. With both packs installed the order is `IME:OS → IME:KO →
IME:JA → IME:OS`, and unavailable languages are skipped. This is separate
from the operating system's input-language switch.

In `IME:KO`, Termleaf maps raw English-layout keys using standard
**두벌식**, e.g. `g k s` → **한**. The big zone updates one character slot in
place as `ㅎ` → `하` → `한`, and `Backspace` disassembles that same cluster one
step at a time. Set the OS keyboard to English while using `IME:KO`, since
Termleaf needs the raw Latin key events to expose each intermediate step.

In `IME:JA`, common romaji sequences become Hiragana inside Termleaf
(`konnichiha` → `こんにちは`). Press `Space` or `Tab`
to start contextual kana-to-kanji conversion, `Shift+Tab` to move to the
previous candidate, and `Left`/`Right` to select another clause. `Enter`
confirms the conversion; `Esc` returns to raw kana without committing. Press
`Ctrl+K` to switch the unconfirmed kana view between Hiragana (`IME:JAあ`) and
Katakana (`IME:JAア`).

The Japanese language pack includes Akaza's offline unigram, bigram, and
skip-bigram model, so whole-sentence conversion considers neighboring words
without sending text to a server or starting a separate IME process. The pack
is roughly 150 MB to download and about 320 MB after installation.

You can also open a document directly from the shell:

```bash
termleaf memo.txt
```

An existing path is loaded; a missing path becomes a new document that will be
created there on save.

## Configuration

Settings live at `~/.config/termleaf/config` (a simple `key = value` file), written
on exit and editable by hand:

```
live_composition = false
live_japanese = false
japanese_katakana = false
focus_mode = false
sound = true
backspace_sound = true
return_sound = true
sound_profile = classic
big_font = true
font_size = 2
line_spacing = 2
page_width = false
# theme: paper, night, xt, or amber
theme = paper
# language: en, ko, or ja (the matching optional pack must be installed)
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

## v0.2 scope notes

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
