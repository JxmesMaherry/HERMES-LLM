#!/bin/bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
CONFIG_DIR="${HERMES_CONFIG_DIR:-$HOME/Library/Application Support/com.jxmesmaherry.hermesllm}"
fail() { echo "HERMES: $*" >&2; exit 1; }
find_docker() {
  for candidate in /Applications/Docker.app/Contents/Resources/bin/docker /usr/local/bin/docker /opt/homebrew/bin/docker; do
    if [ -x "$candidate" ]; then DOCKER_BIN="$candidate"; return; fi
  done
  fail 'Install Docker Desktop from https://www.docker.com/products/docker-desktop/'
}
require_node() {
  command -v node >/dev/null || fail 'Install Node.js 22.12+ to use source development scripts.'
  node -e 'const [major,minor]=process.versions.node.split(".").map(Number);process.exit(major>22||(major===22&&minor>=12)?0:1)' || fail 'Node.js 22.12+ is required.'
}
read_setting() { node -e 'const fs=require("node:fs"); const c=JSON.parse(fs.readFileSync(process.argv[1],"utf8")); const v=c[process.argv[2]]; if(v!==undefined&&v!==null)process.stdout.write(String(v));' "$CONFIG_DIR/config.json" "$1"; }
load_config() {
  require_node
  [ -f "$CONFIG_DIR/config.json" ] || fail 'Run scripts/bootstrap.sh first.'
  HERMES_PORT="$(read_setting port)"
  [[ "$HERMES_PORT" =~ ^[0-9]+$ ]] && [ "$HERMES_PORT" -ge 1024 ] && [ "$HERMES_PORT" -le 65535 ] || fail 'Configured HTTP port must be 1024–65535.'
  HERMES_HF_OFFLINE=1
  if [ "$(read_setting allow_downloads)" = true ]; then HERMES_HF_OFFLINE=0; fi
  export HERMES_PORT HERMES_HF_OFFLINE
}
load_secret() {
  [ "$(uname -s)" = Darwin ] || fail 'The source scripts currently require macOS Keychain.'
  if WEBUI_SECRET_KEY="$(/usr/bin/security find-generic-password -s com.jxmesmaherry.hermesllm -a backend-secret -w 2>/dev/null)"; then :
  else
    keychain_status=$?
    [ "$keychain_status" -eq 44 ] || fail 'Keychain access was denied or unavailable. Unlock the login Keychain and allow the trusted /usr/bin/security request, then retry.'
    WEBUI_SECRET_KEY="$(node -e 'process.stdout.write(require("node:crypto").randomBytes(32).toString("hex"))')"
    /usr/bin/security add-generic-password -s com.jxmesmaherry.hermesllm -a backend-secret -w "$WEBUI_SECRET_KEY" >/dev/null || fail 'Unlock your login Keychain and retry.'
  fi
  export WEBUI_SECRET_KEY
}
compose() { "$DOCKER_BIN" compose --project-name hermes-llm --file "$CONFIG_DIR/compose.yaml" "$@"; }
