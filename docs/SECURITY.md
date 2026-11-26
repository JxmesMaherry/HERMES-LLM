# Security and privacy

## Trust boundaries

HERMES runs a local client and backend. Docker publishes only `127.0.0.1:<configured port>`. Ollama remains native at `127.0.0.1:11434`; HERMES never sets `OLLAMA_HOST=0.0.0.0`. Docker Desktop connects through `host.docker.internal`. Do not expose either service to the LAN or forward its port.

Open WebUI authentication stays enabled. The first administrator is explicitly created by the user; additional public signup is disabled. Sessions and the random backend secret are encrypted in a local vault using a user-chosen HERMES password. Argon2id derives the key; XChaCha20-Poly1305 authenticates and encrypts the credential data. HERMES uses no Keychain APIs and never asks for a Mac password. It caches the unlocked state for the app session. JSON preferences and conversations remain mode 0600 inside a mode-0700 directory. See [password and recovery details](PASSWORDS.md).

Docker administrators can inspect the container environment, including its secret, and operate the backend database. Local account, disk and Docker access are trust boundaries. The credential vault does not encrypt document files or chat stores; use FileVault and account controls as appropriate. Explicit local recovery is available only for HERMES's labelled container/data volume and requires a selected administrator and confirmation. It backs up the database, preserves the user ID and data, and revokes old account sessions. It does not disable authentication or MFA.

## Network policy

Cloud model APIs, direct connections, community access, version checks, web search and telemetry are disabled in the supported Compose configuration. HERMES frontend cannot make arbitrary HTTP requests. Its Rust client bypasses environment proxies and refuses redirects. Remote Markdown images and clickable external links are not rendered.

Explicit network actions are: opening official download pages; starting the backend, which may pull its official container; downloading a chosen Ollama model; and enabling Hugging Face embedding-model downloads. Those requests obtain software/model assets. Documents and prompts go only to loopback services. For strict offline use, cache the image and models first, disable embedding downloads and verify with outbound networking disconnected. Existing backend administrator configuration or an independently modified Docker/Ollama installation remains the operator’s responsibility.

## File and deletion safety

Only the native picker grants a document root. Selecting the entire home or a parent is rejected. Scans exclude hidden/sensitive directories and all symlinks. Upload reads are capability-relative to that root so a replaced symlink cannot escape. Size, entry-count, document-count, type and UTF-8 checks bound work. Files are hashed again after preview to detect changes. Errors reading subfolders abort the scan; an unavailable root never becomes an empty deletion manifest.

Deletions are previewed by name and require a separate explicit checkbox. Preview plans are one-use, expire after ten minutes and are invalidated by settings or folder changes. Failed processing suppresses cleanup; originals are never modified. HERMES retains empty KB directories instead of calling potentially recursive upstream directory removal. Use a dedicated KB per source folder. Concurrent external changes are not a transactional API and need inspection in Open WebUI.

## UI and command security

Markdown raw HTML is skipped and images are omitted; no raw HTML plugin is enabled. CSP restricts scripts, frames, network and resources. Only the main local window receives capabilities. No shell, fs, frontend HTTP or arbitrary opener plugin is exposed. Docker uses known executable paths, explicit arguments, fixed project identity and validated inputs. The UI cannot submit shell strings or arbitrary filesystem roots.

HERMES does not log prompts, documents, passwords or tokens. Open WebUI/Ollama have their own logging; inspect logs locally before sharing because upstream output can contain private metadata. Do not submit private logs as public issues.

## Backup and restore

1. Stop the backend in Settings or run `./scripts/stop.sh`.
2. Copy `~/Library/Application Support/com.jxmesmaherry.hermesllm/` to a private backup location.
3. Back up the named volume using Docker Desktop’s volume export, or an explicitly chosen backup container. Do not run prune or delete volume commands. Backup archives contain private documents and chat data.
4. Back up the encrypted `credentials.vault` and remember its HERMES password. Keep backups private. Moving a vault requires its password; forgotten-password recovery preserves backend data but needs local account reconnection.
5. Restore with the backend stopped and the same supported image version, then verify retrieval and history before upgrading.

Uninstalling the UI leaves model files, source documents, application data and Docker volumes intact. Deliberate erasure is a separate manual operation outside ordinary lifecycle commands.

## Reporting

Do not publish secrets or private examples. Use [GitHub private vulnerability reporting](https://github.com/JxmesMaherry/HERMES-LLM/security/advisories/new) once the maintainer enables it for the public repository. If the private form is unavailable, open an issue asking only for a security contact; do not disclose vulnerability details there. GitHub Actions uses read-only permissions except the tag release job’s scoped release upload permission. External Actions are pinned to reviewed commits, and tagged releases must pass the full CI suite. No signing material belongs in Git.

## Dependency audit status

The 26 November 2026 RustSec audit exited successfully with no vulnerability errors and three upstream warnings in the cross-platform lockfile: `chrono` unmaintained ([RUSTSEC-2026-0543](https://rustsec.org/advisories/RUSTSEC-2026-0543)), `proc-macro-error` unmaintained ([RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370)), and `glib` iterator unsoundness ([RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429)). These are not suppressed. A target-specific `cargo tree` review confirmed that `chrono` and `glib` are absent from the Apple Silicon macOS dependency graph; the cross-platform lockfile still records them. Track upstream Tauri/dependency updates and rerun audits before distributing a release. Passing an advisory database check does not certify the absence of defects.
