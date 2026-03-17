#!/bin/sh

set -eu

OWNER="${LOOM_INSTALL_OWNER:-acartine}"
REPO="${LOOM_INSTALL_REPO:-loom}"
BIN_NAME="loom"
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"
VERSION="${LOOM_VERSION:-latest}"

need_cmd() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "error: missing required command: $1" >&2
        exit 1
    }
}

detect_target() {
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux) os_part="unknown-linux-musl" ;;
        Darwin) os_part="apple-darwin" ;;
        *)
            echo "error: unsupported operating system: $os" >&2
            exit 1
            ;;
    esac

    case "$arch" in
        x86_64|amd64) arch_part="x86_64" ;;
        arm64|aarch64) arch_part="aarch64" ;;
        *)
            echo "error: unsupported architecture: $arch" >&2
            exit 1
            ;;
    esac

    printf "%s-%s" "$arch_part" "$os_part"
}

download_url() {
    target="$1"

    if [ "$VERSION" = "latest" ]; then
        printf "https://github.com/%s/%s/releases/latest/download/loom-%s.tar.gz" "$OWNER" "$REPO" "$target"
    else
        printf "https://github.com/%s/%s/releases/download/%s/loom-%s.tar.gz" "$OWNER" "$REPO" "$VERSION" "$target"
    fi
}

checksums_url() {
    if [ "$VERSION" = "latest" ]; then
        printf "https://github.com/%s/%s/releases/latest/download/loom-checksums.txt" "$OWNER" "$REPO"
    else
        printf "https://github.com/%s/%s/releases/download/%s/loom-checksums.txt" "$OWNER" "$REPO" "$VERSION"
    fi
}

verify_checksum() {
    archive_path="$1"
    target_archive="$2"
    checksums_path="$3"

    if command -v sha256sum >/dev/null 2>&1; then
        (cd "$(dirname "$archive_path")" && grep " ${target_archive}\$" "$checksums_path" | sha256sum -c -)
    elif command -v shasum >/dev/null 2>&1; then
        expected="$(grep " ${target_archive}\$" "$checksums_path" | awk '{print $1}')"
        actual="$(shasum -a 256 "$archive_path" | awk '{print $1}')"
        [ "$expected" = "$actual" ] || {
            echo "error: checksum verification failed for ${target_archive}" >&2
            exit 1
        }
    else
        echo "warning: skipping checksum verification because neither sha256sum nor shasum is available" >&2
    fi
}

need_cmd curl
need_cmd tar
need_cmd install

target="$(detect_target)"
archive_name="loom-${target}.tar.gz"
archive_url="$(download_url "$target")"
checksum_download_url="$(checksums_url)"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT INT TERM

archive_path="${tmpdir}/${archive_name}"
checksums_path="${tmpdir}/loom-checksums.txt"

echo "Downloading ${archive_url}" >&2
curl -fsSL "$archive_url" -o "$archive_path"

echo "Downloading ${checksum_download_url}" >&2
curl -fsSL "$checksum_download_url" -o "$checksums_path"
verify_checksum "$archive_path" "$archive_name" "$checksums_path"

mkdir -p "$BIN_DIR"
tar -xzf "$archive_path" -C "$tmpdir"
install -m 0755 "${tmpdir}/${BIN_NAME}" "${BIN_DIR}/${BIN_NAME}"

echo "Installed ${BIN_NAME} to ${BIN_DIR}/${BIN_NAME}" >&2
echo "Add ${BIN_DIR} to PATH if needed." >&2
