#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
INSTALLER="${ROOT_DIR}/install.sh"
KEEP_TMP="${LOOM_SMOKE_KEEP_TMP:-0}"
INSTALL_DIR_OVERRIDE="${LOOM_SMOKE_INSTALL_DIR:-}"

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: required command '$1' not found" >&2
    exit 1
  fi
}

detect_target() {
  case "$(uname -s | tr '[:upper:]' '[:lower:]')/$(uname -m | tr '[:upper:]' '[:lower:]')" in
    darwin/arm64|darwin/aarch64)
      target_triple="aarch64-apple-darwin"
      binary_path="${ROOT_DIR}/target/release/loom"
      ;;
    linux/x86_64|linux/amd64)
      target_triple="x86_64-unknown-linux-musl"
      binary_path="${ROOT_DIR}/target/release/loom"
      ;;
    linux/aarch64|linux/arm64)
      target_triple="aarch64-unknown-linux-musl"
      binary_path="${ROOT_DIR}/target/release/loom"
      ;;
    *)
      echo "error: smoke installer script supports only aarch64-apple-darwin, x86_64-unknown-linux-musl, and aarch64-unknown-linux-musl hosts" >&2
      exit 1
      ;;
  esac
}

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    echo "error: no SHA256 tool found (need sha256sum or shasum)" >&2
    exit 1
  fi
}

require_cmd cargo
require_cmd tar

detect_target

(cd "${ROOT_DIR}" && cargo build --release --locked --bin loom)

built_version="$("${binary_path}" --version | awk '{print $2}')"
if [[ -z "${built_version}" ]]; then
  echo "error: failed to read version from ${binary_path}" >&2
  exit 1
fi

version="v${built_version#v}"
built_sha="$(sha256_of "${binary_path}")"
archive_name="loom-${target_triple}.tar.gz"

tmp="$(mktemp -d)"

cleanup() {
  if [[ "${KEEP_TMP}" == "1" ]]; then
    echo "Retained smoke test artifacts at ${tmp}"
  else
    rm -rf "${tmp}"
  fi
}

trap cleanup EXIT

cp "${binary_path}" "${tmp}/loom"
tar -czf "${tmp}/${archive_name}" -C "${tmp}" loom -C "${ROOT_DIR}" README.md LICENSE

for path in "${tmp}/local/loom/releases/latest/download" "${tmp}/local/loom/releases/download/${version}"; do
  mkdir -p "${path}"
  cp "${tmp}/${archive_name}" "${path}/${archive_name}"
  (
    cd "${path}"
    if command -v sha256sum >/dev/null 2>&1; then
      sha256sum "${archive_name}" > loom-checksums.txt
    else
      shasum -a 256 "${archive_name}" > loom-checksums.txt
    fi
  )
done

install_dir="${tmp}/install"
if [[ -n "${INSTALL_DIR_OVERRIDE}" ]]; then
  install_dir="${INSTALL_DIR_OVERRIDE}"
fi
mkdir -p "${install_dir}"

download_base="file://${tmp}"

LOOM_GITHUB_REPO="local/loom" \
LOOM_INSTALL_DIR="${install_dir}" \
LOOM_RELEASE_DOWNLOAD_BASE="${download_base}" \
  "${INSTALLER}"

LOOM_GITHUB_REPO="local/loom" \
LOOM_INSTALL_DIR="${install_dir}" \
LOOM_RELEASE_DOWNLOAD_BASE="${download_base}" \
LOOM_VERSION="${version}" \
  "${INSTALLER}"

if [[ ! -x "${install_dir}/loom" ]]; then
  echo "error: loom binary was not installed" >&2
  exit 1
fi

installed_version="$("${install_dir}/loom" --version | awk '{print $2}')"
if [[ "${installed_version}" != "${built_version}" ]]; then
  echo "error: installed version ${installed_version} != built version ${built_version}" >&2
  exit 1
fi

installed_sha="$(sha256_of "${install_dir}/loom")"
if [[ "${installed_sha}" != "${built_sha}" ]]; then
  echo "error: installed binary hash does not match local release build" >&2
  exit 1
fi

echo "Installer smoke test passed for ${version} (${target_triple})"
echo "Installed binary matches local release build at ${install_dir}/loom"
