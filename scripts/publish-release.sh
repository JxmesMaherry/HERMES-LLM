#!/bin/bash
# Publish only a checked tag, after both verified assets have reached a draft.
set -euo pipefail
[ "$#" -eq 2 ] || { echo 'Usage: publish-release.sh vVERSION /path/to/artifacts' >&2; exit 1; }
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
tag="$1"
version="$(node -p "require(process.argv[1]).version" "$REPO_DIR/package.json")"
[ "$tag" = "v$version" ] || { echo 'Tag does not match the application version.' >&2; exit 1; }
artifact_dir="$(cd "$2" && pwd)"
dmg="HERMES-LLM_${version}_aarch64.dmg"
[ -s "$artifact_dir/$dmg" ] && [ -s "$artifact_dir/SHA256SUMS.txt" ]
# Accept precisely this installer, not extra paths supplied through a checksum file.
node -e 'const fs=require("node:fs");const line=fs.readFileSync(process.argv[1],"utf8").trim();if(!new RegExp("^[a-f0-9]{64}  "+process.argv[2].replace(/[.*+?^${}()|[\]\\]/g,"\\$&")+"$").test(line))process.exit(1)' "$artifact_dir/SHA256SUMS.txt" "$dmg"
(cd "$artifact_dir" && shasum -a 256 -c SHA256SUMS.txt)

if draft="$(gh release view "$tag" --json isDraft --jq '.isDraft' 2>/dev/null)"; then
  [ "$draft" = true ] || { echo 'This release is already published; its assets were not changed.' >&2; exit 1; }
else
  # Current 0.x versions are development previews.
  preview=false
  case "$version" in 0.*) preview=true;; esac
  gh release create "$tag" --verify-tag --draft --prerelease="$preview" \
    --title "HERMES-LLM $tag" --notes-file "$REPO_DIR/docs/RELEASE-NOTES.md"
fi
# Replacement is permitted only while the release is still a draft (for safe retries).
gh release upload "$tag" "$artifact_dir/$dmg" "$artifact_dir/SHA256SUMS.txt" --clobber
assets="$(gh release view "$tag" --json assets --jq '[.assets[].name] | sort | join("\n")')"
expected_assets="$(printf '%s\n%s' "$dmg" SHA256SUMS.txt)"
[ "$assets" = "$expected_assets" ] || { echo 'The draft does not contain exactly the installer and checksum; publication stopped.' >&2; exit 1; }
gh release edit "$tag" --draft=false
