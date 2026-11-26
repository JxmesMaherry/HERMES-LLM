# Implementation handover

HERMES-LLM 0.1.1 is a working local-first MVP. Its React interface uses Tauri 2 commands implemented in Rust. Native Ollama executes models; a pinned official Open WebUI container supplies authenticated chat, local embeddings, RAG and persistent knowledge storage. A user-password-protected encrypted vault holds credentials and the backend secret. The app bundles the Compose definition and licence notices and keeps writable data outside its `.app` bundle.

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
    PasswordDialog.tsx, AccountRecovery.tsx
    App.test.tsx, bridge.test.ts, test-setup.ts
  src-tauri/
    Cargo.toml, Cargo.lock, build.rs, tauri.conf.json
    capabilities/main.json, icons/
    src/lib.rs, main.rs, config.rs, services.rs, api.rs, sync.rs, error.rs, credentials.rs
    resources/account_recovery.py
infra/docker/
  compose.yaml, .env.example
scripts/
  stress-test.mjs, vault-reader.mjs
  publish-release.sh
  bootstrap.sh, doctor.sh, start.sh, stop.sh, logs.sh, common.sh
  integration.mjs, check-version.mjs, third-party-notices.mjs, verify-bundle.sh
tests/fixtures/hermes-facts.md, tests/stress.test.mjs, tests/stress-cases.example.json
tests/test_account_recovery.py, tests/release.test.mjs
docs/
  ARCHITECTURE.md, INSTALLATION.md, DEVELOPMENT.md, SECURITY.md
  TROUBLESHOOTING.md, RELEASE.md, RELEASE-NOTES.md, TESTING.md
  THIRD-PARTY.md, THIRD-PARTY-LICENSES.txt, upstream-licenses/
  VERIFICATION.md, HANDOVER.md, QUICKSTART.md, HARDWARE.md
  STRESS-TESTING.md, USER-GUIDE.md, PASSWORDS.md
  PUBLISHING-v0.1.1.md
README.md, LICENSE, .gitignore, package.json, package-lock.json, rust-toolchain.toml
```

Generated frontend assets, Rust targets, app/DMG files and native schemas are ignored by Git. The repository was initially empty; existing Git configuration and remote settings were preserved. Focused commits cover infrastructure, the desktop app, CI/DMG packaging, stress testing, public documentation and password recovery. These commits are now pushed to `main` on GitHub. The repository remains private and no release tag has been published.

## Implemented

Prerequisite detection; first-run wizard; known-path Docker integration; safe lifecycle commands; bounded readiness; port-conflict detection; model discovery/selection/download; authenticated streamed chat/cancellation; safe Markdown and code; desktop history; native document picker; knowledge creation/selection; SHA-256 incremental sync; preview/explicit deletion consent; processing/progress/error states; real native RAG; local Open WebUI fallback; preferences/themes; encrypted credential vault; scripts; CI and DMG/release workflow; documentation and MIT licence.

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

DMG output: `apps/desktop/src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/HERMES-LLM_0.1.1_aarch64.dmg`. See RELEASE.md for verification, manual workflow dispatch, tag creation/push and downloading Actions artifacts. A build-only workflow was dispatched and verified. Tag/release publication remains a separate maintainer action.

## Results and limits

See VERIFICATION.md for exact executed checks: frontend build/lint/typecheck, 10 frontend tests, 7 stress-tool contract tests, 6 publication tests, 31 default Rust tests, 4 Python recovery tests, a separately passed real chat/RAG/sync test, Compose validation, native launch and `.app`/DMG generation. npm audit reported no vulnerabilities; RustSec exited 0 with three upstream warnings.

GitHub CI and the build-only release workflow have passed; see VERIFICATION.md for links and the downloaded artifact checksum. CLI tools use the password-protected Rust helper; Keychain is no longer used. The app is ad-hoc signed, not notarized. Desktop and Open WebUI browser chat histories are separate. Sync is manual, OCR is not bundled, empty directories are deliberately retained, and concurrent upstream edits are not an atomic transaction. A crash after upload but before cleanup may require inspection/retry. Limits are 5,000 documents, 20,000 entries, and a configurable maximum of 50 MB per document.

## Public installation and stress testing

README.md now leads with installation steps and a Mac/model workload table. QUICKSTART.md is the short packaged-app guide; HARDWARE.md distinguishes verified hardware from conservative memory estimates and cites current official specifications. STRESS-TESTING.md includes serial/concurrent RAG, isolated Ollama baselines, per-book accuracy cases and monitoring commands. The stress runner uses the selected KB and an encrypted-vault/temporary token; it makes no index or service changes. Heavy load against the owner's eight textbooks has not been run automatically.

## Remaining priorities

1. Decide when to make the prepared repository public and publish the first tagged release; complete clean-Mac/account installation acceptance using TESTING.md.
2. Complete controlled native picker/permission/restart/sleep/offline/backup acceptance and test broader PDF collections.
3. Track and resolve upstream RustSec warnings; review advisories and licence notices before every release.
4. Add Developer ID signing and notarization when credentials are available.
5. Improve interruption reconciliation and concurrent KB edit handling, then consider optional file watching, OCR, Intel builds and an authenticated updater.

## Password-based replacement, version 0.1.1

The Keychain approach is superseded. Users choose a HERMES password in the app; Argon2id and XChaCha20-Poly1305 protect credentials at rest. The app never accesses Keychain. Password reset retains an encrypted vault backup, and local-account recovery backs up SQLite and uses Open WebUI's password-update method while preserving identity and knowledge access. Four Python data-preservation tests and an isolated real backend password-reset/sign-in/RAG test passed. The owner's actual password remains for them to choose in the new screen.
