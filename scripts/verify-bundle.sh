#!/bin/bash
set -euo pipefail
[ "$#" -eq 2 ] || { echo 'Usage: verify-bundle.sh /path/HERMES-LLM.app /path/file.dmg' >&2; exit 1; }
app="$1"
dmg="$2"
[ -d "$app" ] && [ -s "$dmg" ]
[ -s "$app/Contents/Resources/infra/compose.yaml" ]
[ -s "$app/Contents/Resources/THIRD-PARTY-LICENSES.txt" ]
[ -s "$app/Contents/Resources/LICENSE" ]
executable="$(/usr/libexec/PlistBuddy -c 'Print CFBundleExecutable' "$app/Contents/Info.plist")"
[ -x "$app/Contents/MacOS/$executable" ]
file "$app/Contents/MacOS/$executable"
strings_file="$(mktemp)"
trap 'rm -f "$strings_file"' EXIT
strings "$app/Contents/MacOS/$executable" > "$strings_file"
grep -q 'index\.html' "$strings_file" || { echo 'Embedded frontend index missing' >&2; exit 1; }
codesign --verify --deep --strict "$app"
hdiutil verify "$dmg"
echo 'Verified app executable, embedded frontend index, bundled Compose resource, signature integrity and DMG.'
