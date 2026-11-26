# Publish the first HERMES preview

This is the owner-operated path for making the source public and releasing **v0.1.1**. The workflow runs all tests, builds a fresh installer and attaches its matching checksum. It marks 0.x releases as prereleases because HERMES is a development preview. No public visibility change or tag is performed merely by reading these instructions.

## 1. Use the reviewed source

Confirm that the final review changes are pushed and the latest [CI run](https://github.com/JxmesMaherry/HERMES-LLM/actions/workflows/ci.yml) is green. The final [build-only DMG run](https://github.com/JxmesMaherry/HERMES-LLM/actions/workflows/release.yml) should also be green. [VERIFICATION.md](VERIFICATION.md) records the reviewed commit, tests and downloaded artifact.

The installer is ad-hoc signed, not Apple-notarized. Clean-Mac installation and the remaining manual acceptance cases are listed in [TESTING.md](TESTING.md). Describe this release as a development preview. Private source books, credentials, databases and models are not part of the repository or installer.

## 2. Make the repository public

Open [repository settings](https://github.com/JxmesMaherry/HERMES-LLM/settings), then **General → Danger Zone → Change repository visibility → Public**. Complete GitHub's confirmation for `JxmesMaherry/HERMES-LLM`. This exposes the source history and Actions history/logs. See [GitHub's visibility instructions](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/managing-repository-settings/setting-repository-visibility).

Open the repository in a signed-out/private browser window to confirm that the README, licence and contribution guide are visible.

## 3. Enable private security reports

In repository **Settings → Security and quality → Advanced Security**, enable **Private vulnerability reporting**. This makes the private reporting link in the security policy usable. See [GitHub's reporting setup](https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/configure-vulnerability-reporting/configure-for-a-repository).

## 4. Push the release tag

In Terminal, change into your HERMES-LLM clone. Start with a clean working tree and the reviewed `main`:

```sh
git switch main
git pull --ff-only origin main
git status --short
node scripts/check-version.mjs v0.1.1
git tag -a v0.1.1 -m 'HERMES-LLM 0.1.1 development preview'
git push origin v0.1.1
```

If `git status --short` lists changes, resolve them before tagging. If the tag already exists, inspect it rather than deleting, moving or force-pushing it.

Pushing the tag starts the [macOS DMG workflow](https://github.com/JxmesMaherry/HERMES-LLM/actions/workflows/release.yml). It runs the full CI suite before packaging. After verifying the installer, it creates or resumes an unpublished draft, uploads exactly the DMG and checksum, checks the attachments, and publishes the prerelease. Do not also create a separate published release in the web UI.

If an upload fails, the draft stays unpublished. Fix the cause and rerun the failed workflow. Already published releases are not overwritten by this script.

## 5. Verify the public download

Wait for every job in the tag workflow to pass, then open [v0.1.1](https://github.com/JxmesMaherry/HERMES-LLM/releases/tag/v0.1.1). Its download assets should be:

- `HERMES-LLM_0.1.1_aarch64.dmg`
- `SHA256SUMS.txt`

Download both into the same new folder. In Terminal, change into that folder and run:

```sh
shasum -a 256 -c SHA256SUMS.txt
hdiutil verify HERMES-LLM_0.1.1_aarch64.dmg
```

The checksum must report **OK**. Use the checksum attached to this exact release: separate packaging runs can produce different DMG hashes. Verify installation and startup on a separate Mac/account before presenting broader compatibility as tested.

Share the [releases page](https://github.com/JxmesMaherry/HERMES-LLM/releases) and [installation guide](QUICKSTART.md). Users install Docker Desktop and Ollama separately; they choose their own HERMES password and local account.
