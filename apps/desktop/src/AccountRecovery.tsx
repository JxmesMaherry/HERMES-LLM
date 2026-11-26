import { useState } from "react";
import { bridge, errorText } from "./bridge";
import { Button } from "./components";
export default function AccountRecovery({
  onDone,
}: {
  onDone: () => Promise<void>;
}) {
  const [open, setOpen] = useState(false);
  const [accounts, setAccounts] = useState<
    { id: string; email: string; name: string }[]
  >([]);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirmation, setConfirmation] = useState("");
  const [approved, setApproved] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [complete, setComplete] = useState(false);
  async function load() {
    setOpen(true);
    setBusy(true);
    setError("");
    try {
      const result = await bridge.recoveryAccounts();
      setAccounts(result.accounts);
      setEmail(result.accounts[0]?.email ?? "");
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="account-recovery">
      <button
        className="text-button"
        type="button"
        onClick={() => (open ? setOpen(false) : void load())}
        disabled={busy}
      >
        {open
          ? "Close account recovery"
          : "Forgot your local account password?"}
      </button>
      {open && (
        <div className="recovery-panel">
          <h3>Recover your local account</h3>
          <p>
            A database backup is made first. This updates the selected
            administrator’s password and revokes its old sessions. Its books,
            chats, user ID and knowledge access are kept. You can use the same
            password you just chose for HERMES.
          </p>
          {error && (
            <p role="alert" className="warning">
              {error}
            </p>
          )}
          {complete && (
            <p role="status" className="success-note">
              Local password reset and sign-in completed.
            </p>
          )}
          <form
            onSubmit={async (e) => {
              e.preventDefault();
              setBusy(true);
              setError("");
              try {
                await bridge.resetLocalAccount(
                  email,
                  password,
                  confirmation,
                  approved,
                );
                setComplete(true);
                await onDone();
              } catch (e) {
                setError(errorText(e));
              } finally {
                setBusy(false);
                setPassword("");
                setConfirmation("");
              }
            }}
          >
            <label>
              Local administrator
              <select
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                disabled={busy}
                required
              >
                <option value="" disabled>
                  {accounts.length
                    ? "Select your local account"
                    : "No local administrator found"}
                </option>
                {accounts.map((a) => (
                  <option key={a.id} value={a.email}>
                    {a.name} · {a.email}
                  </option>
                ))}
              </select>
            </label>
            <label>
              New local account password
              <input
                type="password"
                autoComplete="new-password"
                minLength={12}
                maxLength={1024}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
                disabled={busy}
              />
            </label>
            <label>
              Confirm new password
              <input
                type="password"
                autoComplete="new-password"
                minLength={12}
                maxLength={1024}
                value={confirmation}
                onChange={(e) => setConfirmation(e.target.value)}
                required
                disabled={busy}
              />
            </label>
            <label className="checkbox">
              <input
                type="checkbox"
                checked={approved}
                onChange={(e) => setApproved(e.target.checked)}
                disabled={busy}
              />
              I confirm resetting this local account’s password while keeping
              its data.
            </label>
            <Button
              type="submit"
              disabled={
                busy ||
                !email ||
                !approved ||
                password !== confirmation ||
                password.length < 12
              }
            >
              {busy ? "Recovering account…" : "Reset password and sign in"}
            </Button>
          </form>
        </div>
      )}
    </div>
  );
}
