# Third-party assets

## Galmuri9 2.40.4

Termleaf includes a converted English core and optional Korean/Japanese glyph
packs. Together they cover printable ASCII, Hangul Compatibility Jamo, all
11,172 precomposed Hangul syllables, kana, fullwidth forms, CJK punctuation,
and the CJK ideographs available in Galmuri9.

Copyright © 2019–2025 Lee Minseo (quiple@quiple.dev)

The font is distributed under the SIL Open Font License, Version 1.1. The full
license text is in [`assets/OFL-1.1.txt`](assets/OFL-1.1.txt).

- Project: <https://quiple.dev/font/galmuri>
- Source: <https://github.com/quiple/galmuri/blob/v2.40.4/dist/Galmuri9.bdf>

## Hermes Precisa 305 typewriter recordings

Termleaf's key, delete, and carriage-return effects are edited from recordings
of a Hermes Precisa 305 made by Joseph SARDIN. The optimized 44.1 kHz, 16-bit
mono PCM files embedded in Termleaf are derived from these 48 kHz, 24-bit
masters:

- Slow typewriter sequence, sound 2841: <https://bigsoundbank.com/machine-a-ecrire-8-s2841.html>
- Typewriter space, sound 2843: <https://bigsoundbank.com/machine-a-ecrire-espace-s2843.html>
- Typewriter bell, sound 2844: <https://bigsoundbank.com/typewriter-bell-s2844.html>

The recordings are dedicated to the public domain under the Creative Commons
CC0 1.0 Universal license. Attribution is not required, but is included here in
appreciation of the author's work.

- Author: Joseph SARDIN
- Library: <https://bigsoundbank.com>
- License: <https://creativecommons.org/publicdomain/zero/1.0/>
- Library license terms: <https://bigsoundbank.com/licenses.html>

## Akaza Japanese conversion engine and model

Termleaf uses the Akaza kana-to-kanji engine pinned to v2026.602.0. Akaza is
Copyright © 2023 Tokuhiro Matsuno and is distributed under the MIT License.

Akaza uses `rsmarisa` 0.4.2, Copyright © 2024 Tokuhiro Matsuno, under the
BSD 2-Clause License. Termleaf vendors its Rust source with a bounds-safe
prefix-search fix; the license is retained at `vendor/rsmarisa/LICENSE`.

The optional Japanese language pack includes Akaza's default unigram, bigram,
skip-bigram, and system-dictionary data. The model is built from open language
resources including Japanese Wikipedia, Aozora Bunko public-domain works, and
CC-100 Japanese/Common Crawl. Because the generated model contains statistics
derived from Japanese Wikipedia, its redistribution is subject to CC BY-SA
4.0 and attribution to Wikimedia Foundation and Japanese Wikipedia
contributors.

- Project and model source: <https://github.com/akaza-im/akaza/tree/v2026.602.0>
- Model release: <https://github.com/akaza-im/akaza/releases/tag/v2026.602.0>
- CC BY-SA 4.0: <https://creativecommons.org/licenses/by-sa/4.0/>
- Pack-local notices: `language-packs/ja/LICENSE-AKAZA.txt` and
  `language-packs/ja/NOTICE-AKAZA-MODEL.txt`
