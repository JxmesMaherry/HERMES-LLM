import { useState } from "react";
import { LockKeyhole, X } from "lucide-react";
import { bridge, errorText } from "./bridge";
import { Button } from "./components";
import type { VaultStatus } from "./types";
export default function PasswordDialog({
  status,
  onClose,
  onDone,
}: {
  status: VaultStatus;
  onClose: () => void;
  onDone: () => Promise<void>;
}) {
  const [reset, setReset] = useState(false);
  const [password, setPassword] = useState("");
  const [confirmation, setConfirmation] = useState("");
  const [approved, setApproved] = useState(false);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState("");
  const creating = !status.exists || reset;
  return (
    <div className="modal-overlay">
      <section
        className="password-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="password-title"
      >
        <div className="section-heading">
          <LockKeyhole size={23} />
          <h2 id="password-title">
            {reset
              ? "Reset your HERMES password"
              : creating
                ? "Choose your HERMES password"
                : "Unlock HERMES"}
          </h2>
          <button
            className="text-button"
            aria-label="Close password dialog"
            disabled={working}
            onClick={onClose}
          >
            <X size={18} />
          </button>
        </div>
        <p>
          {creating
            ? "Choose a password you know. It encrypts saved local sign-ins on this Mac. Your books, chats and knowledge bases are kept."
            : "Enter the HERMES password you chose in this window. It unlocks your saved sign-in until you quit the app."}
        </p>
        <p className="hint">
          This is your HERMES password. No Mac login or Keychain password is
          requested.
        </p>
        {error && (
          <p role="alert" className="warning">
            {error}
          </p>
        )}
        <form
          onSubmit={async (e) => {
            e.preventDefault();
            setWorking(true);
            setError("");
            try {
              if (creating) {
                await bridge.createCredentials(password, confirmation, reset);
              } else {
                await bridge.unlock(password);
              }
              await onDone();
              onClose();
            } catch (e) {
              setError(errorText(e));
            } finally {
              setWorking(false);
              setPassword("");
              setConfirmation("");
            }
          }}
        >
          <label>
            {creating ? "New HERMES password" : "HERMES password"}
            <input
              autoFocus
              type="password"
              autoComplete={creating ? "new-password" : "current-password"}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              minLength={creating ? 12 : undefined}
              maxLength={1024}
              required
              disabled={working}
            />
          </label>
          {creating && (
            <label>
              Confirm new HERMES password
              <input
                type="password"
                autoComplete="new-password"
                value={confirmation}
                onChange={(e) => setConfirmation(e.target.value)}
                minLength={12}
                maxLength={1024}
                required
                disabled={working}
              />
            </label>
          )}
          {reset && (
            <label className="checkbox">
              <input
                type="checkbox"
                checked={approved}
                onChange={(e) => setApproved(e.target.checked)}
              />
              Reset saved sign-ins and keep my books and chats. Any existing
              vault will be retained as an encrypted backup.
            </label>
          )}
          <Button
            type="submit"
            disabled={
              working ||
              !password ||
              (creating && password !== confirmation) ||
              (reset && !approved)
            }
          >
            {working
              ? "Working…"
              : creating
                ? "Save HERMES password"
                : "Unlock for this session"}
          </Button>
        </form>
        <button
          className="text-button"
          disabled={working}
          onClick={() => {
            setReset(!reset);
            setPassword("");
            setConfirmation("");
            setError("");
            setApproved(false);
          }}
        >
          {reset
            ? status.exists
              ? "Back to unlock"
              : "Back to password setup"
            : status.exists
              ? "Forgot your HERMES password? Reset saved sign-ins"
              : "Recover an existing setup? Reset saved sign-ins"}
        </button>
      </section>
    </div>
  );
}
