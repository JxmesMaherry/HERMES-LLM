#!/usr/bin/env node
// Bounded local API load tests. Never pulls models, syncs/deletes files, or changes services.
import http from "node:http";
import { readFileSync, writeFileSync, statSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { readVaultValue } from "./vault-reader.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const defaultConfig = join(
  homedir(),
  "Library/Application Support/com.jxmesmaherry.hermesllm/config.json",
);
const HELP = `HERMES local stress test (Node.js 22.12+)
  node scripts/stress-test.mjs [options]
  --target ollama|webui   Local runtime or full backend (default: webui)
  --knowledge            Attach the KB selected in HERMES; require emitted retrieval sources
  --model NAME           Default: model selected in HERMES, or qwen3:4b-instruct
  --port NUMBER          Override the local service port (default: 3000 or 11434)
  --requests N           Measured requests, 1–1000 (default: 6)
  --concurrency N        Concurrent clients, 1–8 (default: 1)
  --context N            Context tokens, 512–32768 (default: 4096)
  --tokens N             Output-token limit, 1–2048 (default: 256)
  --timeout N            Per-request seconds, 1–600 (default: 180)
  --max-seconds N        Whole-run budget, 1–7200 (default: 900)
  --warmup N             Unmeasured warmup requests, 0–3 (default: 1)
  --stop-after-errors N  Stop scheduling after N errors, 1–100 (default: 3)
  --cases FILE           JSON array of {id,prompt,expected?}; expected is an optional exact phrase
  --output FILE          Write metrics only, no prompts, answers, source names or credentials
  --show-answers         Print generated answers locally; may contain private book excerpts
  --dry-run              Show plan; no network requests or password prompts
  --help                 Show help
Authentication: unlocks the local encrypted vault with your HERMES password.
Alternative: HERMES_STRESS_TOKEN environment variable, never a command-line token.
All HTTP goes directly to 127.0.0.1 through node:http, bypassing environment proxies.
Ctrl-C cancels this test's requests; it does not stop Docker or Ollama.`;
const numeric = {
  requests: [1, 1000],
  concurrency: [1, 8],
  context: [512, 32768],
  tokens: [1, 2048],
  timeout: [1, 600],
  "max-seconds": [1, 7200],
  warmup: [0, 3],
  "stop-after-errors": [1, 100],
  port: [1024, 65535],
};
export function parseArgs(args, config = {}) {
  const options = {
    target: "webui",
    model: config.model || "qwen3:4b-instruct",
    requests: 6,
    concurrency: 1,
    context: 4096,
    tokens: 256,
    timeout: 180,
    "max-seconds": 900,
    warmup: 1,
    "stop-after-errors": 3,
    knowledge: false,
    "dry-run": false,
    "show-answers": false,
    help: false,
  };
  for (let i = 0; i < args.length; i++) {
    const key = args[i].replace(/^--/, "");
    if (!args[i].startsWith("--"))
      throw new Error(`Unexpected argument: ${args[i]}`);
    if (["knowledge", "dry-run", "show-answers", "help"].includes(key)) {
      options[key] = true;
      continue;
    }
    if (
      ![...Object.keys(numeric), "target", "model", "cases", "output"].includes(
        key,
      )
    )
      throw new Error(`Unknown option: --${key}`);
    const value = args[++i];
    if (!value || value.startsWith("--"))
      throw new Error(`Missing value for --${key}`);
    if (key in numeric) {
      const number = Number(value);
      const [min, max] = numeric[key];
      if (!Number.isInteger(number) || number < min || number > max)
        throw new Error(`--${key} must be ${min}–${max}`);
      options[key] = number;
    } else options[key] = value;
  }
  if (!["ollama", "webui"].includes(options.target))
    throw new Error("--target must be ollama or webui");
  if (
    !/^[A-Za-z0-9][A-Za-z0-9._:/-]{0,159}$/.test(options.model) ||
    options.model.includes("..")
  )
    throw new Error("Invalid model name");
  if (options.knowledge && options.target !== "webui")
    throw new Error(
      "--knowledge requires --target webui; Ollama alone does not query your textbooks",
    );
  options.port ??= options.target === "ollama" ? 11434 : (config.port ?? 3000);
  if (
    !Number.isInteger(options.port) ||
    options.port < 1024 ||
    options.port > 65535
  )
    throw new Error("Invalid configured port");
  options.knowledgeId = options.knowledge ? config.knowledge_id : null;
  if (
    options.knowledge &&
    !/^[0-9a-f]{8}-[0-9a-f-]{27}$/i.test(options.knowledgeId ?? "")
  )
    throw new Error("Select a knowledge base in HERMES first");
  return options;
}
export function validateCases(value) {
  if (!Array.isArray(value) || !value.length || value.length > 100)
    throw new Error("Cases must be a non-empty JSON array, up to 100 cases");
  for (const item of value) {
    if (
      !item ||
      typeof item.id !== "string" ||
      !item.id.trim() ||
      item.id.length > 100 ||
      typeof item.prompt !== "string" ||
      !item.prompt.trim() ||
      item.prompt.length > 100_000 ||
      (item.expected !== undefined &&
        (typeof item.expected !== "string" || !item.expected.length))
    )
      throw new Error(
        "Each case needs a short id, a prompt and an optional non-empty expected phrase",
      );
  }
  return value;
}
export function requestBody(options, item) {
  const messages = [{ role: "user", content: item.prompt }];
  if (options.target === "ollama")
    return {
      model: options.model,
      messages,
      stream: true,
      options: {
        num_ctx: options.context,
        num_predict: options.tokens,
        temperature: 0,
      },
    };
  return {
    model: options.model,
    messages,
    stream: true,
    max_tokens: options.tokens,
    params: {
      function_calling: "legacy",
      num_ctx: options.context,
      num_predict: options.tokens,
      temperature: 0,
    },
    ...(options.knowledge
      ? { files: [{ type: "collection", id: options.knowledgeId }] }
      : {}),
  };
}
export async function measureRequest(options, item, { token, signal } = {}) {
  const started = performance.now();
  let first = null;
  let answer = "";
  let sources = 0;
  let terminal = false;
  let tokens = null;
  let generationNs = null;
  let promptTokens = null;
  const controller = AbortSignal.any([
    AbortSignal.timeout(options.timeout * 1000),
    ...(signal ? [signal] : []),
  ]);
  const body = JSON.stringify(requestBody(options, item));
  const response = await new Promise((accept, reject) => {
    const request = http.request(
      {
        hostname: "127.0.0.1",
        port: options.port,
        path:
          options.target === "ollama" ? "/api/chat" : "/api/chat/completions",
        method: "POST",
        signal: controller,
        headers: {
          "Content-Type": "application/json",
          "Content-Length": Buffer.byteLength(body),
          ...(token ? { Authorization: `Bearer ${token}` } : {}),
        },
      },
      accept,
    );
    request.once("error", reject);
    request.end(body);
  });
  if (response.statusCode !== 200) {
    response.destroy();
    throw new Error(`Local API returned HTTP ${response.statusCode}`);
  }
  if (
    options.target === "webui" &&
    !response.headers["content-type"]?.includes("text/event-stream")
  ) {
    response.destroy();
    throw new Error("Backend did not return an SSE chat stream");
  }
  const decoder = new TextDecoder();
  let pending = "";
  let bytes = 0;
  function line(text) {
    text = text.trim();
    if (!text || text.startsWith(":")) return;
    if (options.target === "webui") {
      if (!text.startsWith("data:")) return;
      text = text.slice(5).trim();
    }
    if (text === "[DONE]") {
      terminal = true;
      return;
    }
    let event;
    try {
      event = JSON.parse(text);
    } catch {
      throw new Error("Local API emitted invalid stream JSON");
    }
    if (event.error) throw new Error("Local API emitted a model error");
    const content =
      options.target === "ollama"
        ? event.message?.content
        : event.choices?.[0]?.delta?.content;
    if (content) {
      first ??= performance.now() - started;
      answer += content;
    }
    if (Array.isArray(event.sources)) sources += event.sources.length;
    if (event.done || event.choices?.some((choice) => choice.finish_reason))
      terminal = true;
    tokens = event.eval_count ?? event.usage?.completion_tokens ?? tokens;
    generationNs = event.eval_duration ?? generationNs;
    promptTokens =
      event.prompt_eval_count ?? event.usage?.prompt_tokens ?? promptTokens;
  }
  try {
    for await (const chunk of response) {
      bytes += chunk.length;
      if (bytes > 8 * 1024 * 1024)
        throw new Error("Stream exceeded the 8 MB safety limit");
      pending += decoder.decode(chunk, { stream: true });
      if (pending.length > 4 * 1024 * 1024)
        throw new Error("Stream event exceeded the safety limit");
      let newline;
      while ((newline = pending.indexOf("\n")) >= 0) {
        line(pending.slice(0, newline));
        pending = pending.slice(newline + 1);
      }
    }
    pending += decoder.decode();
    if (pending.trim()) line(pending);
  } finally {
    response.destroy();
  }
  if (!terminal)
    throw new Error("Chat stream ended without a completion marker");
  if (!answer.trim()) throw new Error("Model returned no answer text");
  if (options.knowledge && sources === 0)
    throw new Error(
      "Knowledge chat emitted no retrieved sources; verify indexing and selected KB",
    );
  const expectedMatch =
    item.expected === undefined
      ? null
      : answer.toLocaleLowerCase().includes(item.expected.toLocaleLowerCase());
  return {
    case: item.id,
    ok: expectedMatch !== false,
    total_ms: Math.round(performance.now() - started),
    first_token_ms: first === null ? null : Math.round(first),
    sources,
    answer_characters: answer.length,
    completion_tokens: tokens,
    prompt_tokens: promptTokens,
    tokens_per_second:
      tokens && generationNs
        ? Math.round((tokens / (generationNs / 1e9)) * 100) / 100
        : null,
    expected_match: expectedMatch,
    ...(expectedMatch === false
      ? { error: "Expected phrase missing; inspect this case manually" }
      : {}),
    ...(options["show-answers"] ? { answer } : {}),
  };
}
export function percentile(values, p) {
  if (!values.length) return null;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.max(0, Math.ceil(sorted.length * p) - 1)];
}
export async function runSuite(
  options,
  cases,
  { token, signal, request = measureRequest, update = () => {} } = {},
) {
  let next = 0;
  let errors = 0;
  const results = [];
  const started = performance.now();
  async function worker() {
    while (!signal?.aborted && errors < options["stop-after-errors"]) {
      const number = next++;
      if (number >= options.requests) break;
      const item = cases[number % cases.length];
      let result;
      const began = performance.now();
      try {
        result = await request(options, item, { token, signal });
      } catch (error) {
        result = {
          case: item.id,
          ok: false,
          total_ms: Math.round(performance.now() - began),
          first_token_ms: null,
          error:
            error.name === "AbortError" || error.name === "TimeoutError"
              ? "Request cancelled or timed out"
              : error.message,
        };
      }
      if (!result.ok) errors++;
      results.push({ request: number + 1, ...result });
      update(result, number + 1);
    }
  }
  await Promise.all(
    Array.from(
      { length: Math.min(options.concurrency, options.requests) },
      worker,
    ),
  );
  const good = results.filter((r) => r.ok);
  const durations = good.map((r) => r.total_ms);
  const ttft = good.map((r) => r.first_token_ms).filter((v) => v !== null);
  return {
    target: options.target,
    model: options.model,
    knowledge_attached: options.knowledge,
    context_requested: options.context,
    output_limit_requested: options.tokens,
    concurrency: options.concurrency,
    requested: options.requests,
    completed: results.length,
    passed: good.length,
    failed: results.length - good.length,
    stopped_early: results.length < options.requests,
    elapsed_ms: Math.round(performance.now() - started),
    successful_latency_ms: {
      p50: percentile(durations, 0.5),
      p95: percentile(durations, 0.95),
    },
    first_token_ms: { p50: percentile(ttft, 0.5), p95: percentile(ttft, 0.95) },
    results: results.map(({ answer: _answer, ...metrics }) => metrics),
  };
}
async function credential(port) {
  if (process.env.HERMES_STRESS_TOKEN) return process.env.HERMES_STRESS_TOKEN;
  return readVaultValue("token", dirname(defaultConfig), port);
}
async function main() {
  if (process.argv.includes("--help")) {
    console.log(HELP);
    return;
  }
  let config = {};
  try {
    config = JSON.parse(readFileSync(defaultConfig, "utf8"));
  } catch (error) {
    if (error.code !== "ENOENT")
      throw new Error("HERMES configuration could not be read");
  }
  const options = parseArgs(process.argv.slice(2), config);
  const casesPath = options.cases
    ? resolve(options.cases)
    : join(root, "tests/stress-cases.example.json");
  const caseInfo = statSync(casesPath);
  if (!caseInfo.isFile() || caseInfo.size > 10 * 1024 * 1024)
    throw new Error(
      "Case input must be a regular JSON file no larger than 10 MB",
    );
  let caseData;
  try {
    caseData = JSON.parse(readFileSync(casesPath, "utf8"));
  } catch {
    throw new Error(
      "Case input is not readable JSON; its contents were not logged",
    );
  }
  const cases = validateCases(caseData);
  const plan = {
    target: options.target,
    endpoint: `http://127.0.0.1:${options.port}`,
    model: options.model,
    knowledge_attached: options.knowledge,
    requests: options.requests,
    concurrency: options.concurrency,
    context: options.context,
    output_limit: options.tokens,
    max_seconds: options["max-seconds"],
    cases: cases.map((c) => c.id),
  };
  console.log(JSON.stringify(plan, null, 2));
  if (options["dry-run"]) return;
  const token =
    options.target === "webui" ? await credential(options.port) : null;
  const cancel = new AbortController();
  const stop = () => cancel.abort();
  process.once("SIGINT", stop);
  const signal = AbortSignal.any([
    cancel.signal,
    AbortSignal.timeout(options["max-seconds"] * 1000),
  ]);
  try {
    for (let i = 0; i < options.warmup; i++) {
      console.log(`Warmup ${i + 1}/${options.warmup} (excluded from metrics)`);
      await measureRequest(options, cases[i % cases.length], { token, signal });
    }
    const report = await runSuite(options, cases, {
      token,
      signal,
      update: (result, index) => {
        console.log(
          `${index}/${options.requests} ${result.ok ? "PASS" : "FAIL"} ${result.case} · ${result.total_ms} ms · first token ${result.first_token_ms ?? "n/a"} ms · ${result.sources ?? 0} source(s)${result.error ? ` · ${result.error}` : ""}`,
        );
        if (options["show-answers"] && result.answer)
          console.log(result.answer);
      },
    });
    const { results, ...summary } = report;
    console.log(JSON.stringify(summary, null, 2));
    if (options.output) {
      writeFileSync(
        resolve(options.output),
        JSON.stringify(report, null, 2) + "\n",
        { mode: 0o600, flag: "wx" },
      );
      console.log("Metrics written. Existing reports are never overwritten.");
    }
    if (report.failed || report.stopped_early) process.exitCode = 1;
  } finally {
    process.removeListener("SIGINT", stop);
  }
}
if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
)
  main().catch((error) => {
    console.error(`HERMES stress test: ${error.message}`);
    process.exitCode = 1;
  });
