// Exercise the ACTUAL hook function: a large environ must not turn the shared
// device language server into a stray when grep exits early under pipefail.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { once } from "node:events";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("../../", import.meta.url));
const HOOK = readFileSync(join(ROOT, ".husky/pre-push.bash"), "utf8");
const start = HOOK.indexOf("ra_strays() {\n");
const end = HOOK.indexOf("\n}\n", start);
assert.ok(start >= 0 && end > start, "hook has ra_strays() function");
const RA_STRAYS = HOOK.slice(start, end + "\n}\n".length);

const cleanEnv = Object.fromEntries(
  Object.entries(process.env).filter(([k]) => !k.startsWith("GIT_") || k.startsWith("GIT_CONFIG_")),
);

test("ra_strays_excludes_device_server_with_large_environ_under_pipefail", async (t) => {
  const env = { ...cleanEnv };
  delete env.ELOHIM_SESSION_ID;
  // Keep each string below Linux's 128 KiB limit, with 240 KiB after the marker.
  const padding = Object.fromEntries(
    Array.from({ length: 4 }, (_, i) => [`RA_TEST_PADDING_${i}`, "x".repeat(60 * 1024)]),
  );
  const children = [];
  t.after(() => { for (const child of children) child.kill(); });

  const device = spawn("sleep", ["30"], {
    env: { ELOHIM_SESSION_ID: "lsp-device", ...env, ...padding }, stdio: "ignore",
  });
  children.push(device);
  await once(device, "spawn");
  const stray = spawn("sleep", ["30"], { env, stdio: "ignore" });
  children.push(stray);
  await once(stray, "spawn");

  const r = spawnSync("bash", ["-c", [
    "set -o pipefail",
    `pgrep() { printf '%s\\n' ${device.pid} ${stray.pid}; }`,
    RA_STRAYS,
    "ra_strays",
  ].join("\n")], { cwd: ROOT, env, encoding: "utf8", timeout: 10000 });
  assert.equal(r.status, 0, r.stdout + r.stderr);
  const pids = r.stdout.trim().split(/\s+/);
  assert.ok(pids.includes(String(stray.pid)), "unmarked child is reported as a stray");
  assert.ok(!pids.includes(String(device.pid)), "device language server is never reported as a stray");
});
