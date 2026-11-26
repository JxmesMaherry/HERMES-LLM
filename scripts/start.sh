#!/bin/bash
set -euo pipefail
source "$(cd "$(dirname "$0")" && pwd)/common.sh"
find_docker
load_config
load_secret
"$DOCKER_BIN" info >/dev/null 2>&1 || fail 'Open Docker Desktop and wait for the engine.'
curl --noproxy '*' -fsS --max-time 5 http://127.0.0.1:11434/api/tags >/dev/null || fail 'Open Ollama first. Keep its API on loopback.'
# A listener is allowed only when Docker reports it belongs to our existing service.
if ! node - "$HERMES_PORT" <<'JS'
const net=require('node:net');const s=net.createServer();s.once('error',()=>process.exit(1));s.listen(Number(process.argv[2]),'127.0.0.1',()=>s.close());
JS
then
  ports="$("$DOCKER_BIN" ps --filter label=com.docker.compose.project=hermes-llm --filter label=com.docker.compose.service=open-webui --format '{{.Ports}}')"
  [[ "$ports" = *"127.0.0.1:$HERMES_PORT->8080/tcp"* ]] || fail 'Port is already used by another application. Change the HTTP port in Settings.'
fi
compose up -d
echo 'Waiting up to 6 minutes for backend readiness…'
for ((attempt=0; attempt<180; attempt++)); do
  if curl --noproxy '*' -fsS --max-time 3 "http://127.0.0.1:$HERMES_PORT/health" >/dev/null 2>&1; then printf 'Ready: http://127.0.0.1:%s\n' "$HERMES_PORT"; exit 0; fi
  sleep 2
done
fail 'Backend did not become healthy. Run scripts/logs.sh; check storage, model download consent and Docker resources.'
