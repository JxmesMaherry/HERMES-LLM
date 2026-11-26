import {
  ArrowRight,
  Box,
  Cpu,
  Database,
  FolderOpen,
  MessageSquare,
  RefreshCw,
  ShieldCheck,
} from "lucide-react";
import { Badge, Button, Heading } from "./components";
import type { Controls } from "./Setup";
import type { Page, SyncState } from "./types";
export default function Dashboard({
  config,
  health,
  models,
  busy,
  run,
  refresh,
  navigate,
  sync,
}: Controls & { navigate: (page: Page) => void; sync: SyncState }) {
  const ready =
    health?.backend.state === "running" &&
    health?.ollama.state === "running" &&
    health?.authenticated &&
    models.some((m) => m.name === config.model);
  const cards = [
    {
      key: "ollama" as const,
      name: "Ollama",
      label: "MODEL RUNTIME",
      icon: <Cpu size={22} />,
    },
    {
      key: "docker" as const,
      name: "Docker Desktop",
      label: "CONTAINER ENGINE",
      icon: <Box size={22} />,
    },
    {
      key: "backend" as const,
      name: "Open WebUI",
      label: "CHAT & RETRIEVAL",
      icon: <Database size={22} />,
    },
  ];
  return (
    <>
      <Heading eyebrow="YOUR LOCAL WORKSPACE" title="Welcome to HERMES">
        <Button
          secondary
          disabled={!!busy}
          onClick={() => void run("Checking your workspace…", refresh)}
        >
          <RefreshCw size={14} />
          Refresh status
        </Button>
      </Heading>
      <section className="hero">
        <div>
          <span className="hero-label">
            <i className="local-dot" />
            PRIVATE BY DESIGN
          </span>
          <h2>
            Big ideas.
            <br />
            <span>Close to home.</span>
          </h2>
          <p>
            A little intelligence, a lot of possibility.
            <br />
            Your AI workspace runs right here on your Mac.
          </p>
          <Button onClick={() => navigate(ready ? "chat" : "setup")}>
            {ready ? "Start a conversation" : "Set up your workspace"}
            <ArrowRight size={16} />
          </Button>
        </div>
        <div className="hero-art" aria-hidden="true">
          <div className="orbit o1" />
          <div className="orbit o2" />
          <div className="orbit o3" />
          <div className="hermes-mark">
            H<span>✦</span>
          </div>
          <div className="art-caption">INTELLIGENCE, WITHIN REACH</div>
        </div>
      </section>
      <div className="section-title">
        <h2>At a glance</h2>
        <span>Live service checks · loopback only</span>
      </div>
      <div className="status-grid">
        {cards.map((card) => (
          <section className="panel status-card" key={card.key}>
            <div className="card-top">
              <span className="icon-tile">{card.icon}</span>
              <Badge
                status={
                  busy.startsWith("Starting") && card.key === "backend"
                    ? { state: "starting", detail: busy }
                    : health?.[card.key]
                }
              />
            </div>
            <p className="eyebrow">{card.label}</p>
            <h3>{card.name}</h3>
            <p>{health?.[card.key].detail ?? "Checking service…"}</p>
          </section>
        ))}
      </div>
      <div className="overview-grid">
        <section className="panel">
          <div className="section-heading">
            <Cpu size={20} />
            <h3>Your model</h3>
          </div>
          <h2 className="model-name">{config.model}</h2>
          <p>
            {models.some((m) => m.name === config.model)
              ? `${models.length} installed model${models.length === 1 ? "" : "s"} available`
              : "Selected model is unavailable. Choose or download a model."}
          </p>
          <button className="text-button" onClick={() => navigate("settings")}>
            Manage models
            <ArrowRight size={14} />
          </button>
        </section>
        <section className="panel">
          <div className="section-heading">
            <FolderOpen size={20} />
            <h3>Your documents</h3>
          </div>
          <p className="path">{config.folder ?? "No source folder selected"}</p>
          <Badge status={health?.folder} />
          <p>
            {sync.success
              ? `Last successful sync: ${new Date(sync.success.completed_at * 1000).toLocaleString()}`
              : "Ready when you are. Add a folder to get started."}
          </p>
          <button className="text-button" onClick={() => navigate("knowledge")}>
            Open Library
            <ArrowRight size={14} />
          </button>
        </section>
      </div>
      <div className="info-strip">
        <ShieldCheck size={20} />
        <p>
          Models run with Ollama. Chat and document retrieval run with Open
          WebUI. Your conversations and documents stay on this Mac.
        </p>
        <MessageSquare size={19} />
      </div>
    </>
  );
}
