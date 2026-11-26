import { useState } from "react";
import { BookOpen, CheckCircle2, Files, RefreshCw } from "lucide-react";
import { bridge } from "./bridge";
import { Button, External, Heading } from "./components";
import { KnowledgePicker, type Controls } from "./Setup";
import type { Preview, SyncState } from "./types";
export default function Knowledge(
  props: Controls & { sync: SyncState; reloadSync: () => Promise<void> },
) {
  const [preview, setPreview] = useState<Preview | null>(null);
  const [deletions, setDeletions] = useState(false);
  const [progress, setProgress] = useState("");
  const { config, run, busy } = props;
  const result = props.sync.attempt;
  const success = props.sync.success;
  return (
    <>
      <Heading eyebrow="YOUR KNOWLEDGE, CONNECTED" title="The Library">
        <External
          onClick={() =>
            void run("Opening Open WebUI…", () => bridge.open("knowledge"))
          }
        >
          Open in Open WebUI
        </External>
      </Heading>
      <p className="page-intro">
        Turn your documents into a conversation. Everything is indexed locally.
      </p>
      <div className="library-grid">
        <section className="panel">
          <div className="section-heading">
            <FolderIcon />
            <div>
              <h2>Sources & destination</h2>
              <p>A folder on your Mac. A knowledge base of your own.</p>
            </div>
          </div>
          <KnowledgePicker {...props} />
        </section>
        <section className="panel sync-panel">
          <div className="section-heading">
            <RefreshCw size={21} />
            <div>
              <h2>Keep it in sync</h2>
              <p>Only new and changed files are processed.</p>
            </div>
          </div>
          <Button
            disabled={
              !!busy ||
              !config.folder ||
              !config.knowledge_id ||
              !props.health?.authenticated
            }
            onClick={() =>
              void run("Scanning and comparing documents…", async () => {
                const p = await bridge.preview();
                setPreview(p);
                setDeletions(false);
              })
            }
          >
            Preview changes
          </Button>
          <p className="hint">Preview first. Apply when you’re ready.</p>
          {preview && (
            <>
              <div className="sync-counts">
                <div>
                  <strong>{preview.added}</strong>
                  <span>added</span>
                </div>
                <div>
                  <strong>{preview.modified}</strong>
                  <span>modified</span>
                </div>
                <div>
                  <strong>{preview.unchanged}</strong>
                  <span>unchanged</span>
                </div>
                <div>
                  <strong>{preview.skipped}</strong>
                  <span>skipped</span>
                </div>
              </div>
              {preview.deletions.length > 0 && (
                <div className="deletion-preview">
                  <h3>
                    {preview.deletions.length} indexed files are absent from
                    this source
                  </h3>
                  <p>
                    They stay indexed unless you explicitly approve removal.
                    Changing folders does not clear a knowledge base.
                  </p>
                  <ul>
                    {preview.deletions.map((n, i) => (
                      <li key={`${n}-${i}`}>{n}</li>
                    ))}
                  </ul>
                  <label className="checkbox">
                    <input
                      type="checkbox"
                      checked={deletions}
                      onChange={(e) => setDeletions(e.target.checked)}
                    />
                    I confirm removal of these indexed files. Original documents
                    are never deleted.
                  </label>
                </div>
              )}
              <Button
                disabled={!!busy}
                onClick={() =>
                  void run("Synchronising documents…", async () => {
                    await bridge.sync(preview.id, deletions, (p) =>
                      setProgress(`${p.done} / ${p.total} · ${p.filename}`),
                    );
                    setPreview(null);
                    await props.reloadSync();
                  })
                }
              >
                Sync now{deletions ? " & remove confirmed files" : ""}
              </Button>
            </>
          )}
          {progress && (
            <p role="status" className="hint">
              {progress}
            </p>
          )}
          <div className="last-sync">
            <CheckCircle2 size={18} />
            <div>
              <strong>Latest successful sync</strong>
              <p>
                {success
                  ? new Date(success.completed_at * 1000).toLocaleString()
                  : "No successful sync yet"}
              </p>
              {success && (
                <small>
                  {success.added} added · {success.modified} modified ·{" "}
                  {success.unchanged} unchanged · {success.skipped} skipped ·{" "}
                  {success.failed} failed · {success.deleted} removed
                </small>
              )}
              {success &&
                (success.folder !== config.folder ||
                  success.knowledge_id !== config.knowledge_id) && (
                  <p className="warning">
                    This result belongs to a different folder or knowledge base.
                  </p>
                )}
            </div>
          </div>
          {result && (
            <div className={result.failed ? "sync-errors" : "sync-result"}>
              <strong>
                Latest attempt:{" "}
                {result.failed ? "needs attention" : "completed"}
              </strong>
              <p>
                {result.added} added · {result.modified} modified ·{" "}
                {result.unchanged} unchanged · {result.skipped} skipped ·{" "}
                {result.failed} failed · {result.deleted} removed ·{" "}
                {result.retained} retained
              </p>
              {result.errors.map((e) => (
                <p key={e}>{e}</p>
              ))}
            </div>
          )}
        </section>
      </div>
      <div className="info-strip">
        <BookOpen size={20} />
        <p>
          After syncing, enable <strong>Use knowledge base</strong> in Chat. If
          retrieval is unavailable in the native chat, open local Open WebUI and
          attach this knowledge base using <strong>#</strong>. Open WebUI keeps
          its own branding and browser chat history.
        </p>
      </div>
    </>
  );
}
function FolderIcon() {
  return <Files size={22} />;
}
