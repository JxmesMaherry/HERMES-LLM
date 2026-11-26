#!/bin/bash
set -euo pipefail
source "$(cd "$(dirname "$0")" && pwd)/common.sh"
find_docker
load_config
load_secret
compose logs --tail 120 --no-color open-webui
