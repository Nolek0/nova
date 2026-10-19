#!/bin/zsh
# Construit NOVA pour macOS : module Swift (Apple) + application Tauri (.app et .dmg).
# Prérequis : Command Line Tools (xcode-select --install) et Rust (https://rustup.rs).
set -euo pipefail
cd "$(dirname "$0")/.."
source "$HOME/.cargo/env" 2>/dev/null || true
command -v cargo-tauri >/dev/null || cargo install tauri-cli --version "^2" --locked
./scripts/build-apple-helper.sh
cd app/src-tauri
cargo tauri build "$@"
echo "✓ Application : app/src-tauri/target/release/bundle/macos/NOVA.app"
echo "✓ Installeur  : app/src-tauri/target/release/bundle/dmg/"
