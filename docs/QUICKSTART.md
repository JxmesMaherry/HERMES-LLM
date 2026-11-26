# Install HERMES-LLM

You need an Apple Silicon Mac, preferably with at least 16 GB memory, and a macOS version supported by Docker Desktop and Ollama. HERMES’s own app target is macOS 13+, but its prerequisites can require newer versions. Check [Docker’s requirements](https://docs.docker.com/desktop/setup/install/mac-install/) and [Ollama’s requirements](https://docs.ollama.com/macos).

## 1. Install the two prerequisites

Download the official macOS applications:

- [Ollama](https://ollama.com/download/mac), which runs the AI model.
- [Docker Desktop for Apple Silicon](https://www.docker.com/products/docker-desktop/), which runs the document/chat backend. Its product licensing applies independently.

Open both. Wait until Docker’s engine is running. HERMES never installs either application for you.

## 2. Download HERMES

Open [GitHub Releases](https://github.com/JxmesMaherry/HERMES-LLM/releases) and download the Apple Silicon DMG and its `SHA256SUMS.txt`. Downloads will appear after the maintainer publishes the first release.

Optional Terminal verification, from the directory containing both downloaded files:

```sh
shasum -a 256 -c SHA256SUMS.txt
```

The filename must match the checksum file; a browser-added suffix such as `(1)` must be accounted for. Continue only if the checksum matches the trusted release attachment.

## 3. Install and open

Open the DMG and drag **HERMES-LLM** into **Applications**. Launch HERMES.

The development build is ad-hoc signed and **not Apple-notarized**. macOS may block it. After verifying the download, use **System Settings → Privacy & Security → Open Anyway** if offered. If no supported approval path is available, build from source or await a notarized release. Do not disable Gatekeeper or remove quarantine attributes.

## 4. Follow the Setup guide

1. Check that Docker and Ollama are running.
2. Start the local Open WebUI backend. Its official container may download on first start. Allow the optional embedding-model download only if you need it and consent to it.
3. Create the first local account or sign in to an existing one. This account belongs to your local backend and requires no paid service. HERMES encrypts its saved session using the HERMES password you choose.
4. Choose an installed model. On a 16 GB Mac, start with `qwen3:4b-instruct`; use the explicit Download button if it is absent.
5. Select a document folder and create/select a knowledge base.

No Git clone, Node.js, Rust, API endpoint editing or `.env` file is needed for the packaged app.

## 5. Ask about your documents

Open **Library → Preview changes → Sync now**. Wait for indexing and review any failures. Then open **Chat**, enable **Use knowledge base** and ask a question whose answer you know from a source document.

Use your own PDF, TXT or Markdown files. A scanned PDF may require external OCR. Manual sync picks up later changes; removing indexed content requires explicit confirmation. Originals remain on your Mac.

Closing HERMES leaves services running. Use **Settings → Stop** to stop the backend without deleting data. For help, read [troubleshooting](TROUBLESHOOTING.md), [hardware guidance](HARDWARE.md) and [privacy/backup information](SECURITY.md).


First choose a HERMES password in the app (at least 12 characters). It is not your Mac password. For a forgotten local account password, use the Settings recovery flow described in [PASSWORDS.md](PASSWORDS.md); books and chats are preserved.
