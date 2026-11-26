# Test plan

## Automated checks

Frontend tests exercise prerequisite failure states, blocked setup, explicit deletion consent, chat streaming/history and offline send controls. Rust tests cover settings validation and persistence, command errors, filtering, SHA-256, symlink containment, missing roots, preview expiration, unchanged files, processing failure and safe cleanup. Stream tests reject truncated/empty answers, knowledge responses without retrieval sources and incomplete model downloads. Slow mock HTTP requests verify wall-clock startup/indexing deadlines.

`npm test` also runs six publication tests with a fake `gh` executable. These verify upload-before-publication, failed/missing uploads, checksum failure, draft resumption and refusal to overwrite published releases. They never publish to GitHub.

Run the commands in DEVELOPMENT.md. A real backend and model are not needed for the default unit/contract suite; HTTP mocks bind ephemeral loopback sockets. The live test is ignored by default.

## Explicit live test

Start Docker Desktop and native Ollama. Install `qwen3:4b-instruct` deliberately before running the test; the runner refuses to silently download an LLM.

```sh
node scripts/integration.mjs --consent
```

The runner may download the official pinned Open WebUI image. It uses a unique test Compose project, port 13000 (override with `HERMES_TEST_PORT`), a separate named volume and in-memory credentials. It uploads only the repository’s invented-facts fixture into that isolated backend. It does not read user-selected documents, alter production HERMES settings, use the production volume or save API credentials in configuration.

The Rust live test:

1. Scans an OS temporary document folder and uploads a known Markdown fact file.
2. Waits for extraction, local embedding and durable knowledge linking.
3. Verifies a second preview reports unchanged content.
4. Asks Qwen through authenticated Open WebUI knowledge chat for the vessel Aster’s departure code without putting the answer in the prompt.
5. Requires `AMBER-7319` in the actual streamed model response.
6. Modifies the fixture and verifies only that file is replaced.
7. Removes only its own temporary fixture; previews a deletion and first declines removal.
8. Verifies the index is retained, then explicitly approves cleanup and verifies the deletion is gone.

The runner stops its own container in `finally` and preserves its data volume. It never runs prune or volume-removal commands. Test volume names are printed so the operator can inspect them. The official image’s cached MiniLM model is used with Hugging Face offline mode; if it is missing, the test fails rather than silently enabling external downloads.

## Native macOS acceptance checklist

| Case | Evidence to record |
| --- | --- |
| Clean checkout | `npm ci`, frontend build and `cargo test --locked` succeed with documented tools |
| Native launch | Real `.app` opens; frontend and bundled Compose load without a repository dependency |
| Prerequisite states | Quit/start Ollama and Docker; verify missing/stopped/running/failed indications, no false success |
| Initial backend | Start from Setup; wait for readiness; inspect loopback binding and persistent volume |
| Port conflict | Occupy the configured port with a harmless local server; Start refuses to adopt it |
| Local account | User-chosen HERMES password, encrypted session persistence, first administrator signup, sign-in/out, backed-up recovery |
| Model discovery | Installed Qwen appears; unavailable selected model prevents send |
| Chat | Real streamed answer, Markdown/code, cancellation, New conversation and saved history |
| Folder picker | Select a dedicated directory outside the repository, including a path with spaces |
| Sync | First add, second unchanged, modification, skipped/oversized file and processing failure |
| Deletion | Preview missing filenames; unchecked confirmation retains; checked confirmation removes only indexed content |
| RAG | Ask known-facts question; answer contains `AMBER-7319`; source is actually indexed |
| Source changes | Select a different folder; existing KB is retained until preview/explicit decision |
| Permissions | Missing/unreadable subfolder fails clearly; old index remains intact |
| Restart | Close/reopen app; preferences, folder and desktop history survive; backend data persists |
| Sleep/resume | Wake and focus app; service checks refresh; no infinite restart loop |
| Offline | Cached models/image and HF offline mode; chat and fixture retrieval work disconnected |
| Backup | Export app data and Docker volume with backend stopped; restore into a separate test environment |
| DMG | Verify non-empty image, signature, frontend/resources; mount read-only and test install on another Mac/account |
| CI and release | Run actual GitHub workflows, download artifact, verify SHA-256 and install independently |

Do not run tests that remove personal documents, unrelated containers or volumes. A macOS File and Folder permission prompt or Gatekeeper approval must be handled through normal OS interfaces, not by disabling protections.

Password-vault tests cover wrong passwords, tampering, parameter bounds, cached access, encrypted reset backups and document preservation. Run `python3 -m unittest discover -s tests -p 'test_*.py'` for SQLite recovery invariants. The isolated live runner additionally verifies a real account password reset and subsequent sign-in with unchanged user/KB identity.
