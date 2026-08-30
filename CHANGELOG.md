# Changelog

All notable changes to Termleaf are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and Termleaf uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2026-08-30

### Added

- Added `Shift+F2` Live Japanese sentence conversion with offline
  romaji-to-kana input, `Space`/`Tab` candidate cycling, `Enter` confirmation,
  `Esc` cancellation, and a `Ctrl+K` Hiragana/Katakana view toggle.

### Changed

- Clarified the difference between display language and operating-system input
  language in the in-app help and language manager.
- Extended the Japanese language-pack release flow so it bundles Akaza's
  checksum-verified contextual conversion model and its upstream notices.
- Changed `F2` to cycle forward through installed in-app input languages and
  `Shift+F2` to cycle in reverse, skipping languages that are not installed.
- Replaced the website's native language select with an accessible branded
  dropdown supporting keyboard navigation and outside-click dismissal.

### Fixed

- Accepted the legacy `F14` sequence emitted by some macOS terminals for the
  reverse `Shift+F2` input cycle.
- Detects Japanese packs that predate the Akaza model, excludes their unusable
  Live Japanese mode from the input cycle, and allows `F9` to update the pack.

## [0.4.0] - 2026-08-25

### Added

- Added installable Korean and Japanese language packs with localized in-app
  guidance and enlarged Hangul, kana, fullwidth, and CJK glyph coverage.
- Added `termleaf language` commands and an `F9` language manager for listing,
  installing, selecting, and removing language support.
- Added Japanese website localization and short locale-aware installers at
  `termleaf.com/install`, `/install/ko`, and `/install/ja`.

### Changed

- Reduced the built-in enlarged font to the English core; optional data-only
  language packs now carry their own licensed glyph data.

## [0.3.7] - 2026-08-13

### Added

- Published `sitemap.xml` and `robots.txt` so search engines can discover the
  canonical Termleaf website.

### Changed

- Reworked the persistent shortcut guide into compact modifier groups with
  Zellij-inspired key styling while keeping function keys such as `F1` intact.
- Replaced the website's plain `T` mark with the Typewriter Body `>T` mark in
  the header, footer, and favicon assets.

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

[Unreleased]: https://github.com/andy5090/termleaf/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/andy5090/termleaf/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/andy5090/termleaf/compare/v0.3.7...v0.4.0
[0.3.7]: https://github.com/andy5090/termleaf/compare/v0.3.6...v0.3.7
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
