#!/bin/zsh
# Fabrique NOVA.ipa (non signé) pour l'installer gratuitement avec SideStore ou AltStore,
# qui le signent avec l'identifiant Apple de la personne qui l'installe.
# Usage : ./build-ipa.sh   →   ios/build/NOVA.ipa
set -euo pipefail
cd "$(dirname "$0")"
echo "→ Compilation pour iPhone (Release)…"
xcodebuild -project NovaPhone.xcodeproj -scheme NovaPhone -configuration Release -sdk iphoneos \
  -derivedDataPath build CODE_SIGNING_ALLOWED=NO CODE_SIGNING_REQUIRED=NO build -quiet
APP="build/Build/Products/Release-iphoneos/NovaPhone.app"
rm -rf build/ipa && mkdir -p build/ipa/Payload
cp -R "$APP" build/ipa/Payload/
(cd build/ipa && zip -qry ../NOVA.ipa Payload)
rm -rf build/ipa
echo "✓ build/NOVA.ipa prêt ($(du -h build/NOVA.ipa | cut -f1)). Ouvre-le avec SideStore ou AltStore sur l'iPhone."
