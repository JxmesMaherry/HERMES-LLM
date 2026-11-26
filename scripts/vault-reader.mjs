// Shared source CLI helper. Own HERMES password only; no macOS Keychain access.
import { spawn } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";
import { dirname, join, resolve } from "node:path";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
async function passwordPrompt() {
  if (process.env.HERMES_PASSWORD) return process.env.HERMES_PASSWORD;
  if (!process.stdin.isTTY || !process.stdin.setRawMode)
    throw new Error(
      "Use an interactive Terminal for your HERMES password, or explicitly supply HERMES_PASSWORD for a local automation.",
    );
  process.stderr.write("Your HERMES password (not your Mac password): ");
  return new Promise((accept, reject) => {
    let value = "";
    const wasRaw = process.stdin.isRaw;
    function finish(error) {
      process.stdin.removeListener("data", onData);
      process.stdin.setRawMode(wasRaw);
      process.stdin.pause();
      process.stderr.write("\n");
      if (error) reject(error);
      else accept(value);
    }
    function onData(chunk) {
      for (const char of chunk.toString("utf8")) {
        if (char === "\r" || char === "\n") {
          finish();
          return;
        }
        if (char === "\u0003" || char === "\u0004") {
          finish(new Error("HERMES unlock cancelled."));
          return;
        }
        if (char === "\u007f" || char === "\b") {
          value = Array.from(value).slice(0, -1).join("");
          continue;
        }
        if (char >= " " && value.length < 1024) value += char;
      }
    }
    process.stdin.setRawMode(true);
    process.stdin.resume();
    process.stdin.on("data", onData);
  });
}
export async function readVaultValue(kind, configDir, port) {
  if (!["token", "backend-secret"].includes(kind))
    throw new Error("Invalid HERMES helper action.");
  if (!existsSync(join(configDir, "credentials.vault")))
    throw new Error(
      "Open HERMES and choose your HERMES password first. Existing books and accounts do not need deleting.",
    );
  const base = join(root, "apps/desktop/src-tauri/target");
  const candidates = [
    process.env.HERMES_CLI_BIN,
    join(base, "aarch64-apple-darwin/release/hermes-llm"),
    join(base, "release/hermes-llm"),
    join(base, "debug/hermes-llm"),
    "/Applications/HERMES-LLM.app/Contents/MacOS/hermes-llm",
  ].filter(Boolean);
  const binary = candidates.find((file) => {
    try {
      return readFileSync(file).includes(Buffer.from("HERMES_VAULT_CLI_V1"));
    } catch {
      return false;
    }
  });
  if (!binary)
    throw new Error(
      "Build the updated HERMES app (npm run dmg or npm run desktop) before using its encrypted-vault CLI helper.",
    );
  const password = await passwordPrompt();
  return new Promise((accept, reject) => {
    const child = spawn(
      binary,
      ["--vault-read", kind, configDir, String(port)],
      { stdio: ["pipe", "pipe", "pipe"] },
    );
    let output = "";
    let error = "";
    const timeout = setTimeout(() => {
      child.kill();
      reject(new Error("HERMES vault helper timed out."));
    }, 30_000);
    child.stdout.on("data", (data) => {
      output += data;
      if (output.length > 16_384) child.kill();
    });
    child.stderr.on("data", (data) => {
      error += data;
      if (error.length > 4096) child.kill();
    });
    child.on("error", () => {
      clearTimeout(timeout);
      reject(new Error("Could not launch the HERMES vault helper."));
    });
    child.on("close", (code) => {
      clearTimeout(timeout);
      if (code !== 0)
        reject(
          new Error(
            error.trim() || "HERMES password did not unlock the vault.",
          ),
        );
      else accept(output.trim());
    });
    child.stdin.on("error", () => {});
    child.stdin.end(JSON.stringify({ password }));
  });
}
if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  readVaultValue(
    process.argv[2],
    resolve(process.argv[3] ?? ""),
    Number(process.argv[4]),
  )
    .then((value) => process.stdout.write(value))
    .catch((error) => {
      console.error(`HERMES: ${error.message}`);
      process.exitCode = 1;
    });
}
