#!/bin/sh
# Build version-compatible, data-only language pack release artifacts.

set -eu

for language in ko ja; do
    root="termleaf-language-$language"
    archive="$root.tar.xz"
    staging="target/language-packs/$root"

    mkdir -p "$staging"
    cp "language-packs/$language/manifest.txt" "$staging/"
    cp "language-packs/$language/glyphs.bin" "$staging/"
    cp assets/OFL-1.1.txt "$staging/LICENSE"
    tar -C target/language-packs -cJf "$archive" "$root"

    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$archive" >"$archive.sha256"
    else
        shasum -a 256 "$archive" >"$archive.sha256"
    fi
done
