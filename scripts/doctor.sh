#!/bin/bash
set -euo pipefail
source "$(cd "$(dirname "$0")" && pwd)/common.sh"
issues=0
check() { if "$@" >/dev/null 2>&1; then return 0; else return 1; fi; }
if command -v node >/dev/null; then echo 'OK: Node.js installed'; else echo 'MISSING: Node.js 22.12+'; issues=$((issues+1)); fi
if command -v cargo >/dev/null; then echo 'OK: Rust installed'; else echo 'MISSING: Rust stable (required to build the desktop app)'; issues=$((issues+1)); fi
find_docker
if check "$DOCKER_BIN" info; then echo 'OK: Docker engine running'; else echo 'STOPPED: Open Docker Desktop and wait for the engine'; issues=$((issues+1)); fi
if check "$DOCKER_BIN" compose version; then echo 'OK: Docker Compose plugin'; else echo 'MISSING: Docker Compose plugin'; issues=$((issues+1)); fi
if check curl --noproxy '*' -fsS --max-time 5 http://127.0.0.1:11434/api/tags; then echo 'OK: native Ollama API'; else echo 'UNAVAILABLE: Open Ollama; its API must listen on 127.0.0.1:11434'; issues=$((issues+1)); fi
if [ -f "$CONFIG_DIR/config.json" ]; then
  load_config
  if check curl --noproxy '*' -fsS --max-time 5 "http://127.0.0.1:$HERMES_PORT/health"; then echo 'OK: Open WebUI health'; else echo 'UNAVAILABLE: Open WebUI; run scripts/start.sh or inspect logs'; issues=$((issues+1)); fi
  printf 'Configured model: %s\n' "$(read_setting model)"
  printf 'Document folder: %s\n' "$(read_setting folder)"
else echo 'UNCONFIGURED: run scripts/bootstrap.sh'; issues=$((issues+1)); fi
printf 'Checks complete: %s issue(s).\n' "$issues"
[ "$issues" -eq 0 ]
