# Changelog

All notable changes to Termleaf are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and Termleaf uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/andy5090/termleaf/compare/v0.2.2...HEAD
[0.2.2]: https://github.com/andy5090/termleaf/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/andy5090/termleaf/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/andy5090/termleaf/releases/tag/v0.2.0
