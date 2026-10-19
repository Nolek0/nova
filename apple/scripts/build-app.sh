#!/bin/zsh
# Construit build/NovaAI.app à partir des sources (aucun Xcode requis).
# Usage : ./scripts/build-app.sh [debug|release]   (release par défaut)
set -euo pipefail
cd "$(dirname "$0")/.."
CONFIG="${1:-release}"
APP="build/NovaAI.app"

echo "→ Compilation ($CONFIG)…"
swift build -c "$CONFIG" --product NovaAI
BIN="$(swift build -c "$CONFIG" --show-bin-path)/NovaAI"

echo "→ Assemblage de $APP…"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/NovaAI"
cp Resources/Info.plist "$APP/Contents/Info.plist"

ICON_CACHE="build/AppIcon.icns"
if [[ ! -f "$ICON_CACHE" || scripts/make-icon.swift -nt "$ICON_CACHE" ]]; then
  echo "→ Génération de l'icône…"
  TMP="build/icon.iconset"; rm -rf "$TMP"; mkdir -p "$TMP"
  swift scripts/make-icon.swift build/icon-1024.png
  for s in 16 32 128 256 512; do
    sips -z $s $s build/icon-1024.png --out "$TMP/icon_${s}x${s}.png" >/dev/null
    sips -z $((s*2)) $((s*2)) build/icon-1024.png --out "$TMP/icon_${s}x${s}@2x.png" >/dev/null
  done
  iconutil -c icns "$TMP" -o "$ICON_CACHE"
  rm -rf "$TMP" build/icon-1024.png
fi
cp "$ICON_CACHE" "$APP/Contents/Resources/AppIcon.icns"

echo "→ Signature locale (ad hoc)…"
codesign --force --sign - --timestamp=none "$APP"

echo "✓ Application prête : $APP"
echo "  Lancer : open $APP"
