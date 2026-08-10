# Changelog

All notable changes to Termleaf are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and Termleaf uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.6] - 2026-08-10

### Added

- Launched the English-first, Korean-localized Termleaf website at
  [termleaf.com](https://termleaf.com) with an actual macOS Terminal capture.

### Changed

- Consolidated self-updates into `termleaf update [--force]` and removed the
  separate `termleaf-update` executable from release archives and the installer.

### Fixed

- Kept enlarged pixel text legible when `NO_COLOR` disables terminal colors by
  rendering unlit pixels as empty cells instead of indistinguishable blocks.

## [0.3.5] - 2026-08-02

### Fixed

- Restored typewriter audio on i686 Linux by backporting CPAL's 32-bit-safe
  ALSA timestamp conversion, letting ALSA choose a device-compatible buffer
  size, and rebuilding streams after backend failures.

## [0.3.4] - 2026-08-02

### Changed

- Added a one-row top margin above the document when big text is disabled,
  while preserving all available editing rows in very small terminals.
- Added `Shift+F5` as a platform-neutral line-spacing shortcut while retaining
  macOS `Option+L` and Windows/Linux `Alt+L` as terminal-dependent alternates.

## [0.3.3] - 2026-08-01

### Changed

- Consolidated typing-sound controls in the F10 panel, renamed the master
  toggle to "Typing sound," reassigned F5 to the macOS-safe page-width toggle,
  and made the footer shortcuts adapt to narrow terminals.

## [0.3.2] - 2026-08-01

### Fixed

- Replaced the synthetic typewriter effects with compact, edited CC0 field
  recordings of a Hermes Precisa 305 for more natural playback, while removing
  the low-frequency resonance that sounded like a drum on MacBook speakers.
- Added four real key-strike variations per sound profile and avoided immediate
  repeats so sustained typing no longer sounds like one identical sample loop.
- Added restrained per-strike pitch, level, brightness, and decay variation to
  make the recorded key differences perceptible without sounding randomized.

## [0.3.1] - 2026-08-01

### Changed

- Replaced horizontal scrolling for long document lines with non-destructive
  soft wrapping at the current screen or page-width boundary.

## [0.3.0] - 2026-07-31

### Added

- Added three persisted document line-spacing levels, with a relaxed
  one-blank-row gap as the default and `Alt+L` to cycle them.
- Added an optional centered 80-column page-width mode toggled with `Alt+P`.

### Changed

- Made the large-text zone display as many cursor-side characters as the
  current terminal width and selected scale can fit instead of stopping at a
  fixed 12-character limit.

## [0.2.4] - 2026-07-31

### Fixed

- Kept i686 audio alive after recoverable ALSA backend errors instead of
  permanently disabling sound on the next keypress.
- Used a stability-focused 4096-frame audio buffer on 32-bit x86 Linux and
  automatically reopened streams after an actual device loss.

## [0.2.3] - 2026-07-31

### Changed

- Made updater network failures actionable by explaining DNS, proxy,
  connectivity, timeout, HTTP, and TLS errors while preserving curl's original
  diagnostic.

## [0.2.2] - 2026-07-31

### Fixed

- Prevented ALSA `POLLERR` failures on i686 and older Linux audio devices by
  using Rodio's device-safe buffer size instead of forcing a 512-frame buffer.
- Kept audio backend failures out of the terminal UI and continued editing
  with sound disabled when a playback stream becomes unusable.

## [0.2.1] - 2026-07-31

### Added

- Prebuilt `i686-unknown-linux-gnu` archives for 32-bit x86 Linux.
- Architecture detection and checksum-verified i686 installation through the
  common curl installer and `termleaf-update`.

## [0.2.0] - 2026-07-30

### Added

- Distraction-free terminal editor with cursor movement, open, save, save-as,
  autosave, document selection, and a chrome-free focus mode.
- Five-level large pixel text rendered from the embedded Galmuri9 font.
- Operating-system IME input and optional live two-set Korean composition that
  assembles jamo inside one character cell.
- Paper, night, XT phosphor-green, and amber themes.
- English and Korean interfaces with persistent preferences and built-in help.
- Optional classic, deep, and soft key sounds with independent delete and
  return effects.
- `termleaf [FILE]`, `--help`, and `--version` command-line entry points.
- `termleaf-update` with actual executable-version checks and a `--force`
  repair option.
- Prebuilt macOS and Linux release archives with a shared curl installer.

### Fixed

- Prevented idle screen flicker by avoiding unnecessary redraws and using
  synchronized terminal updates.
- Made forward Delete work for characters and line boundaries.
- Made Save As reliable through `F12`, including a Markdown default extension.

[Unreleased]: https://github.com/andy5090/termleaf/compare/v0.3.6...HEAD
[0.3.6]: https://github.com/andy5090/termleaf/compare/v0.3.5...v0.3.6
[0.3.5]: https://github.com/andy5090/termleaf/compare/v0.3.4...v0.3.5
[0.3.4]: https://github.com/andy5090/termleaf/compare/v0.3.3...v0.3.4
[0.3.3]: https://github.com/andy5090/termleaf/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/andy5090/termleaf/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/andy5090/termleaf/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/andy5090/termleaf/compare/v0.2.4...v0.3.0
[0.2.4]: https://github.com/andy5090/termleaf/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/andy5090/termleaf/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/andy5090/termleaf/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/andy5090/termleaf/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/andy5090/termleaf/releases/tag/v0.2.0
