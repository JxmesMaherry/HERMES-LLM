# Verification record

Current password-based preview: **0.1.1**, 26 November 2026, Apple Silicon macOS. No source was pushed, no release was published, and no GitHub workflow was dispatched.

## Executed checks

| Check | Actual result |
| --- | --- |
| Frontend tests | 10 passed, including in-app password creation/unlock, explicit saved-sign-in reset and confirmed local-account recovery |
| Stress-tool tests | 7 passed for bounded concurrency, timeouts, private metrics and authenticated retrieval event checks |
| Rust tests | 25 passed; one real-backend test ignored by default and executed separately |
| Encrypted vault | Round-trip persistence, wrong password, tampering, hostile KDF parameters, weak-password rejection, cached reads, backup-on-reset and preservation of fixture documents passed |
| SQLite recovery | 4 Python tests passed: unchanged user/KB/chat identity, original database backup, safe administrator listing, and parameterised updates rejecting unknown/injected addresses |
| Real password reset | Isolated Open WebUI v0.13.5 administrator recovered; sign-in with the new password succeeded; user ID and existing knowledge access were unchanged |
| Real RAG/incremental sync | Isolated native client retrieved AMBER-7319 without the code in its question, then passed unchanged/modified/declined-deletion/confirmed-cleanup cases |
| Frontend checks | Formatting, ESLint, strict TypeScript and production build passed; final packaging reruns the build |
| Native checks | Rust formatting, locked compilation and Clippy with all targets and warnings denied passed |
| Keychain removal | No Keychain API or /usr/bin/security invocation remains in app or CLI credential code; old Keychain entries are not read, changed or deleted |
| Dependency audit | Cargo audit completed without vulnerability errors; upstream lockfile warnings remain documented in SECURITY.md |
| Docker/shell/version checks | Compose configuration, shell syntax and the four package versions are checked locally |

The live recovery/RAG test used the isolated project `hermes-acceptance-7056d84e` and its own named data volume. Its container was stopped and its volume retained. Only invented public fixture facts were uploaded. The owner's books, real account password and production database were not reset by the agent. The user chooses their own password in the updated app.

The earlier 0.1.0 implementation used Keychain, and its memory-cache change did not solve the owner's unknown-Keychain-password problem. Version 0.1.1 supersedes that design with an encrypted local file and a user-chosen password. Old Keychain approval instructions are not required by this build.

## Remaining manual/external checks

- The owner must choose a new HERMES password in the app. If the local AI account password is forgotten too, use the backed-up account recovery screen.
- A clean Mac/account install, minimum supported macOS compatibility, sleep/resume, long collections, large/scanned PDFs, full disconnected/offline use and backup/restore acceptance.
- Heavy/concurrent load and per-book correctness checks against the owner's eight textbooks. Commands are provided; this workload was not run automatically.
- Actual GitHub CI/release execution and download/installation of its artifact.
- Developer ID signing and notarization. Local builds are ad-hoc signed only.

## Final 0.1.1 package

The final Apple Silicon `.app` and DMG built successfully. Bundle verification confirmed the ARM64 executable, embedded frontend, bundled Compose definition and licence resources, ad-hoc signature integrity and valid DMG checksums. The package's Info.plist reports 0.1.1, and its bundled Compose message references the encrypted vault. The CLI helper returns `HERMES_VAULT_CLI_V1` without opening a window.

The finished app was restarted and inspected through the native accessibility tree and screenshot. It displayed **Choose your HERMES password**, both new-password fields, the save button and the existing-setup recovery link. No Keychain prompt appeared during launch or refresh. Docker Desktop, native Ollama and Open WebUI v0.13.5 were responding. The password fields were left empty for the owner; no actual owner credentials were changed.

Local artifact: `.artifacts/HERMES-LLM_0.1.1_aarch64.dmg` (ignored by Git). SHA-256:

```text
d11619594616b4862c43783e3861c8e1b6fbf7d9b2e8ee58182dbc6fe0574a78
```

The final test rerun passed 10 frontend, 7 stress-tool, 25 default Rust and 4 Python tests. Loopback mock-server tests required execution outside the filesystem/network sandbox; the sandboxed attempts failed to bind a port, then the same tests passed with that restriction lifted. The live recovery/RAG result above was run separately against the isolated backend.

See PASSWORDS.md, TESTING.md and RELEASE.md for repeatable procedures.
