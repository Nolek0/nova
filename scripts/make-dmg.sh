#!/bin/zsh
# Fabrique un DMG « sorti d'usine » de NOVA, à partager.
#
# Garanties :
# - aucune donnée personnelle : conversations, réglages, modèles et pièces jointes vivent dans
#   ~/Library/Application Support/local.nova.ai et ne sont jamais copiés dans l'app ;
# - aucun chemin personnel (/Users/<toi>/…) dans les programmes : ils sont anonymisés à la compilation ;
# - app signée localement (ad hoc), signature vérifiée, pour éviter le message « app endommagée ».
#
# Usage : ./scripts/make-dmg.sh      → dist/NOVA-<version>-mac-apple-silicon.dmg
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"
source "$HOME/.cargo/env" 2>/dev/null || true
VERSION=$(python3 -c "import json;print(json.load(open('app/src-tauri/tauri.conf.json'))['version'])")
ME=$(whoami)

echo "→ Module Apple (Swift)…"
(cd apple && swift build -c release --product nova-apple-helper \
  -Xswiftc -file-prefix-map -Xswiftc "$HOME=/build" >/dev/null)
TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
mkdir -p app/src-tauri/binaries
cp "$(cd apple && swift build -c release --show-bin-path)/nova-apple-helper" "app/src-tauri/binaries/nova-apple-helper-$TRIPLE"

# Paquet de mise à jour signé si la clé est présente (sinon : app sans auto-update).
KEY="$HOME/.tauri/nova-updater.key"
EXTRA=()
if [[ -f "$KEY" ]]; then
  export TAURI_SIGNING_PRIVATE_KEY="$(cat "$KEY")"
  export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
else
  EXTRA=(--config '{"bundle":{"createUpdaterArtifacts":false}}')
fi

echo "→ Application (Rust, chemins anonymisés)…"
export RUSTFLAGS="--remap-path-prefix=$HOME/.cargo/registry/src=/crates --remap-path-prefix=$HOME/.rustup=/rustup --remap-path-prefix=$ROOT=/nova --remap-path-prefix=$HOME=/build"
(cd app/src-tauri && cargo tauri build --bundles app "${EXTRA[@]}" >/dev/null)
APP="app/src-tauri/target/release/bundle/macos/NOVA.app"

echo "→ Vérification : aucune trace personnelle…"
LEAKS=$(find "$APP" -type f -exec strings {} \; 2>/dev/null | grep -c -i -e "/Users/$ME" -e "$ME" || true)
if [[ "$LEAKS" != "0" ]]; then
  echo "✗ $LEAKS mention(s) de « $ME » trouvée(s) dans l'app : DMG non créé." >&2
  exit 1
fi

# Signature : certificat stable « NOVA Dev » s'il existe dans le trousseau (macOS garde alors les
# autorisations d'Accessibilité et d'Enregistrement de l'écran d'une version à l'autre), sinon signature locale.
SIGN_ID="${NOVA_SIGN_ID:-NOVA Dev}"
if security find-certificate -c "$SIGN_ID" >/dev/null 2>&1 && codesign --force --deep --sign "$SIGN_ID" --timestamp=none "$APP" 2>/dev/null; then
  echo "→ Signé avec le certificat « $SIGN_ID » (autorisations conservées entre les versions)"
else
  echo "→ Signature locale (sans certificat : macOS redemandera les autorisations après chaque version)"
  codesign --force --deep --sign - --timestamp=none "$APP"
fi
codesign --verify --deep --strict "$APP"

echo "→ Création du DMG…"
STAGE="$(mktemp -d)/NOVA"
mkdir -p "$STAGE"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
cat > "$STAGE/LISEZ-MOI - Installer NOVA.txt" <<'TXT'
NOVA — assistant personnel gratuit, qui fonctionne sur ton Mac
==============================================================

INSTALLER
1. Glisse NOVA sur le dossier « Applications ».
2. Ouvre NOVA depuis le dossier Applications.

AU PREMIER LANCEMENT, macOS affiche « Apple n'a pas pu vérifier… »
C'est normal : NOVA est une app gratuite, non distribuée par l'App Store.
→ Ouvre Réglages Système › Confidentialité et sécurité,
  descends jusqu'au message sur NOVA et clique « Ouvrir quand même ».
  (À faire une seule fois.)

Si macOS dit que l'app est « endommagée », ouvre le Terminal et tape :
  xattr -dr com.apple.quarantine /Applications/NOVA.app
puis relance NOVA.

ENSUITE
- NOVA te propose un modèle d'IA gratuit adapté à ton Mac (page « Modèles ») :
  c'est toi qui décides de le télécharger. Tout fonctionne ensuite sans Internet.
- Tes conversations restent sur ton Mac.

CONFIGURATION
- Mac avec puce Apple (M1, M2, M3, M4, M5…), macOS 13 ou plus récent.
- Modèle Apple et photo avec l'iPhone : macOS 26 ou plus récent.
TXT
mkdir -p dist
DMG="dist/NOVA-$VERSION-mac-apple-silicon.dmg"
rm -f "$DMG"
hdiutil create -volname "NOVA" -srcfolder "$STAGE" -fs HFS+ -format UDZO -ov "$DMG" >/dev/null
rm -rf "$(dirname "$STAGE")"
echo "✓ $DMG ($(du -h "$DMG" | cut -f1))"
