# Contributing to HERMES-LLM

HERMES is an Apple Silicon macOS development preview. Start with the [README](README.md), [architecture](docs/ARCHITECTURE.md) and [development guide](docs/DEVELOPMENT.md). Use GitHub issues for reproducible bugs and focused feature proposals. For security concerns, follow the [security policy](docs/SECURITY.md) instead of opening a public issue.

## Run locally

Install Node.js 22.12+ (24 recommended), Rust stable, Apple Command Line Tools, Docker Desktop and Ollama. Clone the repository, then run:

```sh
npm ci
npm run desktop
```

Complete setup inside HERMES and choose your own password. Model downloads and live document uploads are explicit actions. Use invented, disposable documents when developing retrieval or sync changes; the public fixture is in `tests/fixtures/`.

## Submit a change

Create a branch, make focused commits and open a pull request describing the problem, resulting behavior and checks you ran. Include a reproducible example when fixing a bug. Discuss substantial changes in an issue before implementing them. Keep discussions respectful and specific.

Preserve authentication, loopback-only networking, document-root restrictions and explicit deletion consent. Add meaningful tests when changing API contracts, credential handling, recovery, file access or deletion behavior. Keep documentation and third-party notices current when changing dependencies or behavior.

Before opening a pull request, run:

```sh
npm run format:check
npm run lint
npm run typecheck
npm test
npm run build
python3 -m unittest discover -s tests -p 'test_*.py'
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check
cargo clippy --locked --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path apps/desktop/src-tauri/Cargo.toml
node scripts/check-version.mjs
```

The Rust tests require macOS and the mock-server tests need permission to bind to loopback. The separately documented [live acceptance test](docs/TESTING.md) starts an isolated Docker backend; run it deliberately when changing backend integration. A normal test run does not use your real textbooks or reset your account. See [stress testing](docs/STRESS-TESTING.md) for optional load measurements.

## Share safe reproductions

Do not commit credentials, `.env` files, vaults, databases, private documents, textbook content, model weights or built installers. Remove tokens, account addresses, document names and private filesystem paths from logs and screenshots before sharing. Bug reports should include the app/macOS versions, chip and memory, model name, relevant service versions and reproduction steps.

Contributions are provided under the repository's [MIT licence](LICENSE). Upstream dependencies and models retain their own terms; see [third-party notices](docs/THIRD-PARTY.md).
