#!/bin/zsh
# Publie une nouvelle version de NOVA sur GitHub : l'app de chacun la proposera via
# « NOVA › Rechercher des mises à jour… » (téléchargement, vérification de signature, installation).
#
# Usage : ./scripts/publish.sh 0.4.1 "Ce qui change dans cette version"
# Prérequis (une fois) : gh connecté (gh auth login) ; clé ~/.tauri/nova-updater.key ;
#                        certificat « NOVA Dev » dans le trousseau (conseillé).
set -euo pipefail
cd "$(dirname "$0")/.."
VERSION="${1:?Numéro de version requis, ex. 0.4.1}"
NOTES="${2:-Voir CHANGELOG.md}"
GH="${GH:-$HOME/.local/bin/gh}"
REPO="Nolek0/nova"

"$GH" auth status >/dev/null 2>&1 || { echo "GitHub : lance d'abord « $GH auth login »." >&2; exit 1; }

echo "→ Version $VERSION"
python3 - "$VERSION" <<'PY'
import json, re, sys
v = sys.argv[1]
p = "app/src-tauri/tauri.conf.json"; c = json.load(open(p)); c["version"] = v
open(p, "w").write(json.dumps(c, indent=2, ensure_ascii=False) + "\n")
p = "app/src-tauri/Cargo.toml"; s = open(p).read()
open(p, "w").write(re.sub(r'(?m)^version = "[^"]+"', f'version = "{v}"', s, count=1))
PY

./scripts/release-mac.sh "$NOTES"

echo "→ Dépôt GitHub"
if ! "$GH" repo view "$REPO" >/dev/null 2>&1; then
  "$GH" repo create "$REPO" --public --source . --description "$(sed -n '/^> /{s/^> //;p;q;}' docs/DESCRIPTION.md)"
fi
git remote get-url origin >/dev/null 2>&1 || git remote add origin "https://github.com/$REPO.git"
git add -A
git commit -q -m "NOVA $VERSION" || true
git tag -f "v$VERSION"
git push -u origin HEAD --tags

echo "→ Release v$VERSION"
"$GH" release create "v$VERSION" --repo "$REPO" --title "NOVA $VERSION" --notes "$NOTES" \
  "dist/NOVA-$VERSION-mac-apple-silicon.dmg" "dist/NOVA_${VERSION}_aarch64.app.tar.gz" "dist/NOVA_${VERSION}_aarch64.app.tar.gz.sig" dist/latest.json
echo "✓ Publié : https://github.com/$REPO/releases/tag/v$VERSION"
echo "  Windows : GitHub Actions construit l'installateur automatiquement (secret TAURI_SIGNING_PRIVATE_KEY requis)."
