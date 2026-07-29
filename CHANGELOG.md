# Changelog

All notable changes to Tadak are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and Tadak uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-07-30

### Added

- Curl installations now include `tadak-update` for future in-place updates.

### Changed

- Reworked the carriage-return effect as a connected lever clack, inharmonic
  bell, ratcheting carriage sweep, and damped cabinet stop.

## [0.1.0] - 2026-07-30

### Added

- Distraction-free terminal editor with cursor movement, open, save, save-as,
  autosave, and document selection.
- Five-level large pixel text rendered from the embedded Galmuri9 font.
- Operating-system IME input and optional live two-set Korean composition that
  assembles jamo inside one character cell.
- Paper, night, XT phosphor-green, and amber themes.
- English and Korean interfaces with persistent preferences and built-in help.
- Classic, deep, and soft typewriter key sounds, a gentle rate-limited delete
  effect, and an independently configurable carriage-return bell.
- `tadak [FILE]`, `--help`, and `--version` command-line entry points.
- Prebuilt macOS and Linux release archives with a shared curl installer.

### Fixed

- Prevented idle screen flicker by avoiding unnecessary redraws and using
  synchronized terminal updates.
- Made forward Delete work for characters and line boundaries.
- Made Save As reliable through `F12`, including a Markdown default extension.

[Unreleased]: https://github.com/andy5090/tadak/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/andy5090/tadak/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/andy5090/tadak/releases/tag/v0.1.0
