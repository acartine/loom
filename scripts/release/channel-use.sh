#!/usr/bin/env bash
set -euo pipefail

CHANNEL_ROOT="${LOOM_CHANNEL_ROOT:-${HOME}/.local/bin/acartine_loom}"
ACTIVE_LINK="${LOOM_ACTIVE_LINK:-${HOME}/.local/bin/loom}"

usage() {
  cat <<'USAGE'
Select active loom binary by symlink.

Usage:
  channel-use.sh release
  channel-use.sh local
  channel-use.sh show

Default paths:
  release binary: ~/.local/bin/acartine_loom/release/loom
  local binary:   ~/.local/bin/acartine_loom/local/loom
  active link:    ~/.local/bin/loom

Optional env vars:
  LOOM_CHANNEL_ROOT  Override channel root directory.
  LOOM_ACTIVE_LINK   Override active loom link path.
USAGE
}

show_active() {
  if [[ ! -e "${ACTIVE_LINK}" ]]; then
    echo "No active loom link found"
    return 0
  fi

  resolved="<not-a-symlink>"
  if [[ -L "${ACTIVE_LINK}" ]]; then
    resolved="$(readlink "${ACTIVE_LINK}")"
  fi

  echo "Active loom link: ${ACTIVE_LINK}"
  echo "Resolved target: ${resolved}"
  if [[ -x "${ACTIVE_LINK}" ]]; then
    "${ACTIVE_LINK}" --version
  fi
}

channel="${1:-}"
if [[ -z "${channel}" || "${channel}" == "--help" || "${channel}" == "-h" ]]; then
  usage
  exit 0
fi

if [[ "${channel}" == "show" ]]; then
  show_active
  exit 0
fi

case "${channel}" in
  release|local)
    target="${CHANNEL_ROOT}/${channel}/loom"
    ;;
  *)
    echo "error: unsupported channel '${channel}' (use release|local|show)" >&2
    usage
    exit 1
    ;;
esac

if [[ ! -x "${target}" ]]; then
  echo "error: channel binary not found at ${target}" >&2
  echo "hint: run scripts/release/channel-install.sh ${channel}" >&2
  exit 1
fi

mkdir -p "$(dirname "${ACTIVE_LINK}")"
ln -sfn "${target}" "${ACTIVE_LINK}"

echo "Active loom -> ${target}"
"${ACTIVE_LINK}" --version
