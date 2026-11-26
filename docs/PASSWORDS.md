# HERMES passwords and recovery

HERMES 0.1.1 removes the application's macOS Keychain dependency. A macOS dialog mentioning “Security” or a Keychain item is separate from the HERMES password. Cancel an old Keychain dialog and close older HERMES builds; the updated app does not query those entries. Do not reset your entire macOS Keychain for this app.

## Set a password you know

1. Open the 0.1.1 build. Its own window displays **Choose your HERMES password**.
2. Enter a new password of at least 12 characters and confirm it. This is a password you choose, rather than your Mac login password.
3. Click **Save HERMES password**. Docker Desktop must be running so HERMES can preserve the existing backend secret from its managed container.
4. In Settings, connect your existing local Open WebUI account. The password you originally created for that account may still work; it was not the password requested by macOS Keychain.

The saved session and backend secret are encrypted in `credentials.vault` under HERMES application data. Unlock it once when you open HERMES; refreshes, chat and document operations reuse the unlocked session. The password is not saved in configuration or Keychain.

## Forgot the local AI account password

After unlocking HERMES, open **Settings → Forgot your local account password?**. Select the local administrator whose account owns your books. Choose and confirm a new password (you can use the same one you chose for HERMES), check the recovery confirmation and click **Reset password and sign in**.

Recovery runs only against HERMES's labelled container and named local data volume. It makes a SQLite backup first, then uses the pinned backend's password-update method. It keeps the same user ID, books, chats and knowledge permissions, and revokes the account's old sessions. It does not delete `webui.db`, documents, models or Docker volumes. Backups are private files in `/app/backend/data/hermes-recovery-backups` inside the persistent volume.

If no administrator appears, start the managed backend and retry. An empty, newly installed backend should use **Create the first account** instead. MFA remains enabled if an existing account uses it; this recovery does not bypass MFA.

## Forgot the HERMES unlock password

Use **Forgot your HERMES password? Reset saved sign-ins** in the unlock window. Choose a new password, confirm it and approve the saved-sign-in reset. An encrypted copy of the old vault is retained. The document store, chats, source folder and user accounts remain intact. Sign in again, or use local-account recovery if you also forgot that password.

The old encrypted vault cannot be decrypted without its old password. Recovery preserves the backend secret from its existing container where available. If the old container was removed but the data volume remains, the explicit reset can create a replacement authentication secret; existing sessions then need to sign in again. If no vault exists, use **Recover an existing setup? Reset saved sign-ins** in the password setup window. For normal recovery, keep Docker Desktop running and the existing container present, including when stopped.

## Source CLI tools

`bootstrap.sh` creates configuration and validates Compose without reading credentials. After creating the password in HERMES, `start.sh` asks for **your HERMES password** in Terminal using hidden input. It uses the bundled/built Rust vault helper; no password is placed in argv or a file. Stop, log inspection and doctor require no password.

The stress runner uses the same helper once per invocation. `HERMES_STRESS_TOKEN` remains an optional temporary local API-token alternative. `HERMES_PASSWORD` can supply the vault password to an explicitly configured local automation; prefer interactive entry for normal use and never commit it. The packaged app needs no environment variables.

## Security details

The vault uses RustCrypto Argon2id (64 MiB, three passes, one lane) to derive a 256-bit key from a unique random 16-byte salt, and XChaCha20-Poly1305 authenticated encryption with a fresh random 24-byte nonce for every write. The format/version and KDF parameters are bounded and checked before derivation. Files are written atomically with mode 0600 in the private application data directory. Derived keys and protected secret buffers are cleared on drop where supported.

The password protects credentials at rest, not the already-running Docker backend or source documents. Docker administrators can inspect the backend environment and operate its database; local Docker/account access remains a trust boundary. Keep macOS account protection and FileVault enabled as appropriate. No Mac password is stored or used by HERMES.
