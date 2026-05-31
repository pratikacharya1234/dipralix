#!/usr/bin/env bash
# DIPRALIX Installer
set -euo pipefail

echo "Installing DIPRALIX..."

# Check for Rust
if ! command -v cargo &>/dev/null; then
    echo "Rust not found. Please install it first: https://rustup.rs"
    exit 1
fi

# Clone if not in repo
if [ ! -f "Cargo.toml" ]; then
    git clone https://github.com/pratikacharya1234/dipralix.git
    cd dipralix
fi

cargo build --release
sudo cp target/release/dipralix-cli /usr/local/bin/

echo "DIPRALIX installed successfully! Run 'dipralix-cli' to start."
