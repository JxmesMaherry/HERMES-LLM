import test from "node:test";
import assert from "node:assert/strict";
import http from "node:http";
import { once } from "node:events";
import {
  parseArgs,
  validateCases,
  requestBody,
  measureRequest,
  runSuite,
  percentile,
} from "../scripts/stress-test.mjs";
const kb = "33333333-3333-4333-8333-333333333333";
test("bounds load and forbids remote destinations or RAG through bare Ollama", () => {
  assert.throws(() => parseArgs(["--concurrency", "100"]));
  assert.throws(() => parseArgs(["--url", "https://example.com"]));
  assert.throws(() => parseArgs(["--target", "ollama", "--knowledge"]));
  assert.throws(() => parseArgs(["--knowledge"], {}));
  assert.throws(() => validateCases([]));
  assert.equal(parseArgs(["--target", "ollama"]).port, 11434);
});
test("RAG attaches the configured collection and generation is bounded", () => {
  const options = parseArgs(["--knowledge", "--tokens", "128"], {
    knowledge_id: kb,
  });
  const body = requestBody(options, {
    prompt: "Question",
    expected: "hidden ground truth",
  });
  assert.equal(JSON.stringify(body).includes("hidden ground truth"), false);
  assert.deepEqual(body.files, [{ type: "collection", id: kb }]);
  assert.equal(body.params.num_ctx, 4096);
  assert.equal(body.max_tokens, 128);
});
test("concurrency stays bounded and failed runs stop scheduling", async () => {
  let active = 0,
    peak = 0;
  const options = parseArgs([
    "--requests",
    "20",
    "--concurrency",
    "2",
    "--stop-after-errors",
    "2",
  ]);
  const report = await runSuite(options, [{ id: "case", prompt: "question" }], {
    request: async () => {
      active++;
      peak = Math.max(peak, active);
      await new Promise((r) => setTimeout(r, 5));
      active--;
      throw new Error("Test failure");
    },
  });
  assert.equal(peak, 2);
  assert.ok(report.completed <= 3);
  assert.equal(report.stopped_early, true);
  assert.equal(report.passed, 0);
});
test("metrics exclude private answers and report real percentiles", async () => {
  const report = await runSuite(
    parseArgs(["--requests", "1"]),
    [{ id: "case", prompt: "private prompt" }],
    {
      request: async () => ({
        case: "case",
        ok: true,
        total_ms: 12,
        first_token_ms: 3,
        answer: "private answer",
      }),
    },
  );
  assert.equal(JSON.stringify(report).includes("private"), false);
  assert.equal(report.successful_latency_ms.p95, 12);
  assert.equal(percentile([], 0.95), null);
});
async function serverFor(t, handler) {
  const server = http.createServer(handler);
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  t.after(() => {
    server.closeAllConnections();
    server.close();
  });
  return server.address().port;
}
test("authenticated SSE handles split UTF-8, citations and token data", async (t) => {
  const port = await serverFor(t, (req, res) => {
    assert.equal(req.headers.authorization, "Bearer local-test");
    res.writeHead(200, { "Content-Type": "text/event-stream" });
    const bytes = Buffer.from(
      'data: {"sources":[{"source":{"name":"book"}}]}\n\ndata: {"choices":[{"delta":{"content":"héllo"}}]}\n\ndata: {"usage":{"completion_tokens":2},"choices":[{"finish_reason":"stop"}]}\n\ndata: [DONE]\n\n',
    );
    const split = bytes.indexOf(Buffer.from("é")) + 1;
    res.write(bytes.subarray(0, split));
    setTimeout(() => res.end(bytes.subarray(split)), 5);
  });
  const options = parseArgs(["--port", String(port), "--knowledge"], {
    knowledge_id: kb,
  });
  const result = await measureRequest(
    options,
    { id: "case", prompt: "question", expected: "héllo" },
    { token: "local-test" },
  );
  assert.equal(result.ok, true);
  assert.equal(result.sources, 1);
  assert.equal(result.completion_tokens, 2);
  assert.equal(result.answer, undefined);
});
test("a knowledge response without sources fails instead of pretending RAG worked", async (t) => {
  const port = await serverFor(t, (_req, res) => {
    res.writeHead(200, { "Content-Type": "text/event-stream" });
    res.end(
      'data: {"choices":[{"delta":{"content":"General knowledge answer"}}]}\n\ndata: [DONE]\n\n',
    );
  });
  const options = parseArgs(["--port", String(port), "--knowledge"], {
    knowledge_id: kb,
  });
  await assert.rejects(
    measureRequest(options, { id: "case", prompt: "question" }),
    /no retrieved sources/,
  );
});
test("stalled request is cancelled by its timeout", async (t) => {
  const port = await serverFor(t, (_req, res) => {
    res.writeHead(200, { "Content-Type": "text/event-stream" });
    res.flushHeaders();
  });
  const options = parseArgs(["--port", String(port), "--timeout", "1"]);
  await assert.rejects(
    measureRequest(options, { id: "case", prompt: "question" }),
  );
});
