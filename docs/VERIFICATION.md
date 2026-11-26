# Verification record

Date: 26 November 2026. Actual execution on an Apple Silicon Mac with Docker Desktop, native Ollama and the installed `qwen3:4b-instruct` model. No GitHub workflow was dispatched and no source or release was pushed/published.

## Executed successfully

| Check | Result |
| --- | --- |
| Reproducible npm install | `npm ci --offline` succeeded using the populated local cache |
| ESLint | Passed with supported ESLint 10 |
| TypeScript strict check | Passed |
| Stress-tool contract tests | 7 passed: input limits, RAG request format, bounded concurrency, early stopping, metrics privacy, SSE/UTF-8/source checks and timeouts |
| Stress-tool live smoke | 1 real Ollama request passed, 32-token limit, 2,084 ms total and 1,433 ms to first visible answer text; a smoke check, not a representative benchmark |
| Stress-tool dry runs | Ollama and selected-knowledge plans succeeded without requests or Keychain reads |
| Frontend tests | 6 passed: overlapping refresh protection, honest prerequisite states, setup gating, deletion consent, streamed chat/history and unavailable services |
| Production frontend build | Passed, bundled local assets |
| Rust formatting | `cargo fmt --check` passed |
| Rust lint | `cargo clippy --locked --all-targets -- -D warnings` passed |
| Default Rust suite | 21 passed; 1 live test deliberately ignored by default |
| Live Rust acceptance | Separately executed ignored test passed against real Open WebUI v0.13.5 and native Qwen |
| Actual document retrieval | Public fixture was indexed; streamed answer included `AMBER-7319` without that code in the prompt |
| Live incremental sync | First addition, unchanged second preview, modified replacement, declined deletion and confirmed cleanup all passed |
| Docker configuration | `docker compose config --quiet` passed with a validation-only secret |
| CLI bootstrap / doctor | Temporary-path config creation and doctor passed; bootstrap secret lookup was blocked by Keychain access and was not completed |
| Shell syntax / version consistency | All scripts parsed; all four package versions match 0.1.0 |
| npm audit | No vulnerabilities reported |
| RustSec audit | Exit 0, no vulnerability errors; three upstream warnings described below |
| Native app | Built, launched and observed through macOS accessibility and a screenshot; connected backend, Qwen streaming and rendered Markdown/code were visible |
| Local DMG | Real Apple Silicon `.app` and `.dmg` produced by Tauri; signature/resource/frontend, bundled licence texts and disk-image integrity checks passed for the resource-complete build |
| Redistribution notices | 428 npm/Rust production/build packages, all with supplied licence text or pinned upstream fallback notices |

The live acceptance runner used a unique test Compose project and separate volume. Its container was stopped afterward. The volume `hermes-acceptance-91f6d5ac-data` was intentionally preserved. It contains only public invented test facts and local test account data. No personal source directory was scanned by the automated acceptance test. No production HERMES volume was removed or reset.

The Rust toolchain and cargo-audit used for verification were installed only under `/tmp/hermes-cargo` and `/tmp/hermes-rustup`. No system-wide Rust installation was made. Source-code users still need to install/locate their own Rust toolchain.

## Audit warnings

The RustSec cross-platform lockfile check reported `chrono` (unmaintained, RUSTSEC-2026-0543), `proc-macro-error` (unmaintained, RUSTSEC-2024-0370), and `glib` (iterator unsoundness, RUSTSEC-2024-0429). Audit warnings remain visible; no advisory was ignored. See SECURITY.md. npm audit reported zero vulnerabilities at execution time.

## Awaiting manual or external verification

- Heavy/concurrent stress and per-book correctness checks against the owner’s eight textbooks; commands are provided in STRESS-TESTING.md.
- Actual GitHub CI and release workflow runs, downloadable remote artifact, and tag-to-release publication.
- Installing the final DMG on a different clean macOS account/Mac, including Gatekeeper behaviour and minimum macOS version.
- Complete first-run native folder-picker, permissions and app restart acceptance as a controlled checklist on that clean account.
- Large real PDF collections, scanned/empty PDFs, long-running model downloads and recovery from laptop sleep/network interruption.
- Full offline testing with networking disconnected, backup/restore and long-running concurrent edits through Open WebUI.
- Developer ID signing and Apple notarization. The local build is ad-hoc signed only.

The implementation and mock/live contract results are evidence for the supported MVP, not a claim that every external/manual acceptance case has run. See TESTING.md for the repeatable checklist.

## CLI Keychain verification limit

A CLI bootstrap check in `/tmp/HERMES CLI verification with spaces` created configuration safely, but macOS denied the command-line `security` helper access to the existing backend-secret entry. The bootstrap did not replace that secret; the duplicate add was rejected. The helper now distinguishes missing entries from denied access and fails with recovery guidance instead of attempting to create a secret after access denial. The desktop Keychain flow was observed working; completing the CLI access prompt is a manual acceptance item. `doctor.sh` in the same temporary configuration subsequently reported all installed/running service checks successfully.

## Local commit handover

The owner subsequently requested focused development commits. Local commits now cover backend infrastructure, desktop/chat/RAG, CI/DMG packaging, bounded stress testing, and the public documentation. No remote push, visibility change or release publication was performed.
