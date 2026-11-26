# Installation

## Packaged application

Target: Apple Silicon with a macOS version supported by installed Docker Desktop and Ollama. HERMES’s own bundle minimum is macOS 13, which does not override those prerequisites. Download the DMG from the repository’s GitHub Releases when a build has been published. Compare its SHA-256 with the attached `SHA256SUMS.txt`, open the disk image and drag HERMES-LLM to Applications.

Install Ollama and Docker Desktop separately using official installers. Launch both and wait for Docker’s engine. HERMES never installs them, bundles them or asks for administrator access.

Development DMGs are ad-hoc signed and not notarized. macOS may require an explicit **Open Anyway** in Privacy & Security after you verify the download. If macOS provides no supported way to allow a verified build, build from source or await a notarized release. Do not disable security controls or strip quarantine metadata.

## First run

1. Use **Setup guide → Check your Mac**. Missing, stopped and failed services have different statuses.
2. In **Connect the backend**, allow embedding downloads if needed, and click **Start local backend**. This action may download Open WebUI’s official image. First launch can take several minutes. A bounded timeout shows an error instead of claiming readiness.
3. Create the first local administrator account, or sign in to an existing one. This is an account in your local Open WebUI, not a paid cloud service. HERMES stores its session in Keychain and discards the password after use.
4. Select an installed Ollama model. To download one, expand **Download a model**, review the model name and click the explicit internet download button.
5. Choose a dedicated document folder with the macOS picker. Create a KB or select one you can write to.
6. Finish setup, open **Library**, preview changes and sync. Watch for failed processing.
7. Open **Chat**, select the model and enable **Use knowledge base** for document questions.

For retrieval verification, copy `tests/fixtures/hermes-facts.md` into a temporary document directory, select it, sync, then ask **What is the departure code of the vessel Aster?** The expected answer includes `AMBER-7319`. Keep the fact itself out of the question.

## Existing installation and upgrades

The project and volume identities stay stable. Stop the backend before replacing the app or changing ports. Back up both HERMES’s application data directory and the Docker data volume. A newer image may migrate its database; never downgrade blindly. No migration or reset deletes source documents.

The app requires its pinned backend API version. Reusing a different Open WebUI installation is not supported as a substitute for the managed service. Settings exposes the local port and original Open WebUI interface for inspection.

If saved credentials are locked, click **Unlock credentials** once. HERMES keeps them in memory for the session and does not ask again during background checks or chat. A changed ad-hoc build can need fresh macOS approval. Your documents and backend data remain intact.
