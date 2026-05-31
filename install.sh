#!/usr/bin/env bash
# Dipralix installer — downloads the prebuilt binary for your OS + arch
# from GitHub Releases and installs it to /usr/local/bin (or ~/.local/bin
# if /usr/local/bin isn't writable without sudo).
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/pratikacharya1234/dipralix/main/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/pratikacharya1234/dipralix/main/install.sh | bash -s -- --version v0.1.0
#
# Falls back to building from source via cargo if your platform isn't
# available as a prebuilt binary (e.g. linux-aarch64, freebsd).

set -euo pipefail

REPO="pratikacharya1234/dipralix"
VERSION="latest"
BIN_NAME="dipralix-cli"

while [ $# -gt 0 ]; do
  case "$1" in
    --version) VERSION="$2"; shift 2 ;;
    --version=*) VERSION="${1#*=}"; shift ;;
    *) echo "Unknown option: $1"; exit 1 ;;
  esac
done

# ── Detect OS + arch ──────────────────────────────────────────────────────
OS_RAW="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH_RAW="$(uname -m)"

case "$OS_RAW" in
  darwin) OS="macos" ;;
  linux)  OS="linux" ;;
  *) echo "Unsupported OS: $OS_RAW (only macOS and Linux supported here; use install.ps1 for Windows)"; exit 1 ;;
esac

case "$ARCH_RAW" in
  x86_64|amd64) ARCH="x86_64" ;;
  arm64|aarch64) ARCH="arm64" ;;
  *) echo "Unsupported architecture: $ARCH_RAW"; exit 1 ;;
esac

# Map (OS, ARCH) → asset suffix used by release-binaries.yml
case "$OS-$ARCH" in
  macos-arm64)   ASSET="macos-arm64" ;;
  macos-x86_64)  ASSET="macos-x86_64" ;;
  linux-x86_64)  ASSET="linux-x86_64" ;;
  *)
    echo "No prebuilt binary for $OS-$ARCH yet."
    echo "Falling back to cargo install (requires Rust toolchain)."
    if ! command -v cargo >/dev/null 2>&1; then
      echo "Rust toolchain not found. Install: https://rustup.rs"
      exit 1
    fi
    cargo install --locked --git "https://github.com/$REPO" --bin "$BIN_NAME"
    exit 0
    ;;
esac

# ── Resolve version ───────────────────────────────────────────────────────
if [ "$VERSION" = "latest" ]; then
  echo "Resolving latest release tag…"
  VERSION="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')"
  if [ -z "$VERSION" ]; then
    echo "Could not resolve latest release tag. Specify --version v0.1.0"
    exit 1
  fi
fi

ARCHIVE="dipralix-${VERSION}-${ASSET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARCHIVE}"

echo "Installing Dipralix ${VERSION} for ${OS}-${ARCH}…"
echo "  → ${URL}"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

curl -fsSL "$URL" -o "$TMP/$ARCHIVE"
tar -xzf "$TMP/$ARCHIVE" -C "$TMP"

# ── Pick install dir ──────────────────────────────────────────────────────
if [ -w "/usr/local/bin" ]; then
  DEST="/usr/local/bin"
  SUDO=""
elif command -v sudo >/dev/null 2>&1 && [ -d "/usr/local/bin" ]; then
  DEST="/usr/local/bin"
  SUDO="sudo"
else
  DEST="$HOME/.local/bin"
  mkdir -p "$DEST"
  SUDO=""
fi

$SUDO install -m 0755 "$TMP/$BIN_NAME" "$DEST/$BIN_NAME"

# ── PATH hint ─────────────────────────────────────────────────────────────
if ! command -v "$BIN_NAME" >/dev/null 2>&1; then
  case ":$PATH:" in
    *":$DEST:"*) ;;
    *) echo
       echo "Add this to your shell rc to put $DEST on PATH:"
       echo "  export PATH=\"$DEST:\$PATH\""
       ;;
  esac
fi

echo
echo "Installed: $DEST/$BIN_NAME"
"$DEST/$BIN_NAME" --version || true
echo
echo "Run 'dipralix-cli' to start. Get a free API key at https://aistudio.google.com/apikey"
