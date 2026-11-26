#!/bin/bash
set -euo pipefail
source "$(cd "$(dirname "$0")" && pwd)/common.sh"
find_docker
load_config
load_secret
compose stop
echo 'HERMES backend stopped. All data and volumes preserved. Ollama and Docker Desktop remain running.'
