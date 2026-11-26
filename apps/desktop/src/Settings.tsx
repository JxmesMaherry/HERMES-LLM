import { useState } from "react";
import { Database, Moon, ShieldCheck, Sun } from "lucide-react";
import { bridge } from "./bridge";
import { Badge, Button, External, Heading } from "./components";
import { AuthForm, ModelPicker, KnowledgePicker, type Controls } from "./Setup";
export default function Settings(props: Controls) {
  const { config, health, busy, run, save, refresh } = props;
  const [port, setPort] = useState(String(config.port));
  const [max, setMax] = useState(String(config.max_file_mb));
  const [logs, setLogs] = useState<string | null>(null);
  return (
    <>
      <Heading eyebrow="MAKE IT YOURS" title="Settings" />
      <div className="settings-grid">
        <section className="panel">
          <h2>Intelligence</h2>
          <p>Choose an installed Ollama model for new conversations.</p>
          <ModelPicker {...props} />
        </section>
        <section className="panel">
          <div className="section-heading">
            <Database size={20} />
            <h2>Local backend</h2>
            <Badge status={health?.backend} />
          </div>
          <p>Open WebUI v0.13.5 · {`http://127.0.0.1:${config.port}`}</p>
          <div className="form-row">
            <label>
              HTTP port
              <input
                type="number"
                min={1024}
                max={65535}
                value={port}
                onChange={(e) => setPort(e.target.value)}
              />
            </label>
            <Button
              secondary
              disabled={!!busy}
              onClick={() =>
                void run("Saving port…", async () => {
                  await save({ ...config, port: Number(port) });
                  await refresh();
                })
              }
            >
              Save port
            </Button>
          </div>
          <p className="hint">
            Stop the backend before changing ports, then Start. The same data
            volume is preserved. Sign in again on the new port.
          </p>
          <label className="checkbox">
            <input
              type="checkbox"
              checked={config.allow_downloads}
              disabled={!!busy}
              onChange={(e) =>
                void run("Saving network preference…", () =>
                  save({ ...config, allow_downloads: e.target.checked }),
                )
              }
            />
            Allow local embedding model downloads from Hugging Face. Restart
            backend after changing.
          </label>
          <div className="button-row">
            {["start", "stop", "restart"].map((action) => (
              <Button
                key={action}
                secondary={action !== "start"}
                disabled={!!busy}
                onClick={() =>
                  void run(
                    `${action === "start" ? "Starting" : action === "stop" ? "Stopping" : "Restarting"} backend…`,
                    async () => {
                      await bridge.backend(action);
                      await refresh();
                    },
                  )
                }
              >
                {action[0].toUpperCase() + action.slice(1)}
              </Button>
            ))}
            <Button
              secondary
              disabled={!!busy}
              onClick={() =>
                void run("Reading local backend logs…", async () =>
                  setLogs(await bridge.backend("logs")),
                )
              }
            >
              View logs
            </Button>
          </div>
          <External
            onClick={() =>
              void run("Opening local Open WebUI…", () => bridge.open("webui"))
            }
          >
            Open WebUI
          </External>
          <p className="hint">
            Services remain running after HERMES closes. Stop preserves
            documents, chats and the Docker volume.
          </p>
          {logs !== null && (
            <details open>
              <summary>Local backend logs</summary>
              <p className="hint">
                Upstream logs may include private metadata. Review before
                sharing.
              </p>
              <pre className="logs">{logs || "No log output."}</pre>
              <Button secondary onClick={() => setLogs(null)}>
                Close logs
              </Button>
            </details>
          )}
        </section>
        <section className="panel">
          <h2>Documents & synchronisation</h2>
          <KnowledgePicker {...props} />
          <div className="form-row">
            <label>
              Maximum file size (MB)
              <input
                type="number"
                min={1}
                max={50}
                value={max}
                onChange={(e) => setMax(e.target.value)}
              />
            </label>
            <Button
              secondary
              disabled={!!busy}
              onClick={() =>
                void run("Saving sync options…", () =>
                  save({ ...config, max_file_mb: Number(max) }),
                )
              }
            >
              Save limit
            </Button>
          </div>
          <p className="hint">
            Manual incremental sync only. Empty directories are retained to
            protect files added by another client.
          </p>
        </section>
        <section className="panel">
          <h2>Account & appearance</h2>
          {health?.authenticated ? (
            <>
              <div className="success-note">
                <ShieldCheck size={19} />
                Local session saved in macOS Keychain
              </div>
              <Button
                secondary
                onClick={() =>
                  void run("Signing out…", async () => {
                    await bridge.signOut();
                    await refresh();
                  })
                }
                disabled={!!busy}
              >
                Sign out of HERMES
              </Button>
            </>
          ) : (
            <AuthForm {...props} />
          )}
          <hr />
          <label>
            Appearance
            <select
              value={config.theme}
              onChange={(e) =>
                void run("Saving theme…", () =>
                  save({
                    ...config,
                    theme: e.target.value as "dark" | "light",
                  }),
                )
              }
            >
              <option value="dark">Dark</option>
              <option value="light">Light</option>
            </select>
          </label>
          <p className="hint">
            {config.theme === "dark" ? <Moon size={14} /> : <Sun size={14} />}{" "}
            Your preference is saved on this Mac.
          </p>
          <hr />
          <h3>Application data</h3>
          <p className="path">
            {health?.data_dir ?? "Checking data location…"}
          </p>
          <p className="hint">
            Settings and desktop chat history live here. Open WebUI data lives
            in the Docker volume <code>hermes-llm-open-webui-data</code>. Back
            up both before upgrading.
          </p>
        </section>
      </div>
    </>
  );
}
