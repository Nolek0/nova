#!/bin/zsh
# Lance les tests unitaires. Sans Xcode, le framework Swift Testing des
# Command Line Tools doit être indiqué explicitement au compilateur.
set -euo pipefail
cd "$(dirname "$0")/.."
DEV="$(xcode-select -p)"
FW=""
for candidate in "$DEV/Library/Developer/Frameworks" "$DEV/Platforms/MacOSX.platform/Developer/Library/Frameworks"; do
  [[ -d "$candidate/Testing.framework" ]] && FW="$candidate" && break
done
if [[ -z "$FW" ]]; then
  swift test "$@"
else
  LIB="$(dirname "$FW")/usr/lib"
  swift test -Xswiftc -F"$FW" -Xlinker -F"$FW" -Xlinker -rpath -Xlinker "$FW" \
    -Xlinker -rpath -Xlinker "$LIB" "$@"
fi
