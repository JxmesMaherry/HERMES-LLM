import { useEffect, useRef, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {
  ArrowUp,
  BookOpen,
  MessageSquarePlus,
  Square,
  Trash2,
  Sparkles,
} from "lucide-react";
import { bridge, errorText } from "./bridge";
import { Button } from "./components";
import type { Config, Conversation, Model, Message } from "./types";
export default function Chat({
  config,
  models,
  connected,
  report,
}: {
  config: Config;
  models: Model[];
  connected: boolean;
  report: (error: string) => void;
}) {
  const [history, setHistory] = useState<Conversation[]>([]);
  const [active, setActive] = useState<string | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [model, setModel] = useState(config.model);
  const [input, setInput] = useState("");
  const [streaming, setStreaming] = useState(false);
  const [useKnowledge, setUseKnowledge] = useState(false);
  const [sources, setSources] = useState<string[]>([]);
  const request = useRef<string | null>(null);
  const bottom = useRef<HTMLDivElement>(null);
  const loaded = useRef(false);
  useEffect(() => {
    void bridge
      .history()
      .then((chats) => {
        setHistory(chats);
        loaded.current = true;
      })
      .catch((e) => report(errorText(e)));
  }, [report]);
  useEffect(() => {
    bottom.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);
  const persist = async (next: Conversation[]) => {
    await bridge.saveHistory(next);
    setHistory(next);
  };
  async function send() {
    if (!input.trim() || streaming || !connected || !loaded.current) return;
    const id = active ?? crypto.randomUUID();
    const prompt = input.trim();
    const next: Message[] = [...messages, { role: "user", content: prompt }];
    setActive(id);
    setInput("");
    setMessages([...next, { role: "assistant", content: "" }]);
    setStreaming(true);
    setSources([]);
    const requestId = crypto.randomUUID();
    request.current = requestId;
    let answer = "";
    try {
      // Save the user turn before the network request so an interrupted process keeps it.
      await persist([
        {
          id,
          title: next[0].content.slice(0, 64),
          messages: next,
          model,
          updated: Date.now(),
        },
        ...history.filter((c) => c.id !== id),
      ]);
      await bridge.chat(requestId, model, next, useKnowledge, (event) => {
        const text = event.choices?.[0]?.delta?.content;
        if (text) {
          answer += text;
          setMessages([...next, { role: "assistant", content: answer }]);
        }
        if (event.sources)
          setSources(
            event.sources.map(
              (s) =>
                s.source?.name ?? s.metadata?.[0]?.name ?? "Indexed document",
            ),
          );
      });
      if (!answer)
        report(
          "The backend completed without answer text. Check the model and Open WebUI logs.",
        );
    } catch (e) {
      report(errorText(e));
    } finally {
      const completed: Message[] = answer
        ? [...next, { role: "assistant", content: answer }]
        : next;
      try {
        await persist([
          {
            id,
            title: next[0].content.slice(0, 64),
            messages: completed,
            model,
            updated: Date.now(),
          },
          ...history.filter((c) => c.id !== id),
        ]);
      } catch (e) {
        report(errorText(e));
      }
      setMessages(completed);
      setStreaming(false);
      request.current = null;
    }
  }
  function newChat() {
    setActive(null);
    setMessages([]);
    setSources([]);
    setInput("");
    setModel(config.model);
  }
  return (
    <div className="chat-layout">
      <aside className="chat-history">
        <Button secondary disabled={streaming} onClick={newChat}>
          <MessageSquarePlus size={16} />
          New conversation
        </Button>
        <p className="eyebrow">ON THIS MAC</p>
        {history.length === 0 && (
          <p className="hint">Your conversations will appear here.</p>
        )}
        {history.map((c) => (
          <div
            key={c.id}
            className={`history-item ${active === c.id ? "selected" : ""}`}
          >
            <button
              disabled={streaming}
              onClick={() => {
                setActive(c.id);
                setMessages(c.messages);
                setModel(c.model);
                setSources([]);
              }}
            >
              {c.title}
            </button>
            <button
              aria-label={`Delete ${c.title}`}
              disabled={streaming}
              onClick={() =>
                void persist(history.filter((h) => h.id !== c.id))
                  .then(() => {
                    if (active === c.id) newChat();
                  })
                  .catch((e) => report(errorText(e)))
              }
            >
              <Trash2 size={13} />
            </button>
          </div>
        ))}
        <p className="history-note">
          Desktop history is stored locally. Open WebUI browser chats have their
          own history.
        </p>
      </aside>
      <section className="chat-main">
        <header className="chat-toolbar">
          <div>
            <span className="eyebrow">PRIVATE CONVERSATION</span>
            <select
              aria-label="Chat model"
              value={models.some((m) => m.name === model) ? model : ""}
              disabled={streaming}
              onChange={(e) => setModel(e.target.value)}
            >
              <option value="" disabled>
                Select an installed model
              </option>
              {models.map((m) => (
                <option key={m.name}>{m.name}</option>
              ))}
            </select>
          </div>
          <label
            className={`knowledge-toggle ${useKnowledge ? "enabled" : ""}`}
          >
            <BookOpen size={16} />
            <input
              type="checkbox"
              checked={useKnowledge}
              disabled={!config.knowledge_id || streaming}
              onChange={(e) => setUseKnowledge(e.target.checked)}
            />
            Use knowledge base
          </label>
        </header>
        <div className="messages">
          {!messages.length && (
            <div className="chat-welcome">
              <div className="orb">
                <Sparkles size={34} />
              </div>
              <p className="eyebrow">A SPACE TO THINK</p>
              <h1>What’s on your mind?</h1>
              <p>
                Explore an idea, work through a problem,
                <br />
                or find an answer in your documents.
              </p>
              <div className="suggestions">
                {[
                  "Help me think through an idea",
                  "Summarise what you know about…",
                  "Find a fact in my documents",
                ].map((s) => (
                  <button
                    key={s}
                    onClick={() => {
                      setInput(s);
                      if (s.startsWith("Find") && config.knowledge_id)
                        setUseKnowledge(true);
                    }}
                  >
                    {s}
                    <ArrowUp size={13} />
                  </button>
                ))}
              </div>
            </div>
          )}
          {messages.map((m, i) => (
            <article key={i} className={`message ${m.role}`}>
              <div className="message-role">
                {m.role === "user" ? "YOU" : "HERMES"}
                {m.role === "assistant" && <span>LOCAL AI</span>}
              </div>
              <div className="prose">
                <ReactMarkdown
                  remarkPlugins={[remarkGfm]}
                  skipHtml
                  components={{
                    img: () => <em>[Image omitted for privacy]</em>,
                    a: ({ children }) => (
                      <span className="rendered-link">{children}</span>
                    ),
                  }}
                >
                  {m.content || (streaming ? "Thinking…" : "")}
                </ReactMarkdown>
              </div>
            </article>
          ))}
          {sources.length > 0 && (
            <div className="sources">
              <BookOpen size={14} /> Retrieved sources:{" "}
              {[...new Set(sources)].join(", ")}
            </div>
          )}
          <div ref={bottom} />
        </div>
        <form
          className="composer"
          onSubmit={(e) => {
            e.preventDefault();
            void send();
          }}
        >
          <textarea
            aria-label="Message"
            placeholder={
              connected
                ? "Ask anything. It stays on this Mac."
                : "Start the backend and sign in to chat."
            }
            value={input}
            onChange={(e) => setInput(e.target.value)}
            rows={3}
            maxLength={200_000}
            disabled={streaming}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                void send();
              }
            }}
          />
          <div className="composer-footer">
            <span>
              <i className="local-dot" />
              Local inference
              {useKnowledge ? " · Knowledge retrieval enabled" : ""}
            </span>
            {streaming ? (
              <button
                aria-label="Cancel response"
                className="send"
                type="button"
                onClick={() => {
                  if (request.current)
                    void bridge
                      .cancel(request.current)
                      .catch((e) => report(errorText(e)));
                }}
              >
                <Square size={16} />
              </button>
            ) : (
              <button
                className="send"
                aria-label="Send message"
                disabled={
                  !connected ||
                  !input.trim() ||
                  !models.some((m) => m.name === model)
                }
              >
                <ArrowUp size={19} />
              </button>
            )}
          </div>
        </form>
        <p className="chat-disclaimer">
          AI can make mistakes. Check important answers against the original
          sources.
        </p>
      </section>
    </div>
  );
}
