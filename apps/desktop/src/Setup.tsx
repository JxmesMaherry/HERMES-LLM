import { useState } from "react";
import {
  CheckCircle2,
  Download,
  FolderOpen,
  ArrowRight,
  ShieldCheck,
} from "lucide-react";
import { Badge, Button, External, Heading, Step } from "./components";
import { bridge } from "./bridge";
import type { Config, Health, Model, Knowledge } from "./types";
export interface Controls {
  config: Config;
  health: Health | null;
  models: Model[];
  knowledge: Knowledge[];
  busy: string;
  save: (config: Config) => Promise<void>;
  run: (name: string, action: () => Promise<void>) => Promise<void>;
  refresh: () => Promise<void>;
  chooseFolder: () => Promise<void>;
}
export function AuthForm({
  refresh,
  run,
  busy,
}: Pick<Controls, "refresh" | "run" | "busy">) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [create, setCreate] = useState(false);
  return (
    <form
      className="auth-form"
      onSubmit={(e) => {
        e.preventDefault();
        void run("Connecting your local account…", async () => {
          try {
            await bridge.auth(email, password, create);
            await refresh();
          } finally {
            setPassword("");
          }
        });
      }}
    >
      <h3>Connect your local Open WebUI account</h3>
      <p>
        On a new backend, create its first administrator account. Credentials
        stay on this Mac; the session is saved in Keychain.
      </p>
      <div className="form-row">
        <label>
          Email
          <input
            type="email"
            autoComplete="username"
            required
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
        </label>
        <label>
          Password
          <input
            type="password"
            autoComplete={create ? "new-password" : "current-password"}
            required
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
        </label>
      </div>
      <label className="checkbox">
        <input
          type="checkbox"
          checked={create}
          onChange={(e) => setCreate(e.target.checked)}
        />
        Create the first account on a new backend
      </label>
      <Button type="submit" disabled={!!busy}>
        {create ? "Create local account" : "Sign in locally"}
      </Button>
    </form>
  );
}
export function ModelPicker({
  config,
  models,
  save,
  run,
  refresh,
  busy,
}: Controls) {
  const [name, setName] = useState("qwen3:4b-instruct");
  const [progress, setProgress] = useState("");
  return (
    <div className="stack">
      <label>
        Installed model
        <select
          value={
            models.some((m) => m.name === config.model) ? config.model : ""
          }
          onChange={(e) =>
            void run("Saving model…", () =>
              save({ ...config, model: e.target.value }),
            )
          }
          disabled={!!busy}
        >
          <option value="" disabled>
            Choose an installed model
          </option>
          {models.map((m) => (
            <option key={m.name} value={m.name}>
              {m.name} · {(m.size / 1e9).toFixed(1)} GB
            </option>
          ))}
        </select>
      </label>
      {!models.length && (
        <p className="hint">
          No installed models found. Start Ollama, refresh, or download a model.
        </p>
      )}
      <details>
        <summary>
          <Download size={15} /> Download a model
        </summary>
        <p>
          This contacts Ollama’s model registry and uses disk space and
          bandwidth. Downloads start only when you press the button.
        </p>
        <label>
          Ollama model name
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            maxLength={160}
          />
        </label>
        <Button
          secondary
          disabled={!!busy || !name.trim()}
          onClick={() =>
            void run("Downloading model…", async () => {
              await bridge.pull(name, (p) =>
                setProgress(
                  `${p.status ?? "Downloading"}${p.total ? ` · ${Math.round(((p.completed ?? 0) / p.total) * 100)}%` : ""}`,
                ),
              );
              await refresh();
              await save({ ...config, model: name });
            })
          }
        >
          Download model from internet
        </Button>
        {progress && <p role="status">{progress}</p>}
      </details>
    </div>
  );
}
export function KnowledgePicker({
  config,
  knowledge,
  chooseFolder,
  save,
  run,
  refresh,
  busy,
}: Controls) {
  const [name, setName] = useState("My documents");
  return (
    <div className="stack">
      <div className="folder-choice">
        <FolderOpen size={24} />
        <div>
          <strong>Document source</strong>
          <p className="path">{config.folder ?? "No folder selected"}</p>
        </div>
        <Button
          secondary
          disabled={!!busy}
          onClick={() => void run("Choosing document folder…", chooseFolder)}
        >
          Choose folder
        </Button>
      </div>
      <p className="hint">
        PDF, TXT and Markdown · {config.max_file_mb} MB per file · hidden files
        and symbolic links excluded. Choose a dedicated folder. Scanned PDFs may
        need OCR before indexing.
      </p>
      <label>
        Knowledge base
        <select
          value={config.knowledge_id ?? ""}
          disabled={!!busy}
          onChange={(e) =>
            void run("Selecting knowledge base…", () =>
              save({ ...config, knowledge_id: e.target.value || null }),
            )
          }
        >
          <option value="">Choose a knowledge base</option>
          {knowledge.map((k) => (
            <option key={k.id} value={k.id}>
              {k.name}
            </option>
          ))}
        </select>
      </label>
      <div className="form-row">
        <label>
          New knowledge-base name
          <input
            value={name}
            maxLength={100}
            onChange={(e) => setName(e.target.value)}
          />
        </label>
        <Button
          secondary
          disabled={!!busy || !name.trim()}
          onClick={() =>
            void run("Creating knowledge base…", async () => {
              const k = await bridge.createKnowledge(name);
              await save({ ...config, knowledge_id: k.id });
              await refresh();
            })
          }
        >
          Create knowledge base
        </Button>
      </div>
      <p className="hint">
        Selecting a new source never clears a knowledge base. Every sync
        previews changes; removing indexed files requires your confirmation.
      </p>
    </div>
  );
}
export default function Setup(
  props: Controls & { finish: () => Promise<void> },
) {
  const [step, setStep] = useState(0);
  const { config, health, busy, run, refresh, save } = props;
  const prereqs =
    health?.docker.state === "running" && health?.ollama.state === "running";
  const model = props.models.some((m) => m.name === config.model);
  const ready = [
    prereqs,
    health?.backend.state === "running" && health?.authenticated,
    model,
    !!config.folder &&
      !!config.knowledge_id &&
      health?.folder.state === "ready",
  ][step];
  return (
    <>
      <Heading eyebrow="WELCOME TO HERMES" title="A little setup. All yours." />
      <p className="page-intro">
        Your models, documents and conversations. Running locally on your Mac.
      </p>
      <div className="wizard">
        <aside>
          {[
            "Check your Mac",
            "Connect the backend",
            "Choose your model",
            "Bring your documents",
          ].map((s, i) => (
            <Step key={s} n={i + 1} active={step === i} done={step > i}>
              {s}
            </Step>
          ))}
          <div className="privacy-note">
            <ShieldCheck size={22} />
            <p>
              No paid AI account.
              <br />
              No document uploads to the cloud.
            </p>
          </div>
        </aside>
        <section className="panel wizard-content">
          {step === 0 && (
            <>
              <span className="section-number">01 / PREREQUISITES</span>
              <h2>Give HERMES a local home.</h2>
              <p>
                Install the official applications, then open them. Ollama runs
                natively for Apple Silicon acceleration.
              </p>
              {(["ollama", "docker"] as const).map((s) => (
                <div className="prereq" key={s}>
                  <div>
                    <h3>{s === "ollama" ? "Ollama" : "Docker Desktop"}</h3>
                    <p>{health?.[s].detail ?? "Checking your Mac…"}</p>
                  </div>
                  <Badge status={health?.[s]} />
                  <Button
                    secondary
                    disabled={!!busy}
                    onClick={() =>
                      void run(`Opening ${s}…`, () => bridge.open(s))
                    }
                  >
                    Open
                  </Button>
                  <External
                    onClick={() =>
                      void run("Opening official download…", () =>
                        bridge.open(`${s}-download`),
                      )
                    }
                  >
                    Download
                  </External>
                </div>
              ))}
              <Button
                secondary
                disabled={!!busy}
                onClick={() => void run("Checking services…", refresh)}
              >
                Check again
              </Button>
            </>
          )}
          {step === 1 && (
            <>
              <span className="section-number">02 / LOCAL BACKEND</span>
              <h2>Connect your private workspace.</h2>
              <p>
                Start downloads Open WebUI’s official container if needed. Data
                persists in a dedicated Docker volume.
              </p>
              <label className="checkbox">
                <input
                  type="checkbox"
                  checked={config.allow_downloads}
                  disabled={!!busy}
                  onChange={(e) =>
                    void run("Saving download preference…", () =>
                      save({ ...config, allow_downloads: e.target.checked }),
                    )
                  }
                />
                Allow the initial local embedding model download from Hugging
                Face (about 90 MB). No documents are sent.
              </label>
              <div className="inline">
                <Badge status={health?.backend} />
                <Button
                  disabled={!!busy || !prereqs}
                  onClick={() =>
                    void run(
                      "Starting Open WebUI; first launch can take several minutes…",
                      async () => {
                        await bridge.backend("start");
                        await refresh();
                      },
                    )
                  }
                >
                  Start local backend
                </Button>
              </div>
              {health?.backend.state === "running" &&
                (health.authenticated ? (
                  <div className="success-note">
                    <CheckCircle2 size={20} />
                    Local account connected
                  </div>
                ) : (
                  <AuthForm {...props} />
                ))}
            </>
          )}
          {step === 2 && (
            <>
              <span className="section-number">03 / INTELLIGENCE</span>
              <h2>Choose the mind behind your workspace.</h2>
              <p>
                Start with a small instruct model. A 4B model is a practical
                choice for a 16 GB Mac.
              </p>
              <ModelPicker {...props} />
            </>
          )}
          {step === 3 && (
            <>
              <span className="section-number">04 / KNOWLEDGE</span>
              <h2>Make room for what you know.</h2>
              <p>
                Select documents on this Mac and a destination knowledge base.
                Sync them from the Library after setup.
              </p>
              <KnowledgePicker {...props} />
            </>
          )}
          <footer className="wizard-footer">
            <Button
              secondary
              disabled={step === 0 || !!busy}
              onClick={() => setStep(step - 1)}
            >
              Back
            </Button>
            <Button
              disabled={!ready || !!busy}
              onClick={() =>
                step === 3
                  ? void run("Finishing setup…", props.finish)
                  : setStep(step + 1)
              }
            >
              {step === 3 ? "Open HERMES" : "Continue"}
              <ArrowRight size={16} />
            </Button>
          </footer>
        </section>
      </div>
    </>
  );
}
