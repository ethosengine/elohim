// The pre-push hook's live-run wiring (spec 2026-10-26-gate-live-failure-channel-design.md): the
// block before the gate loop opens a run log, the block after it closes gates-passed/failed, and
// the EXIT trap closes it interrupted on a kill. Like prepush-berth.test.mjs this runs the ACTUAL
// hook's text, sliced by exact comment lines (none of these three are `# ──` markers), in a scratch
// git repository holding a copy of the real gate-live.mjs.
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("../../", import.meta.url));
const HOOK = readFileSync(join(ROOT, ".husky/pre-push.bash"), "utf8");
const SHA = "a".repeat(40);
const TARGET = "refs/heads/dev";

function slice(startMarker, endMarker) {
  const start = HOOK.indexOf(startMarker);
  const end = HOOK.indexOf(endMarker, start);
  assert.ok(start >= 0 && end > start, `hook has ${startMarker} .. ${endMarker}`);
  return HOOK.slice(start, end);
}
// The trap prelude: HOOK_ON_EXIT, hook_on_exit and the trap lines.
const PRELUDE = slice('HOOK_ON_EXIT=""\n', 'BERTH_BIN="$HOOK_WORK_TREE');
const OPEN_BLOCK = slice("# Live gate events:", "TOTAL_START=");
const CLOSE_BLOCK = slice(
  "# The run describes the gate loop",
  'if [ -n "$FAILED" ]; then\n  echo "====',
);

const cleanEnv = Object.fromEntries(
  Object.entries(process.env).filter(([k]) => !k.startsWith("GIT_") || k.startsWith("GIT_CONFIG_")),
);

