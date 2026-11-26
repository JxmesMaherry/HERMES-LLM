# Security and privacy

## Trust boundaries

HERMES runs a local client and backend. Docker publishes only `127.0.0.1:<configured port>`. Ollama remains native at `127.0.0.1:11434`; HERMES never sets `OLLAMA_HOST=0.0.0.0`. Docker Desktop connects through `host.docker.internal`. Do not expose either service to the LAN or forward its port.

Open WebUI authentication stays enabled. The first administrator is explicitly created by the user; additional public signup is disabled. Sessions and the random backend application secret are stored in macOS Keychain. JSON preferences and conversation files are private to the user (mode 0600 in a mode-0700 directory). Passwords are not saved. Keychain access errors do not silently generate replacement secrets.

Docker administrators can inspect container environment variables, including the backend secret. Local account access, disk access, Docker access and an unlocked login Keychain are part of the trust boundary. HERMES does not provide encryption at rest for chats/documents; use macOS FileVault and account controls when needed. Keychain tokens can expire; sign in again.

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
4. Keep the Keychain `backend-secret` available. Moving to another Mac without it invalidates existing sessions; sign in again and plan secret rotation carefully. Do not publish exported Keychain data.
5. Restore with the backend stopped and the same supported image version, then verify retrieval and history before upgrading.

Uninstalling the UI leaves model files, source documents, application data and Docker volumes intact. Deliberate erasure is a separate manual operation outside ordinary lifecycle commands.

## Reporting

Do not publish secrets or private examples. Until a private advisory channel is enabled, report reproducible vulnerabilities to the repository owner through an appropriate private contact. GitHub Actions uses read-only permissions except the tag release job’s scoped release upload permission. No signing material belongs in Git.

## Dependency audit status

The 26 November 2026 RustSec audit exited successfully with no vulnerability errors and three upstream warnings in the cross-platform lockfile: `chrono` unmaintained ([RUSTSEC-2026-0543](https://rustsec.org/advisories/RUSTSEC-2026-0543)), `proc-macro-error` unmaintained ([RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370)), and `glib` iterator unsoundness ([RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429)). These are not suppressed. Track upstream Tauri/dependency updates and rerun audits before distributing a release. Passing an advisory database check does not certify the absence of defects.
