# HERMES-LLM v0.1.1 — development preview

Install Docker Desktop and Ollama separately, then use the Setup guide. Includes local streaming chat, native document selection and incremental knowledge-base sync using Open WebUI v0.13.5.

This build is ad-hoc signed and is not Apple-notarized. Gatekeeper may require explicit approval. Verify the attached checksum and follow the installation/security documentation. Existing application data and Docker volumes are preserved.

Known limits: manual sync only, no built-in OCR, separate desktop/browser chat histories, no Intel DMG or automatic updater. Review docs/VERIFICATION.md for actual acceptance results.

Version 0.1.1 replaces Keychain with a user-chosen HERMES password and an encrypted local credential vault. Local-account recovery backs up the database and preserves books, chats and user identity. See docs/PASSWORDS.md.

The final review adds interrupted-response and model-download checks, verification that knowledge chat emitted retrieved sources, and wall-clock startup/indexing deadlines. Publication runs the full CI suite and attaches the installer and checksum before making the release visible.

Download `HERMES-LLM_0.1.1_aarch64.dmg` and `SHA256SUMS.txt` into the same folder, then verify with `shasum -a 256 -c SHA256SUMS.txt` before installing. See the [installation guide](https://github.com/JxmesMaherry/HERMES-LLM/blob/main/docs/QUICKSTART.md) and [password recovery guide](https://github.com/JxmesMaherry/HERMES-LLM/blob/main/docs/PASSWORDS.md).
