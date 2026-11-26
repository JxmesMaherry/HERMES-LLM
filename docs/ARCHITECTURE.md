# Architecture

HERMES is a macOS desktop client with a separately managed local backend.

| Layer | Responsibility | Location |
| --- | --- | --- |
| React / TypeScript / Vite | Original UI, wizard, progress, safe Markdown | `apps/desktop/src` |
| Tauri 2 / Rust | Validated IPC, files, credentials, service lifecycle and HTTP | `apps/desktop/src-tauri/src` |
| Docker Compose | Pinned Open WebUI, local embeddings, persistent volume | `infra/docker/compose.yaml` |
| Native Ollama | Local model execution and Metal acceleration | Installed separately |

## Native modules

- `config.rs`: validated preferences, atomic mode-0600 writes and Keychain access.
- `services.rs`: known Docker binary paths, daemon checks, explicit command arguments, lifecycle and readiness timeouts.
- `api.rs`: authenticated Open WebUI calls, local Ollama model management, chat SSE decoder and history validation.
- `sync.rs`: bounded filesystem scan, capability-relative document access, manifests, upstream diff, asynchronous processing and cleanup.
- `lib.rs`: narrow Tauri command surface and operation locking. Folder paths can only be changed through the Rust-owned native dialog.

The frontend has no general-purpose shell, filesystem, HTTP or opener capability. Rust HTTP endpoints are built from `127.0.0.1` and validated ports; proxy inheritance and redirects are disabled. Only allowlisted official download pages and local backend pages can be opened.

## Backend contract

The supported image is `ghcr.io/open-webui/open-webui:v0.13.5`. The implementation was inspected at that exact Git tag. Upgrades must run the contract and live retrieval tests before changing the pin.

| Operation | API |
| --- | --- |
| Local sign in / first administrator | `POST /api/v1/auths/signin` / `signup` |
| Discover Ollama models | `GET http://127.0.0.1:11434/api/tags` |
| Download Ollama model | `POST http://127.0.0.1:11434/api/pull` |
| Chat / knowledge retrieval | `POST /api/chat/completions`, SSE, `files: [{type: "collection", id}]` |
| Knowledge list / create | `GET /api/v1/knowledge/?page=N`, `POST /api/v1/knowledge/create` |
| Read-only incremental comparison | `POST /api/v1/knowledge/{id}/sync/diff` |
| Create knowledge subdirectories | `POST /api/v1/knowledge/{id}/dirs/create` |
| Upload and auto-link to knowledge | `POST /api/v1/files/`, multipart `metadata` with `knowledge_id`, `file_hash`, optional `directory_id` |
| Wait for extraction, embedding and linking | `GET /api/v1/files/{id}/process/status` until `completed` |
| Remove confirmed stale index entries | `POST /api/v1/knowledge/{id}/sync/cleanup` |

Chat explicitly selects legacy retrieval mode for predictable context injection with small instruct models. It uses actual backend retrieval, not a frontend-generated answer. Open WebUI may change API semantics; the local browser option is available when native retrieval needs investigation.

## Sync transaction boundaries

1. Scan a native-picker-selected root, exclude unsafe paths, read each document through a filesystem capability and hash its bytes.
2. Send `{manifest: [{path, filename, checksum, size}]}`. Save a one-use preview in Rust memory, bound to that folder and KB for ten minutes.
3. Create missing directories in parent-first order.
4. Re-read each requested document safely and verify its hash still matches the preview. Upload only added/modified files and await processing completion.
5. If every upload succeeded, clean up successfully replaced IDs and explicitly confirmed deletions. Any failure suppresses all cleanup.
6. Persist attempt results and a separate latest-success record. Display counts and errors.

The app does not delete empty directories: upstream directory cleanup can remove files recursively and does not safely account for another client adding content after preview. Remote edits between preview and apply are not an atomic server transaction; use a dedicated KB for each document source. A process crash between upload and cleanup can leave duplicate/stale backend entries. Preview again and inspect Open WebUI before deleting content.

## Persistence and lifecycle

Application data: `~/Library/Application Support/com.jxmesmaherry.hermesllm/`. Files include `config.json`, copied `compose.yaml`, `conversations.json`, `sync-attempt.json`, and `sync-success.json`. User data is never written inside the installed `.app` or repository.

Keychain service: `com.jxmesmaherry.hermesllm`; accounts: `backend-secret` and `webui-token-<port>`. No API key or password is stored in JSON. Open WebUI stores its own data in `hermes-llm-open-webui-data`.

Closing HERMES leaves services running. Explicit Stop uses `docker compose stop`; it does not call `down -v`, prune, remove models or delete documents. Restart recreates only the managed backend container with current settings and the same volume. Focus and 30-second status checks recover after sleep without automatically restarting failed services.

## Official references

- [Tauri configuration](https://v2.tauri.app/reference/config/), [resources](https://v2.tauri.app/develop/resources/) and [native dialogs](https://v2.tauri.app/plugin/dialog/).
- [Open WebUI API guidance](https://docs.openwebui.com/reference/api-endpoints/), [knowledge sync](https://docs.openwebui.com/features/workspace/knowledge/).
- Pinned source: [knowledge router](https://github.com/open-webui/open-webui/blob/v0.13.5/backend/open_webui/routers/knowledge.py), [files router](https://github.com/open-webui/open-webui/blob/v0.13.5/backend/open_webui/routers/files.py), [chat middleware](https://github.com/open-webui/open-webui/blob/v0.13.5/backend/open_webui/utils/middleware.py), [authentication](https://github.com/open-webui/open-webui/blob/v0.13.5/backend/open_webui/routers/auths.py).
- [Official oikb reference client](https://github.com/open-webui/oikb), [Ollama API](https://docs.ollama.com/api).
