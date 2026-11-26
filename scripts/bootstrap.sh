#!/bin/bash
set -euo pipefail
source "$(cd "$(dirname "$0")" && pwd)/common.sh"
require_node
find_docker
[ -d /Applications/Ollama.app ] || command -v ollama >/dev/null || fail 'Install Ollama from https://ollama.com/download/mac'
[ "${1:-}" = '' ] || [ "$1" = --allow-downloads ] || fail 'Usage: bootstrap.sh [--allow-downloads]'
umask 077
mkdir -p "$CONFIG_DIR"
chmod 700 "$CONFIG_DIR"
node - "$CONFIG_DIR/config.json" "${1:-}" <<'JS'
const fs=require('node:fs');const path=process.argv[2];
const config=fs.existsSync(path)?JSON.parse(fs.readFileSync(path,'utf8')):{port:3000,model:'qwen3:4b-instruct',folder:null,knowledge_id:null,theme:'dark',setup_complete:false,allow_downloads:false,max_file_mb:20};
if(process.argv[3]==='--allow-downloads')config.allow_downloads=true;
const temp=path+'.tmp';fs.writeFileSync(temp,JSON.stringify(config,null,2),{mode:0o600});fs.renameSync(temp,path);
JS
cp "$REPO_DIR/infra/docker/compose.yaml" "$CONFIG_DIR/compose.yaml"
load_config
load_secret
compose config --quiet
printf 'HERMES configuration ready at %s\n' "$CONFIG_DIR"
if [ "$HERMES_HF_OFFLINE" = 1 ]; then echo 'Embedding downloads are disabled. For a new backend, rerun with --allow-downloads to consent to the Hugging Face model download.'; fi
echo 'Next: open Docker Desktop and Ollama, then run scripts/start.sh.'
