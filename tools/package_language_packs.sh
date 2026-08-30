#!/bin/sh
# Build version-compatible, data-only language pack release artifacts.

set -eu

AKAZA_MODEL_URL="https://github.com/akaza-im/akaza/releases/download/v2026.602.0/akaza-default-model.tar.gz"
AKAZA_MODEL_SHA256="fdbfa1040c9a1d33af725a0ae90ba70babb1a3145978b5db1600e1f7b6f1fa34"
temporary_dir=$(mktemp -d "${TMPDIR:-/tmp}/termleaf-language-packs.XXXXXX")
trap 'rm -rf -- "$temporary_dir"' EXIT HUP INT TERM

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{ print $1 }'
    else
        shasum -a 256 "$1" | awk '{ print $1 }'
    fi
}

for language in ko ja; do
    root="termleaf-language-$language"
    archive="$root.tar.xz"
    staging="target/language-packs/$root"

    rm -rf "$staging"
    mkdir -p "$staging"
    cp -R "language-packs/$language/." "$staging/"
    cp assets/OFL-1.1.txt "$staging/LICENSE"
    if [ "$language" = "ja" ]; then
        model_archive="$temporary_dir/akaza-default-model.tar.gz"
        if [ -n "${TERMLEAF_AKAZA_MODEL_ARCHIVE:-}" ]; then
            cp "$TERMLEAF_AKAZA_MODEL_ARCHIVE" "$model_archive"
        else
            curl --proto '=https' --tlsv1.2 -LsSf "$AKAZA_MODEL_URL" -o "$model_archive"
        fi
        actual_model_sha=$(sha256 "$model_archive")
        [ "$actual_model_sha" = "$AKAZA_MODEL_SHA256" ] || {
            printf '%s\n' "Akaza model checksum verification failed" >&2
            exit 1
        }
        tar -xzf "$model_archive" -C "$staging"
        cache_home="$temporary_dir/cache-home"
        XDG_CACHE_HOME="$cache_home" cargo run --quiet -- \
            __build-japanese-cache "$staging/akaza-default-model"
        mkdir -p "$staging/akaza-default-model/prebuilt-cache"
        cp "$cache_home/akaza/"*.marisa \
            "$staging/akaza-default-model/prebuilt-cache/"
    fi
    tar -C target/language-packs -cJf "$archive" "$root"

    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$archive" >"$archive.sha256"
    else
        shasum -a 256 "$archive" >"$archive.sha256"
    fi
done
