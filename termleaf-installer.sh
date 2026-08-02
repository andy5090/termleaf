#!/bin/sh
# Install the latest Termleaf release on supported macOS and Linux systems.

set -eu

quiet=0
modify_path=1

say() {
    if [ "$quiet" -eq 0 ]; then
        printf '%s\n' "$*"
    fi
}

die() {
    printf 'termleaf-installer: %s\n' "$*" >&2
    exit 1
}

usage() {
    cat <<'EOF'
Usage: termleaf-installer.sh [OPTIONS]

Options:
  --quiet           Suppress informational output
  --no-modify-path  Do not update the shell PATH
  -h, --help        Show this help

Environment:
  CARGO_HOME             Installation root (default: $HOME/.cargo)
  TERMLEAF_VERSION       Release tag or version to install (default: latest)
  TERMLEAF_DOWNLOAD_URL  Override the release asset base URL
EOF
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --quiet)
            quiet=1
            ;;
        --no-modify-path)
            modify_path=0
            ;;
        -h | --help)
            usage
            exit 0
            ;;
        *)
            die "unknown option: $1"
            ;;
    esac
    shift
done

for command_name in curl uname tar mktemp awk; do
    command -v "$command_name" >/dev/null 2>&1 ||
        die "required command not found: $command_name"
done

os_name=$(uname -s)
cpu_name=$(uname -m)

case "$os_name" in
    Darwin)
        case "$cpu_name" in
            arm64 | aarch64)
                target="aarch64-apple-darwin"
                ;;
            x86_64 | amd64)
                target="x86_64-apple-darwin"
                ;;
            *)
                die "unsupported macOS architecture: $cpu_name"
                ;;
        esac
        ;;
    Linux)
        bitness=$(getconf LONG_BIT 2>/dev/null || printf '')
        case "$cpu_name" in
            i386 | i486 | i586 | i686 | i786 | x86)
                target="i686-unknown-linux-gnu"
                ;;
            x86_64 | x86-64 | x64 | amd64)
                if [ "$bitness" = "32" ]; then
                    target="i686-unknown-linux-gnu"
                else
                    target="x86_64-unknown-linux-gnu"
                fi
                ;;
            *)
                die "unsupported Linux architecture: $cpu_name"
                ;;
        esac

        if ldd --version 2>&1 | grep -q 'musl'; then
            die "musl Linux is not supported by the prebuilt installer"
        fi
        ;;
    *)
        die "unsupported operating system: $os_name"
        ;;
esac

release_tag=${TERMLEAF_VERSION:-}
if [ -z "$release_tag" ]; then
    latest_url="https://github.com/andy5090/termleaf/releases/latest"
    effective_url=$(
        curl --proto '=https' --tlsv1.2 -LsSf \
            -o /dev/null -w '%{url_effective}' "$latest_url"
    )
    release_tag=${effective_url##*/}
else
    case "$release_tag" in
        v*) ;;
        *) release_tag="v$release_tag" ;;
    esac
fi

case "$release_tag" in
    v[0-9]*.[0-9]*.[0-9]*) ;;
    *) die "could not determine a valid release tag" ;;
esac

if [ -n "${TERMLEAF_DOWNLOAD_URL:-}" ]; then
    download_base=${TERMLEAF_DOWNLOAD_URL%/}
else
    download_base="https://github.com/andy5090/termleaf/releases/download/$release_tag"
fi

archive_name="termleaf-$target.tar.xz"
checksum_name="$archive_name.sha256"
temporary_dir=$(mktemp -d "${TMPDIR:-/tmp}/termleaf-installer.XXXXXX")

cleanup() {
    rm -rf -- "$temporary_dir"
}
trap cleanup 0 1 2 15

say "Installing Termleaf ${release_tag#v} for $target..."

curl --proto '=https' --tlsv1.2 -LsSf \
    "$download_base/$archive_name" -o "$temporary_dir/$archive_name"
curl --proto '=https' --tlsv1.2 -LsSf \
    "$download_base/$checksum_name" -o "$temporary_dir/$checksum_name"

expected_checksum=$(awk 'NR == 1 { print $1 }' "$temporary_dir/$checksum_name")
[ -n "$expected_checksum" ] || die "release checksum is empty"

if command -v sha256sum >/dev/null 2>&1; then
    actual_checksum=$(sha256sum "$temporary_dir/$archive_name" | awk '{ print $1 }')
elif command -v shasum >/dev/null 2>&1; then
    actual_checksum=$(shasum -a 256 "$temporary_dir/$archive_name" | awk '{ print $1 }')
else
    die "sha256sum or shasum is required to verify the download"
fi

[ "$actual_checksum" = "$expected_checksum" ] ||
    die "release checksum verification failed"

tar -xJf "$temporary_dir/$archive_name" -C "$temporary_dir"
archive_root="$temporary_dir/${archive_name%.tar.xz}"

[ -f "$archive_root/termleaf" ] || die "release archive is missing termleaf"

if [ -n "${CARGO_HOME:-}" ]; then
    cargo_home=${CARGO_HOME%/}
else
    [ -n "${HOME:-}" ] || die "HOME is not set; set CARGO_HOME explicitly"
    cargo_home="$HOME/.cargo"
fi

bin_dir="$cargo_home/bin"
mkdir -p "$bin_dir"

staging_path="$bin_dir/.termleaf.termleaf-install.$$"
cp "$archive_root/termleaf" "$staging_path"
chmod 755 "$staging_path"
mv -f "$staging_path" "$bin_dir/termleaf"

path_changed=0
case ":${PATH:-}:" in
    *":$bin_dir:"*) ;;
    *)
        if [ "$modify_path" -eq 1 ] &&
            [ -n "${HOME:-}" ] &&
            [ "$cargo_home" = "$HOME/.cargo" ]; then
            env_file="$HOME/.cargo/env"
            if [ ! -f "$env_file" ]; then
                cat >"$env_file" <<'EOF'
#!/bin/sh
case ":${PATH}:" in
    *:"$HOME/.cargo/bin":*) ;;
    *) export PATH="$HOME/.cargo/bin:$PATH" ;;
esac
EOF
            fi

            case "${SHELL:-}" in
                */zsh) shell_profile="$HOME/.zshrc" ;;
                */bash) shell_profile="$HOME/.bashrc" ;;
                *) shell_profile="$HOME/.profile" ;;
            esac

            # Keep HOME literal so the profile remains portable.
            # shellcheck disable=SC2016
            source_line='. "$HOME/.cargo/env"'
            if [ ! -f "$shell_profile" ] ||
                ! grep -F "$source_line" "$shell_profile" >/dev/null 2>&1; then
                printf '\n%s\n' "$source_line" >>"$shell_profile"
            fi
            path_changed=1
        fi
        ;;
esac

say "Installed termleaf in $bin_dir"
if [ "$path_changed" -eq 1 ]; then
    say "Open a new terminal before running termleaf."
elif ! command -v termleaf >/dev/null 2>&1; then
    say "Add $bin_dir to PATH before running termleaf."
fi
