#!/bin/zsh
# Compile le module Swift (macOS) et le place là où Tauri l'embarque dans NOVA.app.
set -euo pipefail
cd "$(dirname "$0")/.."
TRIPLE="$(rustc -vV 2>/dev/null | sed -n 's/^host: //p')"
[[ -z "$TRIPLE" ]] && TRIPLE="aarch64-apple-darwin"
(cd apple && swift build -c release --product nova-apple-helper)
BIN="$(cd apple && swift build -c release --show-bin-path)/nova-apple-helper"
mkdir -p app/src-tauri/binaries
cp "$BIN" "app/src-tauri/binaries/nova-apple-helper-$TRIPLE"
echo "✓ Module Apple : app/src-tauri/binaries/nova-apple-helper-$TRIPLE"
