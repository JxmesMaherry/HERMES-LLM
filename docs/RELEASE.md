# macOS releases

The release workflow runs the complete reusable CI suite before building an Apple Silicon `.app` and `.dmg` using the official Tauri action on `macos-15`. `workflow_dispatch` builds an Actions artifact without publishing a release. A pushed `v*` tag uploads the verified DMG and SHA-256 file to a draft, checks that both attachments exist, then publishes the GitHub Release. Versions beginning with `0.` are marked as development prereleases. External Actions are pinned to reviewed commit hashes.

A build-only run has passed on GitHub, and its downloaded installer has been verified. See [VERIFICATION.md](VERIFICATION.md) for the run and checksum. The repository is currently private and no release tag has been published. Publishing requires the repository owner’s explicit approval; do not push tags as part of ordinary development.

## Local development distribution

```sh
npm ci
rustup target add aarch64-apple-darwin
npm run dmg
```

Look in:

```text
apps/desktop/src-tauri/target/aarch64-apple-darwin/release/bundle/macos/HERMES-LLM.app
apps/desktop/src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/
```

Validate the actual output paths (the DMG filename includes version/architecture):

```sh
./scripts/verify-bundle.sh \
  'apps/desktop/src-tauri/target/aarch64-apple-darwin/release/bundle/macos/HERMES-LLM.app' \
  'apps/desktop/src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/HERMES-LLM_0.1.1_aarch64.dmg'
shasum -a 256 apps/desktop/src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/*.dmg
```

The verifier checks the executable, embedded frontend index, bundled Compose resource, code signature integrity and disk-image integrity. Signature validity is different from Apple notarization.

For manual install verification, mount the image read-only with `hdiutil attach -readonly <path>`, inspect the app’s resources and drag it into a test Applications directory. Launch it with the repository unavailable to confirm resource independence. Eject the mounted image using Finder. Complete the checks in TESTING.md on a clean macOS account.

## GitHub Actions

After approval to publish, update all four package versions and lockfiles, commit the intended source and create/push a matching tag:

```sh
node scripts/check-version.mjs
git tag -a v0.1.1 -m 'HERMES-LLM 0.1.1'
git push origin v0.1.1
```

To test without publishing a release (requires GitHub CLI or the Actions UI):

```sh
gh workflow run release.yml --ref main
gh run list --workflow release.yml
gh run watch <run-id>
gh run download <run-id> --name HERMES-LLM-macOS-aarch64 --dir ./release-artifacts
(cd release-artifacts && shasum -a 256 -c SHA256SUMS.txt)
```

The workflow validates the tag against package versions, runs all CI checks, builds both app and DMG, verifies required resources and non-empty output, uploads an artifact for 14 days, and publishes only for tag builds. `scripts/publish-release.sh` validates the local checksum before any GitHub mutation. It can resume/replace attachments in an unpublished draft, and refuses to change an already published release. Upload failures leave the release unpublished. Six tests exercise these cases using a fake CLI without contacting GitHub. Actions must be enabled and the intended source/ref must already exist remotely.

For the first public preview, follow [PUBLISHING-v0.1.1.md](PUBLISHING-v0.1.1.md). Do not create an empty published release separately: pushing the tag runs the tested publishing path.

## Ad-hoc signing and future notarization

`bundle.macOS.signingIdentity` is `"-"`, an ad-hoc signature. It requires no paid Apple credentials. This **does not** provide a Developer ID identity or Apple notarization and may trigger Gatekeeper warnings. Do not describe a local successful build as notarized, universally trusted or tested on every supported Mac.

To add proper distribution later:

1. Obtain an Apple Developer ID Application certificate.
2. Save `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD` and `APPLE_SIGNING_IDENTITY` as GitHub Actions secrets.
3. Override the ad-hoc `signingIdentity` using a private/generated Tauri config or change the distribution configuration to use the Developer ID identity. Pass the identity and certificate secrets to the Tauri action through its `env`.
4. Add Tauri-supported notarization credentials: either Apple ID/app-specific password/team ID or the documented App Store Connect API key configuration.
5. Build, notarize and staple; verify with `codesign --verify --deep --strict`, `xcrun stapler validate` and `spctl --assess --type execute` on a clean Mac.

Never commit certificates, API key files, Apple passwords or Keychain exports. See the current [Tauri GitHub pipeline guide](https://v2.tauri.app/distribute/pipelines/github/) and [macOS signing guide](https://v2.tauri.app/distribute/sign/macos/) before adding credentials.
