#!/usr/bin/env bash
# DIPRALIX Release Script
set -euo pipefail

VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d '"' -f 2)
echo "Releasing DIPRALIX v$VERSION..."

cargo build --release

# Create packages
mkdir -p packages
tar -czf packages/dipralix-v$VERSION-macos-arm64.tar.gz -C target/release dipralix-cli

echo "Done."
