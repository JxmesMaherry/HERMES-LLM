# Troubleshooting

| Symptom | Action |
| --- | --- |
| Ollama missing | Install the official macOS app. HERMES does not install a runtime. |
| Ollama stopped / unavailable | Open Ollama. Check `curl --noproxy '*' http://127.0.0.1:11434/api/tags`. Keep binding on loopback. |
| Docker installed, daemon stopped | Open Docker Desktop; wait for the engine. Check `docker info`. |
| Docker command absent in GUI | HERMES searches Docker.app, `/usr/local/bin` and `/opt/homebrew/bin`; reinstall official Docker Desktop if none exists. |
| Port conflict | Stop HERMES’s backend. Choose an unused port 1024–65535 in Settings, Save, then Start. The original volume is retained. |
| Backend health fails / first start stalls | Inspect Settings → View logs or `scripts/logs.sh`. Check disk space, Docker memory and image download access. Readiness stops after a bounded wait. |
| Embedding load fails offline | The cache/image may lack MiniLM. Explicitly allow embedding downloads, then Restart. Disable downloads again for offline use. |
| Repeated Keychain prompts in an older build | Quit that HERMES window and open the updated build. Click Unlock credentials once if needed. The updated app caches credentials and never retries Keychain from background refreshes; no books or volumes need resetting. |
| CLI Keychain access denied | Unlock the login Keychain and review the normal macOS request from the trusted `/usr/bin/security` helper. Retry bootstrap only after access is allowed; HERMES does not replace an existing secret on access failure. |
| Chat rejected / token expired | Sign in again in Settings. API requests always require authentication. |
| Model unavailable | Refresh and choose a model listed by Ollama, or explicitly download one. |
| Folder disappeared or macOS denied access | Choose the folder again. Review macOS Files and Folders permissions; do not grant broad access merely to bypass a failed document. |
| Preview expired / source changed | Preview again. A preview is bound to the folder/KB and expires after ten minutes. |
| Upload succeeds but processing fails | Inspect errors. Empty/scanned PDFs may need external OCR. Keep the existing index until processing succeeds. |
| Sync partly failed / interrupted | Old indexed entries are retained. Inspect the KB in Open WebUI, then preview/retry. A crash after upload can leave duplicates. |
| Native document question gives no facts | Confirm the selected KB, successful indexing and Use knowledge base. Test the known-facts fixture. Use the local Open WebUI chat and attach the KB with `#` while investigating native compatibility. |
| Browser-only frontend has no data | Use `npm run desktop`. `npm run dev` intentionally cannot control local services. |
| Restart after sleep | Focus the window or Refresh status. HERMES rechecks without unbounded automatic restart loops. |
| macOS blocks the app | Verify provenance/checksum and use supported Privacy & Security approval if available. See INSTALLATION.md. Never disable Gatekeeper. |

Services stay running after closing HERMES. The explicit Stop action stops only the managed container and preserves all volumes. Docker Desktop and Ollama remain running until you close them yourself.
