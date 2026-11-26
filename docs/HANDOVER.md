# Implementation handover

HERMES-LLM 0.1.0 is a working local-first MVP. Its React interface uses Tauri 2 commands implemented in Rust. Native Ollama executes models; a pinned official Open WebUI container supplies authenticated chat, local embeddings, RAG and persistent knowledge storage. Keychain holds credentials and the backend secret. The app bundles the Compose definition and licence notices and keeps writable data outside its `.app` bundle.

## Created files

```text
.github/workflows/
  ci.yml, release.yml
apps/desktop/
  package.json, tsconfig.json, vite.config.ts, eslint.config.js, index.html
  public/icon.svg
  src/
    App.tsx, Dashboard.tsx, Setup.tsx, Chat.tsx, Knowledge.tsx, Settings.tsx
    bridge.ts, types.ts, components.tsx, main.tsx, styles.css
    App.test.tsx, bridge.test.ts, test-setup.ts
  src-tauri/
    Cargo.toml, Cargo.lock, build.rs, tauri.conf.json
    capabilities/main.json, icons/
    src/lib.rs, main.rs, config.rs, services.rs, api.rs, sync.rs, error.rs, credentials.rs
infra/docker/
  compose.yaml, .env.example
scripts/
  stress-test.mjs
  bootstrap.sh, doctor.sh, start.sh, stop.sh, logs.sh, common.sh
  integration.mjs, check-version.mjs, third-party-notices.mjs, verify-bundle.sh
tests/fixtures/hermes-facts.md, tests/stress.test.mjs, tests/stress-cases.example.json
docs/
  ARCHITECTURE.md, INSTALLATION.md, DEVELOPMENT.md, SECURITY.md
  TROUBLESHOOTING.md, RELEASE.md, RELEASE-NOTES.md, TESTING.md
  THIRD-PARTY.md, THIRD-PARTY-LICENSES.txt, upstream-licenses/
  VERIFICATION.md, HANDOVER.md, QUICKSTART.md, HARDWARE.md
  STRESS-TESTING.md, USER-GUIDE.md
README.md, LICENSE, .gitignore, package.json, package-lock.json, rust-toolchain.toml
```

Generated frontend assets, Rust targets, app/DMG files and native schemas are ignored by Git. The repository was initially empty; existing Git configuration and remote settings were preserved. The implementation is recorded in focused local commits for infrastructure, the desktop app, CI/DMG packaging, stress testing, and public documentation. No push or release publication was performed.

## Implemented

Prerequisite detection; first-run wizard; known-path Docker integration; safe lifecycle commands; bounded readiness; port-conflict detection; model discovery/selection/download; authenticated streamed chat/cancellation; safe Markdown and code; desktop history; native document picker; knowledge creation/selection; SHA-256 incremental sync; preview/explicit deletion consent; processing/progress/error states; real native RAG; local Open WebUI fallback; preferences/themes; Keychain; scripts; CI and DMG/release workflow; documentation and MIT licence.

## Dependencies users install

Packaged app: Docker Desktop and Ollama, plus a deliberately selected model. Source builds additionally need Node.js 22.12+ (24 recommended), Rust stable and Apple Command Line Tools. No paid AI or cloud account is required. Docker Desktop and model licences apply independently.

## Commands

From a clone:

```sh
npm ci
./scripts/bootstrap.sh
./scripts/doctor.sh
npm run desktop
```

CLI backend management:

```sh
./scripts/bootstrap.sh --allow-downloads # Explicit optional embedding-download consent
./scripts/start.sh
./scripts/logs.sh
./scripts/stop.sh
```

Local macOS distribution:

```sh
rustup target add aarch64-apple-darwin
npm run dmg
```

For this development session only, the temporary toolchain can be used without a system installation:

```sh
PATH=/tmp/hermes-cargo/bin:$PATH CARGO_HOME=/tmp/hermes-cargo RUSTUP_HOME=/tmp/hermes-rustup npm run desktop
```

DMG output: `apps/desktop/src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/HERMES-LLM_0.1.0_aarch64.dmg`. See RELEASE.md for verification, manual workflow dispatch, tag creation/push and downloading Actions artifacts. Those publishing commands require explicit maintainer approval; none were executed here.

## Results and limits

See VERIFICATION.md for exact executed checks: frontend build/lint/typecheck, 7 frontend tests, 7 stress-tool contract tests, 27 default Rust tests, a separately passed real chat/RAG/sync test, Compose validation, native launch and local `.app`/DMG generation. npm audit reported no vulnerabilities; RustSec exited 0 with three upstream warnings.

The release workflow is implemented but has not run on GitHub. CLI bootstrap requires allowing the standard macOS Keychain request from the trusted `security` helper; that access was denied during verification, and the full CLI secret-read path remains a manual check. The app is ad-hoc signed, not notarized. Desktop and Open WebUI browser chat histories are separate. Sync is manual, OCR is not bundled, empty directories are deliberately retained, and concurrent upstream edits are not an atomic transaction. A crash after upload but before cleanup may require inspection/retry. Limits are 5,000 documents, 20,000 entries, and a configurable maximum of 50 MB per document.

## Public installation and stress testing

README.md now leads with installation steps and a Mac/model workload table. QUICKSTART.md is the short packaged-app guide; HARDWARE.md distinguishes verified hardware from conservative memory estimates and cites current official specifications. STRESS-TESTING.md includes serial/concurrent RAG, isolated Ollama baselines, per-book accuracy cases and monitoring commands. The stress runner uses the selected KB and a Keychain/temporary token; it makes no index or service changes. Heavy load against the owner's eight textbooks has not been run automatically.

## Remaining priorities

1. Run CI and a manual release workflow after the source is approved/pushed; verify the downloaded DMG on a clean Mac/account using TESTING.md.
2. Complete controlled native picker/permission/restart/sleep/offline/backup acceptance and test broader PDF collections.
3. Track and resolve upstream RustSec warnings; review advisories and licence notices before every release.
4. Add Developer ID signing and notarization when credentials are available.
5. Improve interruption reconciliation and concurrent KB edit handling, then consider optional file watching, OCR, Intel builds and an authenticated updater.

## Keychain prompt-loop fix

A separate fix commit consolidates secrets into one protected vault, suppresses all startup/background authentication dialogs, caches credentials for the session and provides explicit Unlock/retry. Migration preserves the existing backend secret and indexed data. Tests cover 100 repeated cached checks, denied-access latching, migration, sign-out persistence, corrupt credentials and UI refreshes without automatic unlock.
