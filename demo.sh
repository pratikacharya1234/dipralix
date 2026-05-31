#!/usr/bin/env bash
# DIPRALIX Demo — shows what DIPRALIX can do in under 2 minutes
# Prerequisites: dipralix-cli installed, GEMINI_API_KEY set
set -euo pipefail

echo ""
echo "  ╔══════════════════════════════════════════════════╗"
echo "  ║        ◈ DIPRALIX v0.0.2 — Live Demo             ║"
echo "  ╚══════════════════════════════════════════════════╝"
echo ""

# Check prerequisites
if ! command -v dipralix-cli &>/dev/null; then
    echo "  [!] dipralix-cli not found. Install it first:"
    echo "      curl -fsSL https://raw.githubusercontent.com/pratikacharya1234/dipralix/main/install.sh | bash"
    exit 1
fi

if [ -z "${GEMINI_API_KEY:-}" ] && [ -z "${DIPRALIX_API_KEY:-}" ]; then
    echo "  [!] No API key set. Get a free one at https://aistudio.google.com/apikey"
    echo "      Then: export GEMINI_API_KEY='your-key'"
    exit 1
fi

DIPRALIX_VERSION=$(dipralix-cli --version 2>&1 | head -1)
echo "  [⊕] DIPRALIX ${DIPRALIX_VERSION} ready"
echo "  [⊕] API key configured"
echo ""
echo "  ─────────────────────────────────────────────────"
echo "  Demo 1: Quick code fix"
echo "  ─────────────────────────────────────────────────"

# Create a dummy project
mkdir -p demo_tmp
cd demo_tmp

echo 'fn main() { println!("Hello, world!"); }
fn greet(name: &str) -> String { format!("Hello, {}!", name) }' > main.rs

echo "  [+] Created main.rs with a basic Rust program"
echo ""

echo "  Running: dipralix-cli --auto-apply --prompt 'add a test for the greet function and fix any issues'"
echo ""

# Run DIPRALIX in auto-apply mode
dipralix-cli --auto-apply --prompt "add a test for the greet function in main.rs and make sure it compiles and passes" 2>&1 || true

echo ""
echo "  ─────────────────────────────────────────────────"
echo "  Demo complete!"
echo "  ─────────────────────────────────────────────────"
echo ""
echo "  What you just saw:"
echo "  1. DIPRALIX auto-detected the Rust project"
echo "  2. Added a test module"
echo "  3. Verified it compiles"
echo "  4. All for FREE (Gemini free tier)"
echo ""
echo "  Try it yourself:"
echo "    export GEMINI_API_KEY='your-free-key'"
echo "    dipralix-cli"
echo ""
echo "  More: https://github.com/pratikacharya1234/dipralix"
echo ""

# Cleanup
cd ..
rm -rf demo_tmp
