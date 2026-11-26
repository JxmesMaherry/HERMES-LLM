# Development

Install Node 22.12+ (24 recommended), Rust stable, Apple Command Line Tools, Python 3 (for recovery tests), Docker Desktop and Ollama. Full Xcode may be needed for signing/distribution features on some systems. No system dependency is installed by HERMES scripts.

```sh
npm ci
./scripts/bootstrap.sh
./scripts/doctor.sh
npm run desktop
```

The bundled Compose file is copied by the native application into its application data directory. Tauri resolves resources for both development and packaged builds. Do not run the debug binary directly without Tauri’s resource layout.

Use `npm run dev` for frontend styling only; it exposes no native operations. Browser mode clearly reports that desktop integration is unavailable.

## Checks

```sh
npm run format:check
npm run lint
npm run typecheck
npm test
npm run build
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check
cargo clippy --locked --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path apps/desktop/src-tauri/Cargo.toml
for script in scripts/*.sh; do bash -n "$script"; done
WEBUI_SECRET_KEY=validation-only docker compose -f infra/docker/compose.yaml config --quiet
node scripts/check-version.mjs
```

Tests bind ephemeral loopback ports for mocks; a restrictive sandbox may need explicit loopback access. They never contact cloud AI services, scan your home or delete Docker volumes. Live tests are ignored by default and require explicit configuration; see TESTING.md.

## Dependency changes

Commit `package-lock.json` and `Cargo.lock`. Use `npm ci` and Cargo’s `--locked` in CI. Audit npm dependencies and RustSec advisories. Review Open WebUI security updates and run integration tests before updating the pinned image. The original SVG icon lives in `apps/desktop/public/icon.svg`; generate bundle icons with:

```sh
cd apps/desktop
npx tauri icon public/icon.svg --output src-tauri/icons
```

Only macOS bundle assets need to be retained. Keep dependencies local to the project, prefer narrow native commands and do not introduce frontend shell or arbitrary HTTP access.

## Local load and textbook tests

Run `node scripts/stress-test.mjs --help` for the bounded API runner. `npm test` also executes its seven Node contract tests with loopback mocks. Follow STRESS-TESTING.md for authenticated knowledge requests, monitoring, per-book expected facts and metric-only reports. The stress tool is a source development tool; it is not bundled as an additional runtime in the DMG.

Run `python3 -m unittest discover -s tests -p 'test_*.py'` to verify recovery database/data invariants. Source CLI credential access uses `scripts/vault-reader.mjs` and the built Rust helper; it asks for the chosen HERMES password and never accesses Keychain.
