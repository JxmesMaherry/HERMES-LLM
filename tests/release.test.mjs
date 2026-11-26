import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, existsSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";

const root = fileURLToPath(new URL("../", import.meta.url));
const version = JSON.parse(readFileSync(join(root, "package.json"))).version;
const tag = `v${version}`;
const dmg = `HERMES-LLM_${version}_aarch64.dmg`;

function fixture(t, mode = "fresh", badHash = false) {
  const dir = mkdtempSync(join(tmpdir(), "hermes-publish-test-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const bin = join(dir, "bin");
  const assets = join(dir, "assets");
  const log = join(dir, "calls.jsonl");
  mkdirSync(bin);
  mkdirSync(assets);
  const contents = Buffer.from("invented installer fixture; never executed");
  writeFileSync(join(assets, dmg), contents);
  const hash = badHash ? "0".repeat(64) : createHash("sha256").update(contents).digest("hex");
  writeFileSync(join(assets, "SHA256SUMS.txt"), `${hash}  ${dmg}\n`);
  // Only this disposable executable is put on PATH. No real GitHub calls occur.
  writeFileSync(join(bin, "gh"), `#!/usr/bin/env node
const fs = require('node:fs');
const args = process.argv.slice(2);
fs.appendFileSync(process.env.HERMES_TEST_CALLS, JSON.stringify(args) + '\\n');
const action = args[1];
const mode = process.env.HERMES_TEST_MODE;
const state = process.env.HERMES_TEST_STATE;
if (action === 'view' && args.includes('isDraft')) {
  if (mode === 'published') console.log('false');
  else if (mode === 'draft' || fs.existsSync(state)) console.log('true');
  else process.exit(1);
} else if (action === 'view' && args.includes('assets')) {
  console.log(process.env.HERMES_TEST_DMG);
  if (mode !== 'missing-asset') console.log('SHA256SUMS.txt');
} else if (action === 'create') fs.writeFileSync(state, 'draft');
else if (action === 'upload' && mode === 'upload-failure') process.exit(7);
else if (action === 'edit') fs.writeFileSync(state, 'published');
else if (action !== 'upload') process.exit(8);
`, { mode: 0o700 });
  const result = spawnSync("bash", [join(root, "scripts/publish-release.sh"), tag, assets], {
    encoding: "utf8",
    timeout: 15_000,
    env: { ...process.env, PATH: `${bin}:${process.env.PATH}`, HERMES_TEST_MODE: mode,
      HERMES_TEST_CALLS: log, HERMES_TEST_STATE: join(dir, "state"), HERMES_TEST_DMG: dmg },
  });
  const calls = existsSync(log) ? readFileSync(log, "utf8").trim().split("\n").map(JSON.parse) : [];
  return { result, calls };
}

test("publishes only after a draft contains both checksum-verified assets", t => {
  const { result, calls } = fixture(t);
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(calls.map(c => c[1]), ["view", "create", "upload", "view", "edit"]);
  assert(calls[1].includes("--draft"));
  assert(calls[1].includes("--verify-tag"));
  assert(calls[1].includes(`--prerelease=${version.startsWith("0.")}`));
  assert(calls[4].includes("--draft=false"));
});
test("upload failure leaves the release unpublished", t => {
  const { result, calls } = fixture(t, "upload-failure");
  assert.notEqual(result.status, 0);
  assert(!calls.some(c => c[1] === "edit"));
});
test("missing checksum attachment prevents publication", t => {
  const { result, calls } = fixture(t, "missing-asset");
  assert.notEqual(result.status, 0);
  assert(!calls.some(c => c[1] === "edit"));
});
test("a mismatched installer checksum prevents every GitHub call", t => {
  const { result, calls } = fixture(t, "fresh", true);
  assert.notEqual(result.status, 0);
  assert.deepEqual(calls, []);
});
test("published release assets are never overwritten", t => {
  const { result, calls } = fixture(t, "published");
  assert.notEqual(result.status, 0);
  assert.deepEqual(calls.map(c => c[1]), ["view"]);
});
test("an existing draft can resume uploads before publication", t => {
  const { result, calls } = fixture(t, "draft");
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(calls.map(c => c[1]), ["view", "upload", "view", "edit"]);
  assert(calls[1].includes("--clobber"));
});
