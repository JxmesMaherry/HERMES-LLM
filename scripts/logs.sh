#!/bin/bash
set -euo pipefail
source "$(cd "$(dirname "$0")" && pwd)/common.sh"
find_docker
load_config
export WEBUI_SECRET_KEY=unused-for-stop-or-logs
compose logs --tail 120 --no-color open-webui
