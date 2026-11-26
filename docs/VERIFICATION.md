# Verification record

Current password-based preview: **0.1.1**, 26 November 2026, Apple Silicon macOS. Source commits are now pushed to GitHub `main`; CI and a build-only installer workflow passed. The repository remains private and no tagged release has been published.

## Executed checks

| Check | Actual result |
| --- | --- |
| Frontend tests | 10 passed, including in-app password creation/unlock, explicit saved-sign-in reset and confirmed local-account recovery |
| Stress-tool tests | 7 passed for bounded concurrency, timeouts, private metrics and authenticated retrieval event checks |
| Publication tests | 6 passed: checksum validation, draft-first uploads, upload/missing-asset failures, draft resumption and published-release protection |
| Rust tests | 31 passed after final review; one real-backend test ignored by default and executed separately |
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
- Installation acceptance of the GitHub-built artifact on a clean Mac/account; tagged release publication.
- Developer ID signing and notarization. Local builds are ad-hoc signed only.

## Initial 0.1.1 package, before final review

The final Apple Silicon `.app` and DMG built successfully. Bundle verification confirmed the ARM64 executable, embedded frontend, bundled Compose definition and licence resources, ad-hoc signature integrity and valid DMG checksums. The package's Info.plist reports 0.1.1, and its bundled Compose message references the encrypted vault. The CLI helper returns `HERMES_VAULT_CLI_V1` without opening a window.

The finished app was restarted and inspected through the native accessibility tree and screenshot. It displayed **Choose your HERMES password**, both new-password fields, the save button and the existing-setup recovery link. No Keychain prompt appeared during launch or refresh. Docker Desktop, native Ollama and Open WebUI v0.13.5 were responding. The password fields were left empty for the owner; no actual owner credentials were changed.

Local artifact: `.artifacts/HERMES-LLM_0.1.1_aarch64.dmg` (ignored by Git). SHA-256:

```text
d11619594616b4862c43783e3861c8e1b6fbf7d9b2e8ee58182dbc6fe0574a78
```

That test rerun passed 10 frontend, 7 stress-tool, 25 default Rust and 4 Python tests. Loopback mock-server tests required execution outside the filesystem/network sandbox; the sandboxed attempts failed to bind a port, then the same tests passed with that restriction lifted. The live recovery/RAG result above was run separately against the isolated backend.

See PASSWORDS.md, TESTING.md and RELEASE.md for repeatable procedures.

## GitHub and open-source preparation

On 26 November 2026, the implementation history was pushed to [JxmesMaherry/HERMES-LLM](https://github.com/JxmesMaherry/HERMES-LLM). Contribution instructions, bug/feature templates, a PR template, a CI badge, repository description and discovery topics are in place. The MIT licence, hardware/model guide, password recovery guide and stress commands are included. The repository is still private; public visibility and the first tagged release remain future maintainer actions.

A local review of 157 historical file versions before the first push found no textbook/document files, databases, vaults or common credential patterns. This was a targeted history check, not a guarantee that every possible sensitive value or vulnerability can be detected. Private application data, generated installers and model files are excluded from Git.

[GitHub CI](https://github.com/JxmesMaherry/HERMES-LLM/actions/runs/45816252142) passed on commit `c34ace4`: frontend checks and tests, Python account-recovery tests, dependency audit, Compose/shell validation, Rust formatting, Clippy, native tests and RustSec audit. [The macOS build-only workflow](https://github.com/JxmesMaherry/HERMES-LLM/actions/runs/45816251981) also passed and uploaded artifact `HERMES-LLM-macOS-aarch64` (artifact ID `15116625806`, retained until 10 December 2026). Release publication was skipped because this was a manual build rather than a tag push.

The first build exposed an artifact-upload configuration error: the upload action excluded the hidden staging directory. Commit `c34ace4` fixed the upload by including hidden paths and restricting the upload to the DMG and checksum file. The successful rerun verified the ARM64 application bundle, embedded frontend/resources, ad-hoc signature and DMG integrity on the hosted Mac.

The uploaded ZIP was downloaded through the GitHub connector. Its SHA-256 matched GitHub's artifact digest; its two expected files were extracted, the DMG matched its attached checksum, and `hdiutil verify` passed locally. The downloaded files are retained under `.artifacts/github-45816251981/`, ignored by Git. The GitHub-built DMG SHA-256 is:

```text
c40912568de218a73d8a7e46c7996f6f18a287027e71fe89d4167e5051332b9a
```

The locally built DMG checksum above and this hosted-build checksum identify separate packaging runs; byte-for-byte reproducible packaging is not claimed. The downloaded artifact was checked for integrity but was not installed over the owner's active application or represented as clean-Mac acceptance.

## Final pre-publication review

The review found and corrected incomplete stream handling, polling-based deadlines and publication before asset attachment. Native chat now requires a completion marker and non-empty answer; knowledge chat additionally requires an actual retrieval-source event. Model downloads require Ollama's explicit success status. Startup and indexing deadlines include time spent waiting on HTTP requests. Vault loading rejects non-regular files and bounds password input.

Release workflows now use pinned Action commits and call the full CI suite before packaging. The publisher verifies the attached checksum locally, uploads both files to a draft, confirms the attachment names and only then publishes. Draft retries can replace their attachments; published releases are left untouched. Versions beginning with 0 are marked as prereleases. The six mocked publication tests do not make remote mutations.

Local final-review results: 10 frontend tests, 7 stress tests, 6 publication tests, 31 Rust tests and 4 Python tests passed, along with formatting, lint, strict types, production frontend build, Clippy and fresh dependency audits. npm reported zero vulnerabilities. RustSec retained the same three documented warnings; `cargo tree --target aarch64-apple-darwin` showed no `chrono` or `glib` dependency in this target.

The live native recovery/RAG/sync test passed again against isolated project `hermes-acceptance-e5ae95cd`, using only invented fixtures. Its container was stopped and its volume retained. Real password recovery, subsequent sign-in, identity/knowledge preservation, retrieval sources, unchanged/modified sync and declined/confirmed cleanup passed. Production textbooks and account credentials were not changed.

The final publishing path is documented in [PUBLISHING-v0.1.1.md](PUBLISHING-v0.1.1.md). Notarization, clean-Mac acceptance and the other manual cases above remain explicit limits on the preview.