function fixture(t, { brokenCli = false } = {}) {
  const dir = mkdtempSync(join(tmpdir(), "prepush-gate-live-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const init = spawnSync("git", ["-C", dir, "init", "--quiet"], { env: cleanEnv, encoding: "utf8" });
  assert.equal(init.status, 0, init.stderr);
  const orch = join(dir, "genesis/orchestrator");
  mkdirSync(orch, { recursive: true });
  const cli = join(orch, "gate-live.mjs");
  if (brokenCli) writeFileSync(cli, "process.exit(1);\n");
  else copyFileSync(join(ROOT, "genesis/orchestrator/gate-live.mjs"), cli);
  return dir;
}

const env = () => ({
  ...cleanEnv,
  EPR_BIN: "/nonexistent/epr",
  ELOHIM_SESSION_ID: "hook-test",
  GATE_COMMIT: SHA,
  GATE_PUSH_TARGETS: TARGET,
});

const script = (...parts) =>
  `set -o pipefail\nGATE_COMMIT=${SHA}\nGATE_PUSH_TARGETS=${TARGET}\n${parts.join("\n")}\n`;

function runsOf(dir) {
  const runs = join(dir, ".eprfs/status/gate-live/runs");
  return existsSync(runs) ? readdirSync(runs).map((f) => join(runs, f)) : [];
}
const eventsOf = (file) =>
  readFileSync(file, "utf8").split("\n").filter(Boolean).map((l) => JSON.parse(l));

for (const [label, failed, outcome] of [["gates_passed", '""', "gates-passed"], ["failed", '" storage"', "failed"]]) {
  test(`live_run_opens_and_closes_${label}`, (t) => {
    const dir = fixture(t);
    const r = spawnSync(
      "bash",
      ["-c", script(`HOOK_WORK_TREE='${dir}'`, PRELUDE, OPEN_BLOCK, `FAILED=${failed}`,
        'FAILED=$(echo "$FAILED" | sed \'s/^ //\')', CLOSE_BLOCK, 'echo "RUN=$GATE_LIVE_RUN"')],
      { cwd: dir, env: env(), encoding: "utf8", timeout: 15000 },
    );
    assert.equal(r.status, 0, r.stdout + r.stderr);
    const runs = runsOf(dir);
    assert.equal(runs.length, 1, `one run file: ${runs}`);
    const events = eventsOf(runs[0]);
    assert.equal(events[0].kind, "run-open");
    assert.ok(JSON.stringify(events[0]).includes(SHA), "run-open carries the sha");
    assert.ok(JSON.stringify(events[0]).includes(TARGET), "run-open carries the target");
    const last = events.at(-1);
    assert.equal(last.kind, "run-close");
    assert.equal(last.outcome, outcome);
    assert.equal(last.deferred, 0);
    assert.equal(events.filter((e) => e.kind === "run-close").length, 1, "closed once (trap adds none after)");
    const follow = r.stdout.split("\n").find((l) => l.includes("Live failures:"));
    assert.ok(follow, r.stdout);
    assert.ok(follow.includes(r.stdout.match(/RUN=(\S+)/)[1]), "follow line names the run path");
    assert.ok(follow.includes(runs[0]) || follow.includes(runs[0].split("/").at(-1)), follow);
    const pointer = readFileSync(join(dir, ".eprfs/status/gate-live/by-session/hook-test"), "utf8").trim();
    assert.equal(pointer, runs[0]);
  });
}

test("live_run_counts_deferred_verdicts_before_later_push_refusal", (t) => {
  const dir = fixture(t);
  const r = spawnSync(
    "bash",
    ["-c", script(`HOOK_WORK_TREE='${dir}'`, PRELUDE, OPEN_BLOCK,
      'node genesis/orchestrator/gate-live.mjs event --run "$GATE_LIVE_RUN" --json \'{"kind":"gate-verdict","project":"slow","verdict":{"exit":0,"deferred":true}}\'',
      'node genesis/orchestrator/gate-live.mjs event --run "$GATE_LIVE_RUN" --json \'{"kind":"gate-verdict","project":"fast","verdict":{"exit":0}}\'',
      'FAILED=""', CLOSE_BLOCK, 'exit 1')],
    { cwd: dir, env: env(), encoding: "utf8", timeout: 15000 },
  );
  assert.equal(r.status, 1, r.stdout + r.stderr);
  const events = eventsOf(runsOf(dir)[0]);
  const last = events.at(-1);
  assert.equal(last.outcome, "gates-passed");
  assert.equal(last.deferred, 1);
  assert.equal(events.filter((e) => e.kind === "run-close").length, 1);
});

test("live_run_sigterm_closes_interrupted", async (t) => {
  const dir = fixture(t);
  const child = spawn(
    "bash",
    ["-c", script(`HOOK_WORK_TREE='${dir}'`, PRELUDE, OPEN_BLOCK, "sleep 30")],
    { cwd: dir, env: env(), stdio: ["ignore", "pipe", "ignore"] },
  );
  let out = "";
  child.stdout.on("data", (d) => { out += d; });
  const exited = new Promise((res) => child.on("exit", (code, signal) => res({ code, signal })));
  const deadline = Date.now() + 10000;
  // The follow line prints after the trap handler is registered; a kill any earlier is not this path.
  while (Date.now() < deadline && !out.includes("Live failures:")) {
    await new Promise((r) => setTimeout(r, 50));
  }
  assert.equal(runsOf(dir).length, 1, "run file exists before the kill");
  child.kill("SIGTERM");
  const { code, signal } = await exited;
  assert.equal(signal, null);
  assert.equal(code, 143);
  const events = eventsOf(runsOf(dir)[0]);
  assert.equal(events.at(-1).kind, "run-close");
  assert.equal(events.at(-1).outcome, "interrupted");
});

test("live_run_broken_cli_never_blocks_the_push", (t) => {
  const dir = fixture(t, { brokenCli: true });
  const r = spawnSync(
    "bash",
    ["-c", script(`HOOK_WORK_TREE='${dir}'`, PRELUDE, OPEN_BLOCK, 'echo "RUN=[$GATE_LIVE_RUN]"',
      'FAILED=""', CLOSE_BLOCK, "echo REACHED")],
    { cwd: dir, env: env(), encoding: "utf8", timeout: 15000 },
  );
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.ok(r.stdout.includes("RUN=[]"), r.stdout);
  assert.ok(!r.stdout.includes("Live failures:"), r.stdout);
  assert.ok(r.stdout.includes("REACHED"));
  assert.equal(runsOf(dir).length, 0);
});
