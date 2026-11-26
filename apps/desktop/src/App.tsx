import { useCallback, useEffect, useMemo, useState } from "react";
import {
  BookOpen,
  ChevronRight,
  CircleHelp,
  LayoutDashboard,
  MessageSquare,
  Settings2,
  ShieldCheck,
  X,
} from "lucide-react";
import { bridge, errorText, singleFlight } from "./bridge";
import { Busy } from "./components";
import Dashboard from "./Dashboard";
import Chat from "./Chat";
import Knowledge from "./Knowledge";
import Settings from "./Settings";
import Setup from "./Setup";
import type {
  Config,
  Health,
  Model,
  Knowledge as KnowledgeType,
  Page,
  SyncState,
} from "./types";
export default function App() {
  const [config, setConfig] = useState<Config | null>(null);
  const [health, setHealth] = useState<Health | null>(null);
  const [models, setModels] = useState<Model[]>([]);
  const [knowledge, setKnowledge] = useState<KnowledgeType[]>([]);
  const [sync, setSync] = useState<SyncState>({ attempt: null, success: null });
  const [page, setPage] = useState<Page>("dashboard");
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");
  const report = useCallback((text: string) => setError(text), []);
  const reloadSync = useCallback(
    async () => setSync(await bridge.lastSync()),
    [],
  );
  const refresh = useMemo(
    () =>
      singleFlight(async () => {
        const h = await bridge.health();
        setHealth(h);
        if (h.ollama.state === "running") setModels(await bridge.models());
        else setModels([]);
        if (h.authenticated) setKnowledge(await bridge.knowledge());
        else setKnowledge([]);
      }),
    [],
  );
  useEffect(() => {
    let current = true;
    void bridge
      .config()
      .then(async (c) => {
        if (!current) return;
        setConfig(c);
        if (!c.setup_complete) setPage("setup");
        await Promise.all([refresh(), reloadSync()]);
      })
      .catch((e) => {
        if (current) report(errorText(e));
      });
    return () => {
      current = false;
    };
  }, [refresh, reloadSync, report]);
  useEffect(() => {
    if (!config) return;
    const check = () => {
      if (!document.hidden) void refresh().catch((e) => report(errorText(e)));
    };
    const timer = window.setInterval(check, 30_000);
    window.addEventListener("focus", check);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener("focus", check);
    };
  }, [config, refresh, report]);
  useEffect(() => {
    document.documentElement.dataset.theme = config?.theme ?? "dark";
  }, [config?.theme]);
  async function run(name: string, action: () => Promise<void>) {
    if (busy) return;
    setBusy(name);
    setError("");
    try {
      await action();
    } catch (e) {
      report(errorText(e));
    } finally {
      setBusy("");
    }
  }
  async function save(c: Config) {
    setConfig(await bridge.save(c));
  }
  const props = config
    ? {
        config,
        health,
        models,
        knowledge,
        busy,
        save,
        run,
        refresh,
        chooseFolder: async () => setConfig(await bridge.folder()),
      }
    : null;
  const nav = [
    {
      id: "dashboard" as const,
      label: "Overview",
      icon: <LayoutDashboard size={18} />,
    },
    { id: "chat" as const, label: "Chat", icon: <MessageSquare size={18} /> },
    {
      id: "knowledge" as const,
      label: "Library",
      icon: <BookOpen size={18} />,
    },
    {
      id: "settings" as const,
      label: "Settings",
      icon: <Settings2 size={18} />,
    },
  ];
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-symbol">
            H<span>✦</span>
          </span>
          <div>
            HERMES<span>LOCAL INTELLIGENCE</span>
          </div>
        </div>
        <p className="sidebar-caption">WORKSPACE</p>
        <nav>
          {nav.map((n) => (
            <button
              key={n.id}
              className={page === n.id ? "active" : ""}
              disabled={!!busy}
              onClick={() => setPage(n.id)}
            >
              {n.icon}
              {n.label}
              {page === n.id && <ChevronRight size={14} />}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <button
            className={page === "setup" ? "active" : ""}
            disabled={!!busy}
            onClick={() => setPage("setup")}
          >
            <CircleHelp size={17} />
            Setup guide
          </button>
          <div className="privacy-card">
            <ShieldCheck size={19} />
            <div>
              <strong>On your Mac.</strong>
              <span>Under your control.</span>
            </div>
          </div>
          <span className="version">HERMES-LLM · v0.1.0</span>
        </div>
      </aside>
      <main>
        <div className="topbar">
          <span>
            HERMES /{" "}
            <strong>
              {page === "knowledge"
                ? "Library"
                : page[0].toUpperCase() + page.slice(1)}
            </strong>
          </span>
          <span>
            <i
              className={`connection-dot ${health?.backend.state === "running" && health?.authenticated ? "online" : ""}`}
            />
            {health?.backend.state === "running"
              ? health.authenticated
                ? "Local backend connected"
                : "Local backend · sign-in needed"
              : "Local backend offline"}
          </span>
        </div>
        {error && (
          <div role="alert" className="error-banner">
            <span>{error}</span>
            <button aria-label="Dismiss error" onClick={() => setError("")}>
              <X size={16} />
            </button>
          </div>
        )}
        {busy && <Busy text={busy} />}
        <div className={`content ${page === "chat" ? "chat-content" : ""}`}>
          {!props ? (
            <div className="empty">
              <h1>HERMES-LLM</h1>
              <p>
                {error
                  ? "Launch the native desktop application to connect your local workspace."
                  : "Opening your local workspace…"}
              </p>
            </div>
          ) : (
            <>
              {page === "dashboard" && (
                <Dashboard {...props} navigate={setPage} sync={sync} />
              )}
              {page === "setup" && (
                <Setup
                  {...props}
                  finish={async () => {
                    await save({ ...props.config, setup_complete: true });
                    setPage("dashboard");
                  }}
                />
              )}
              {/* Keep Chat mounted to preserve an in-flight response across navigation. */}
              <div hidden={page !== "chat"} className="chat-container">
                <Chat
                  config={props.config}
                  models={models}
                  connected={
                    !!health?.authenticated &&
                    health.backend.state === "running" &&
                    health.ollama.state === "running"
                  }
                  report={report}
                />
              </div>
              {page === "knowledge" && (
                <Knowledge {...props} sync={sync} reloadSync={reloadSync} />
              )}
              {page === "settings" && <Settings {...props} />}
            </>
          )}
        </div>
      </main>
    </div>
  );
}
