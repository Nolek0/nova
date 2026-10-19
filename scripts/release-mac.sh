#!/bin/zsh
# Prépare tous les fichiers macOS d'une release dans dist/ :
#   - NOVA-<v>-mac-apple-silicon.dmg      (installation)
#   - NOVA_<v>_aarch64.app.tar.gz + .sig  (mise à jour automatique, signée)
#   - latest.json                         (lu par l'app pour savoir s'il existe une mise à jour)
# Usage : ./scripts/release-mac.sh "Notes de version"
set -euo pipefail
cd "$(dirname "$0")/.."
source "$HOME/.cargo/env" 2>/dev/null || true
KEY="$HOME/.tauri/nova-updater.key"
[[ -f "$KEY" ]] || { echo "Clé de signature absente : $KEY" >&2; exit 1; }
NOTES="${1:-See CHANGELOG.md}"
VERSION=$(python3 -c "import json;print(json.load(open('app/src-tauri/tauri.conf.json'))['version'])")
REPO="Nolek0/nova"

./scripts/make-dmg.sh

APP="app/src-tauri/target/release/bundle/macos/NOVA.app"
TARBALL="dist/NOVA_${VERSION}_aarch64.app.tar.gz"
echo "→ Paquet de mise à jour…"
COPYFILE_DISABLE=1 tar -czf "$TARBALL" -C "$(dirname "$APP")" NOVA.app
cargo tauri signer sign -f "$KEY" -p "" "$TARBALL" >/dev/null
SIG=$(cat "$TARBALL.sig")

python3 - "$VERSION" "$NOTES" "$SIG" "https://github.com/$REPO/releases/download/v$VERSION/$(basename "$TARBALL")" <<'PY'
import json, sys, datetime
version, notes, sig, url = sys.argv[1:5]
data = {
    "version": version,
    "notes": notes,
    "pub_date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "platforms": {"darwin-aarch64": {"signature": sig, "url": url}},
}
json.dump(data, open("dist/latest.json", "w"), indent=2)
PY
echo "✓ Fichiers prêts dans dist/ :"
ls -lh dist | sed 1d
