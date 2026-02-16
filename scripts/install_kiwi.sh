#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/install_kiwi.sh [version|latest]

Examples:
  scripts/install_kiwi.sh
  scripts/install_kiwi.sh v0.22.2
  scripts/install_kiwi.sh latest

Environment:
  KIWI_PREFIX         Install prefix (default: $HOME/.local/kiwi)
  KIWI_MODEL_VARIANT  Model variant (default: base)
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

VERSION="${1:-latest}"
DEFAULT_PREFIX="${HOME:-/usr/local}/.local/kiwi"
if [[ -z "${HOME:-}" ]]; then
  DEFAULT_PREFIX="/usr/local"
fi
PREFIX="${KIWI_PREFIX:-$DEFAULT_PREFIX}"
MODEL_VARIANT="${KIWI_MODEL_VARIANT:-base}"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "[kiwi-rs] Missing required command: $1" >&2
    exit 1
  fi
}

need_cmd curl
need_cmd tar

resolve_tag() {
  local version="$1"
  if [[ "$version" != "latest" ]]; then
    if [[ "$version" == v* ]]; then
      echo "$version"
    else
      echo "v$version"
    fi
    return
  fi

  local api_url="https://api.github.com/repos/bab2min/Kiwi/releases/latest"
  local json
  json="$(curl -fsSL "$api_url")"

  local tag
  tag="$(printf '%s' "$json" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)"
  if [[ -z "$tag" ]]; then
    echo "[kiwi-rs] Could not resolve latest tag from GitHub API." >&2
    exit 1
  fi
  echo "$tag"
}

prefix_is_writable() {
  local probe="$1"
  while [[ ! -e "$probe" ]]; do
    local parent
    parent="$(dirname "$probe")"
    if [[ "$parent" == "$probe" ]]; then
      break
    fi
    probe="$parent"
  done

  [[ -w "$probe" ]]
}

run_install_cmd() {
  if prefix_is_writable "$PREFIX"; then
    "$@"
    return
  fi

  if ! command -v sudo >/dev/null 2>&1; then
    echo "[kiwi-rs] $PREFIX is not writable and sudo is unavailable." >&2
    exit 1
  fi
  sudo "$@"
}

UNAME_S="$(uname -s)"
UNAME_M="$(uname -m)"

case "$UNAME_S" in
  Linux)
    OS="lnx"
    ;;
  Darwin)
    OS="mac"
    ;;
  *)
    echo "[kiwi-rs] Unsupported OS for helper installer: $UNAME_S" >&2
    exit 1
    ;;
esac

if [[ "$OS" == "mac" ]]; then
  case "$UNAME_M" in
    arm64|aarch64) ARCH="arm64" ;;
    x86_64) ARCH="x86_64" ;;
    *)
      echo "[kiwi-rs] Unsupported macOS arch: $UNAME_M" >&2
      exit 1
      ;;
  esac
else
  case "$UNAME_M" in
    x86_64|amd64) ARCH="x86_64" ;;
    arm64|aarch64) ARCH="aarch64" ;;
    ppc64le|powerpc64le) ARCH="ppc64le" ;;
    *)
      echo "[kiwi-rs] Unsupported Linux arch: $UNAME_M" >&2
      exit 1
      ;;
  esac
fi

TAG="$(resolve_tag "$VERSION")"
VERSION_NO_V="${TAG#v}"

LIB_ASSET="kiwi_${OS}_${ARCH}_v${VERSION_NO_V}.tgz"
MODEL_ASSET="kiwi_model_v${VERSION_NO_V}_${MODEL_VARIANT}.tgz"
BASE_URL="https://github.com/bab2min/Kiwi/releases/download/${TAG}"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

LIB_ARCHIVE="${TMP_DIR}/${LIB_ASSET}"
MODEL_ARCHIVE="${TMP_DIR}/${MODEL_ASSET}"

echo "[kiwi-rs] OS=${OS}, ARCH=${ARCH}, TAG=${TAG}"
echo "[kiwi-rs] Downloading ${LIB_ASSET}"
curl -fL --retry 3 --retry-delay 1 -o "$LIB_ARCHIVE" "${BASE_URL}/${LIB_ASSET}"

echo "[kiwi-rs] Downloading ${MODEL_ASSET}"
curl -fL --retry 3 --retry-delay 1 -o "$MODEL_ARCHIVE" "${BASE_URL}/${MODEL_ASSET}"

echo "[kiwi-rs] Ensuring install prefix ${PREFIX}"
run_install_cmd mkdir -p "$PREFIX"

echo "[kiwi-rs] Extracting library archive to ${PREFIX}"
run_install_cmd tar -xzf "$LIB_ARCHIVE" -C "$PREFIX"

echo "[kiwi-rs] Extracting model archive to ${PREFIX}"
run_install_cmd tar -xzf "$MODEL_ARCHIVE" -C "$PREFIX"

if [[ "$OS" == "lnx" && -x "$(command -v ldconfig || true)" && "$PREFIX" == /usr/* ]]; then
  echo "[kiwi-rs] Running ldconfig"
  run_install_cmd ldconfig || true
fi

if [[ "$OS" == "mac" ]]; then
  LIB_PATH="${PREFIX}/lib/libkiwi.dylib"
else
  LIB_PATH="${PREFIX}/lib/libkiwi.so"
fi
MODEL_PATH="${PREFIX}/models/cong/${MODEL_VARIANT}"

echo
echo "[kiwi-rs] Install done."
echo "[kiwi-rs] Library path: ${LIB_PATH}"
echo "[kiwi-rs] Model path:   ${MODEL_PATH}"
echo
echo "[kiwi-rs] Optional env setup:"
echo "  export KIWI_LIBRARY_PATH='${LIB_PATH}'"
echo "  export KIWI_MODEL_PATH='${MODEL_PATH}'"
