# HERMES-LLM

**Private AI, on your Mac.** Chat with a local model and ask questions about your PDF, TXT and Markdown documents. HERMES combines a native macOS interface, Ollama and Open WebUI. No paid AI account is needed.

> Development preview for Apple Silicon. The DMG is ad-hoc signed, not Apple-notarized. Public downloads become available after the first release is published.

## Install

1. Install [Ollama](https://ollama.com/download/mac) and [Docker Desktop for Apple Silicon](https://www.docker.com/products/docker-desktop/). Open both applications.
2. Download `HERMES-LLM_<version>_aarch64.dmg` from [GitHub Releases](https://github.com/JxmesMaherry/HERMES-LLM/releases).
3. Open the DMG, drag **HERMES-LLM** into **Applications**, then launch it.
4. Follow the Setup guide: start the local backend, create/sign in to a local account, choose an installed model or explicitly download one, and select a document folder and knowledge base.
5. Open **Library → Preview changes → Sync now**. In **Chat**, enable **Use knowledge base** to ask about your documents.

You do not need Node.js, Rust, a Git clone or an environment file to use the packaged app. Docker Desktop and Ollama remain separate installations. Your macOS version must satisfy their current requirements; the HERMES app itself targets macOS 13+.

If Gatekeeper blocks a verified download, use **System Settings → Privacy & Security → Open Anyway** when macOS offers it. Do not disable Gatekeeper or strip quarantine attributes. See the [simple installation guide](docs/QUICKSTART.md) for checksums and first-run help.

## Which Mac and model?

These are conservative workload recommendations for quantised models with a short starting context and one active request. All levels use the same app and features. RAM, context length and concurrent requests determine what fits; model download size alone is insufficient.

| Workload | Suggested Mac | Unified memory | Starting model |
| --- | --- | --- | --- |
| Everyday study and document Q&A | Apple Silicon MacBook Air, Mac mini or iMac; a 16 GB M5 Air is the tested development configuration | 16 GB | `qwen3:4b-instruct` (~2.5 GB download) |
| More detailed explanations and coding | 24–32 GB Air for occasional use; Mac mini or MacBook Pro for sustained use | 24–32 GB | `qwen3:8b` (~5.2 GB); consider `qwen3:14b` (~9.3 GB) on 32 GB |
| Larger coding and reasoning models | MacBook Pro with Pro/Max chip, Mac mini Pro, or Mac Studio | 48–64 GB | `qwen3:30b` (~19 GB) or `qwen3:32b` (~20 GB) |
| Large-model workstation | Mac Studio Max/Ultra, or a suitably configured 96–128 GB Max MacBook Pro | 96–128+ GB | `llama3.3:70b` (~43 GB), subject to its licence |

**8 GB Macs:** experimental with small 0.6B–1.7B models; the full Docker/RAG stack has little memory headroom. We recommend 16 GB or more. Intel Macs do not have a packaged build in this release.

Start at **4,096 context tokens and concurrency 1**, then measure. The table is a planning estimate, not a benchmark or fit guarantee for every Mac. Reserve memory for macOS, Docker, embeddings and context cache. The [hardware guide](docs/HARDWARE.md) explains these choices and links the official [Qwen model sizes](https://ollama.com/library/qwen3/tags) and [Apple specifications](https://www.apple.com/mac/compare/).

## Your documents

Choose a dedicated local folder. HERMES uses SHA-256 checksums to upload only new or changed documents and waits for indexing to finish. Deletions are previewed and require explicit confirmation; source documents are never deleted. Hidden files, sensitive paths and symlinks are excluded.

Manual sync is supported now. Scanned PDFs may need OCR outside HERMES. The default file limit is 20 MB, adjustable up to 50 MB. Having eight textbooks indexed does not place all eight books into the model’s context; retrieval selects relevant passages for each question.

## Stress-test your setup

For developers with Node.js 22.12+, run these from the repository after the backend is ready and indexing has completed:

```sh
# Preview the test plan without requests or Keychain access.
node scripts/stress-test.mjs --knowledge --requests 3 --dry-run

# Exercise textbook retrieval through authenticated Open WebUI.
node scripts/stress-test.mjs --knowledge --requests 3 --concurrency 1

# Compare with the model runtime alone; this does not use your documents.
node scripts/stress-test.mjs --target ollama --requests 6 --concurrency 1
```

The tool reports first-token latency, total latency, p50/p95, errors and emitted retrieval sources. It is bounded, makes no configuration changes and does not save prompts or answers. See [stress testing](docs/STRESS-TESTING.md) for concurrency, soak tests, eight-book accuracy checks, Keychain authentication and monitoring commands.

## Privacy and operation

The interface is React/TypeScript in Tauri 2. Rust manages local files, Keychain credentials and services. Open WebUI v0.13.5 runs in Docker and connects to native Ollama for GPU-accelerated inference.

Models and documents stay local during chat/indexing. Starting the backend may download its official image; model and optional embedding downloads require explicit actions. Cloud model connections, web search and telemetry are disabled in the supported configuration. Desktop and Open WebUI browser chat histories are separate.

Closing HERMES leaves services running. Use **Settings → Stop** to stop the backend while preserving data. See [security and backups](docs/SECURITY.md) and [troubleshooting](docs/TROUBLESHOOTING.md).

## Build from source

Install Node.js 22.12+ (24 recommended), Rust stable and Apple Command Line Tools, alongside Docker Desktop and Ollama.

```sh
git clone https://github.com/JxmesMaherry/HERMES-LLM.git
cd HERMES-LLM
npm ci
npm run desktop
```

Complete setup in the app. Optional CLI setup uses `./scripts/bootstrap.sh`, `./scripts/start.sh`, `./scripts/doctor.sh` and `./scripts/stop.sh`; allow the standard trusted Keychain request when prompted.

```sh
npm test        # Frontend and stress-tool contract tests
npm run build   # Frontend production build
npm run dmg     # Apple Silicon app and DMG
```

Read [development](docs/DEVELOPMENT.md), [architecture](docs/ARCHITECTURE.md), [actual verification results](docs/VERIFICATION.md), [release instructions](docs/RELEASE.md) and [the handover](docs/HANDOVER.md). The GitHub release workflow supports manual artifact builds and `v*` tag releases; it has not yet been run remotely.

## Contributing and licences

Use focused commits, include tests for API/file/deletion changes, run the documented checks, and keep private documents, credentials and local data out of Git. [MIT](LICENSE) covers HERMES code. [Open WebUI](https://github.com/open-webui/open-webui), [Ollama](https://github.com/ollama/ollama), Docker and downloaded models retain their own terms. Open WebUI’s original branding is preserved. See [third-party notices](docs/THIRD-PARTY.md).

**Screenshot placeholder:** a public native-window screenshot will be added after capturing a clean demonstration workspace without private chats or documents.

If saved credentials are locked, click **Unlock credentials** once. HERMES keeps them in memory for the session and does not ask again during background checks or chat. A changed ad-hoc build can need fresh macOS approval. Your documents and backend data remain intact.
