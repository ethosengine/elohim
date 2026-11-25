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
import { constants } from "node:fs";
import { open } from "node:fs/promises";
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
const OPEN_BLOCK = slice("# Live gate events:", "# ── Live run open");
// The red recorder, the stop probe and the early-pass refusal that follow the open.
const RED_BLOCK = slice("# A red from a check is recorded", "# ── Red recorder end");
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
  else {
    copyFileSync(join(ROOT, "genesis/orchestrator/gate-live.mjs"), cli);
    copyFileSync(join(ROOT, "genesis/orchestrator/file-lock.mjs"), join(orch, "file-lock.mjs"));
  }
  return dir;
}

const env = () => ({
  ...cleanEnv,
  EPR_BIN: "/nonexistent/epr",
  ELOHIM_SESSION_ID: "hook-test",
  GATE_COMMIT: SHA,
  GATE_PUSH_TARGETS: TARGET,
});

// Hook slices end their legs by recording a step reading; outside the hook the recorder is stood
// in for (it passes its first argument, the leg's status, through).
const script = (...parts) =>
  `set -o pipefail\ngate_step_reading() { return "$1"; }\nGATE_COMMIT=${SHA}\nGATE_PUSH_TARGETS=${TARGET}\nLIVE_SHA=${SHA}\nLIVE_TARGETS=${TARGET}\n${parts.join("\n")}\n`;

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
  // Only bash is signalled, so the TERM trap alone can end the run: bash runs it once its
  // foreground child returns, and that child (an external `cat` of a fifo) exits 0 only when the
  // test releases it after the signal. A hook whose TERM trap does nothing therefore runs on to
  // exit 0, which this test refuses. (A fixed `sleep 30` made the wait 30 s.)
  const gate = join(dir, "foreground.fifo");
  assert.equal(spawnSync("mkfifo", [gate]).status, 0, "mkfifo");
  const child = spawn(
    "bash",
    ["-c", script(`HOOK_WORK_TREE='${dir}'`, PRELUDE, OPEN_BLOCK, `cat '${gate}' > /dev/null`)],
    { cwd: dir, env: env(), stdio: ["ignore", "pipe", "ignore"], detached: true },
  );
  // Its own process group, reaped whatever an assertion below does.
  t.after(() => { try { process.kill(-child.pid, "SIGKILL"); } catch { /* already gone */ } });
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
  // Release the foreground child only after the signal is pending, so it is the trap that ends it.
  // A non-blocking open finds no reader (ENXIO) until `cat` has the fifo open; stop once bash exits.
  let gone = false;
  exited.then(() => { gone = true; });
  while (!gone) {
    try {
      const fifo = await open(gate, constants.O_WRONLY | constants.O_NONBLOCK);
      await fifo.write("release\n");
      await fifo.close();
      break;
    } catch (error) {
      if (error.code !== "ENXIO") throw error;
      await new Promise((r) => setTimeout(r, 20));
    }
  }
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

for (const [label, code, outcome] of [["exit_0_closes_passed", 0, "passed"], ["exit_1_closes_failed", 1, "failed"]]) {
  test(`live_run_${label}_by_the_hook_exit_status`, (t) => {
    const dir = fixture(t);
    const r = spawnSync("bash", ["-c", script(`HOOK_WORK_TREE='${dir}'`, PRELUDE, OPEN_BLOCK, `exit ${code}`)],
      { cwd: dir, env: env(), encoding: "utf8", timeout: 15000 });
    assert.equal(r.status, code, r.stdout + r.stderr);
    const events = eventsOf(runsOf(dir)[0]);
    assert.equal(events.at(-1).kind, "run-close");
    assert.equal(events.at(-1).outcome, outcome);
    assert.ok(r.stdout.includes("[harvest]"), `the close prints the integrator's harvest: ${r.stdout}`);
  });
}

test("a pre-gate red is live at once, the push continues, and an early pass still refuses", (t) => {
  const dir = fixture(t);
  writeFileSync(join(dir, "genesis/orchestrator/gate-preflight.mjs"), "process.stdout.write('continue\\n');\n");
  const r = spawnSync("bash", ["-c", script(`HOOK_WORK_TREE='${dir}'`, PRELUDE, OPEN_BLOCK, RED_BLOCK,
    'leg_red habit-register "habits.yaml stale"', "echo AFTER-RED", "pass_or_refuse")],
    { cwd: dir, env: env(), encoding: "utf8", timeout: 15000 });
  assert.equal(r.status, 1, r.stdout + r.stderr);
  assert.ok(r.stdout.includes("AFTER-RED"), "continue: the next check still runs");
  assert.ok(r.stderr.includes("red before the gates: habit-register"), r.stderr);
  const events = eventsOf(runsOf(dir)[0]);
  const red = events.find((e) => e.kind === "failure");
  assert.equal(red.project, "habit-register");
  assert.equal(red.failure.excerpt[0], "habits.yaml stale");
  assert.equal(events.at(-1).outcome, "failed");
});

test("GATE_ON_RED=stop ends the push at its first pre-gate red", (t) => {
  const dir = fixture(t);
  writeFileSync(join(dir, "genesis/orchestrator/gate-preflight.mjs"), "process.stdout.write('stop\\n');\n");
  const r = spawnSync("bash", ["-c", script(`HOOK_WORK_TREE='${dir}'`, PRELUDE, OPEN_BLOCK, RED_BLOCK,
    'leg_red t2-receipt "refused"', "echo AFTER-RED")],
    { cwd: dir, env: env(), encoding: "utf8", timeout: 15000 });
  assert.equal(r.status, 1, r.stdout + r.stderr);
  assert.ok(!r.stdout.includes("AFTER-RED"));
  assert.equal(eventsOf(runsOf(dir)[0]).at(-1).outcome, "failed");
});

test("a stop requested on the live run is seen by the hook's probe", (t) => {
  const dir = fixture(t);
  writeFileSync(join(dir, "genesis/orchestrator/gate-preflight.mjs"), "process.stdout.write('continue\\n');\n");
  const r = spawnSync("bash", ["-c", script(`HOOK_WORK_TREE='${dir}'`, PRELUDE, OPEN_BLOCK, RED_BLOCK,
    'gate_stop_requested && echo STOP-BEFORE || echo NO-STOP-BEFORE',
    'node genesis/orchestrator/gate-live.mjs stop --wait 0 --run "$GATE_LIVE_RUN" --reason "integrator" >/dev/null',
    'gate_stop_requested && echo STOP-AFTER || echo NO-STOP-AFTER')],
    { cwd: dir, env: env(), encoding: "utf8", timeout: 15000 });
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.ok(r.stdout.includes("NO-STOP-BEFORE") && r.stdout.includes("STOP-AFTER"), r.stdout);
});

// A stop refuses the push until the run's last stop point, and is told so after it (gate-stop
// review, BLOCKING 2): stop_point marks the point in the run log first, then reads the stop.
test("a stop before the last stop point refuses; the run stays open for it until then", (t) => {
  for (const stop of [false, true]) {
    const dir = fixture(t);
    writeFileSync(join(dir, "genesis/orchestrator/gate-preflight.mjs"), "process.stdout.write('continue\\n');\n");
    const r = spawnSync("bash", ["-c", script(`HOOK_WORK_TREE='${dir}'`, PRELUDE, OPEN_BLOCK, RED_BLOCK,
      'GATE_LOOP_OUTCOME=gates-passed',
      stop ? 'node genesis/orchestrator/gate-live.mjs stop --wait 0 --run "$GATE_LIVE_RUN" --reason "late" >/dev/null' : 'true',
      'stop_point', 'echo PASSED-THROUGH')],
      { cwd: dir, env: env(), encoding: "utf8", timeout: 15000 });
    const events = eventsOf(runsOf(dir)[0]);
    if (stop) {
      assert.equal(r.status, 1, r.stdout + r.stderr);
      assert.match(r.stderr, /REFUSED: a stop was requested on the live run .*nothing is landed or pushed/);
      assert.ok(!r.stdout.includes("PASSED-THROUGH"));
      assert.equal(events.at(-1).outcome, "failed");
    } else {
      assert.equal(r.status, 0, r.stdout + r.stderr);
      assert.ok(r.stdout.includes("PASSED-THROUGH"));
      assert.ok(events.some((e) => e.kind === "stop-boundary"), "the stop point is marked");
      assert.equal(events.at(-1).kind, "run-close", "closed only at the stop point");
      assert.equal(events.at(-1).outcome, "gates-passed");
    }
  }
});

test("a stop after the last stop point is told the push is proceeding, and exits non-zero", (t) => {
  const dir = fixture(t);
  writeFileSync(join(dir, "genesis/orchestrator/gate-preflight.mjs"), "process.stdout.write('continue\\n');\n");
  const r = spawnSync("bash", ["-c", script(`HOOK_WORK_TREE='${dir}'`, PRELUDE, OPEN_BLOCK, RED_BLOCK,
    'GATE_LOOP_OUTCOME=gates-passed', 'stop_point', 'echo PASSED-THROUGH',
    'node genesis/orchestrator/gate-live.mjs stop --now --run "$GATE_LIVE_RUN"; echo "STOP-EXIT=$?"')],
    { cwd: dir, env: env(), encoding: "utf8", timeout: 15000 });
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.ok(r.stdout.includes("PASSED-THROUGH"));
  assert.match(r.stdout, /this run has passed its last stop point; the push is proceeding/);
  assert.match(r.stdout, /STOP-EXIT=1/);
  assert.equal(eventsOf(runsOf(dir)[0]).filter((e) => e.kind === "stop-request").length, 0, "nothing appended");
});

test("a stop appended after the stop-point mark refuses nothing", (t) => {
  const dir = fixture(t);
  writeFileSync(join(dir, "genesis/orchestrator/gate-preflight.mjs"), "process.stdout.write('continue\\n');\n");
  const r = spawnSync("bash", ["-c", script(`HOOK_WORK_TREE='${dir}'`, PRELUDE, OPEN_BLOCK, RED_BLOCK,
    'node genesis/orchestrator/gate-live.mjs boundary --run "$GATE_LIVE_RUN"',
    // Appended past the mark while the run is still open (the CLI's own race): it does not count.
    'node genesis/orchestrator/gate-live.mjs event --run "$GATE_LIVE_RUN" --json \'{"kind":"stop-request","immediate":true}\'',
    'gate_stop_requested && echo COUNTS || echo IGNORED')],
    { cwd: dir, env: env(), encoding: "utf8", timeout: 15000 });
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.ok(r.stdout.includes("IGNORED"), r.stdout);
});

// The real hook_pass, its landing machinery stood in for: the stop point, the claim, the hand-off.
const HOOK_PASS = slice("hook_pass() {\n", "\nphase_l_on_exit() {");
const LANDING_STUBS = [
  "phase_l_channel() { echo 'a b c'; }", "phase_l_owner_checks() { :; }",
  "phase_l_handoff() { echo HANDED-OFF; }", "phase_l_refuse() { echo \"$1\" >&2; exit 1; }",
  "PUSH_LINES=()", "BERTH_TARGETS=' dev'", "BERTH_TTL_S=600",
];
// A gate-live CLI whose stop-point mark cannot be written; every other verb is the real one.
function failingMark(dir) {
  const orch = join(dir, "genesis/orchestrator");
  copyFileSync(join(orch, "gate-live.mjs"), join(orch, "gate-live-real.mjs"));
  writeFileSync(join(orch, "gate-live.mjs"), [
    "import { spawnSync } from 'node:child_process';",
    "if (process.argv[2] === 'boundary') process.exit(1);",
    "const r = spawnSync(process.execPath, [new URL('./gate-live-real.mjs', import.meta.url).pathname, ...process.argv.slice(2)], { stdio: 'inherit' });",
    "process.exit(r.status ?? 1);",
  ].join("\n"));
}

test("an unmarked stop point is read again after the landing claim; a stop there refuses, and stop says so with exit 0", (t) => {
  const dir = fixture(t);
  writeFileSync(join(dir, "genesis/orchestrator/gate-preflight.mjs"), "process.stdout.write('continue\\n');\n");
  failingMark(dir);
  // The stop arrives while the landing claims dev-integration, after the first (unmarked) stop point.
  const claim = 'phase_l_claim() { { node genesis/orchestrator/gate-live.mjs stop --wait 10 --run "$GATE_LIVE_RUN" > stop.out; echo "STOP-EXIT=$?" >> stop.out; } & ' +
    'until grep -q "\\"stop-request\\"" "$GATE_LIVE_RUN"; do sleep 0.05; done; }';
  const r = spawnSync("bash", ["-c", script(`HOOK_WORK_TREE='${dir}'`, PRELUDE, OPEN_BLOCK, RED_BLOCK, ...LANDING_STUBS, claim,
    HOOK_PASS, 'GATE_LOOP_OUTCOME=gates-passed', 'hook_pass')],
    { cwd: dir, env: env(), encoding: "utf8", timeout: 20000 });
  assert.equal(r.status, 1, r.stdout + r.stderr);
  assert.match(r.stderr, /last stop point could not be marked/);
  assert.match(r.stderr, /REFUSED: a stop was requested on the live run/);
  assert.ok(!r.stdout.includes("HANDED-OFF"), "the push is not handed off");
  const until = Date.now() + 10000;
  while (!/STOP-EXIT=/.test(existsSync(join(dir, "stop.out")) ? readFileSync(join(dir, "stop.out"), "utf8") : "") && Date.now() < until) spawnSync("sleep", ["0.1"]);
  const out = readFileSync(join(dir, "stop.out"), "utf8");
  assert.match(out, /STOP-EXIT=0/, out);
  assert.match(out, /the run took the stop \(landing-handoff\)/, out);
});

test("an unmarked stop point that the push passes: a later stop never gets exit 0", (t) => {
  const dir = fixture(t);
  writeFileSync(join(dir, "genesis/orchestrator/gate-preflight.mjs"), "process.stdout.write('continue\\n');\n");
  failingMark(dir);
  // A non-landing push passes its (unmarked) stop point and exits 0; the exit trap closes the run.
  const r = spawnSync("bash", ["-c", script(`HOOK_WORK_TREE='${dir}'`, PRELUDE, OPEN_BLOCK, RED_BLOCK, ...LANDING_STUBS,
    "BERTH_TARGETS=''", HOOK_PASS, 'echo "RUN=$GATE_LIVE_RUN"', 'hook_pass')],
    { cwd: dir, env: env(), encoding: "utf8", timeout: 20000 });
  assert.equal(r.status, 0, r.stdout + r.stderr);
  const run = runsOf(dir)[0];
  const stop = spawnSync(process.execPath, [join(dir, "genesis/orchestrator/gate-live-real.mjs"), "stop", "--wait", "2", "--run", run],
    { cwd: dir, env: env(), encoding: "utf8", timeout: 15000 });
  assert.notEqual(stop.status, 0, stop.stdout + stop.stderr);
  assert.match(stop.stdout, /passed its last stop point; the push is proceeding/);
});

test("a boundary and a stop with one sequence number: the stop counts for nothing, and the push proceeds", (t) => {
  const dir = fixture(t);
  writeFileSync(join(dir, "genesis/orchestrator/gate-preflight.mjs"), "process.stdout.write('continue\\n');\n");
  // An unlocked append's tie, written as it would land: the mark, then a stop with the mark's seq.
  const tie = 'node genesis/orchestrator/gate-live.mjs boundary --run "$GATE_LIVE_RUN"; ' +
    'seq=$(node -e "const l=require(\'fs\').readFileSync(process.argv[1],\'utf8\').trim().split(\'\\n\').map(JSON.parse); process.stdout.write(String(l.find(e=>e.kind===\'stop-boundary\').seq))" "$GATE_LIVE_RUN"); ' +
    'printf \'{"v":1,"run":"x","seq":%s,"at":"2026-11-25T00:00:00.000Z","kind":"stop-request","immediate":true}\\n\' "$seq" >> "$GATE_LIVE_RUN"';
  const r = spawnSync("bash", ["-c", script(`HOOK_WORK_TREE='${dir}'`, PRELUDE, OPEN_BLOCK, RED_BLOCK, ...LANDING_STUBS,
    tie, 'refuse_if_stopped', 'echo PROCEEDS')],
    { cwd: dir, env: env(), encoding: "utf8", timeout: 20000 });
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.ok(r.stdout.includes("PROCEEDS"), "the hook does not count the tie");
});

test("the stop point is the last act before each pass: hand-over, non-landing and dev landing", () => {
  const handover = slice("handover_finish() {\n", "\n}\n");
  assert.match(handover, /stop_point\n  echo "\[pre-push\] hand-over: seconds checks green[^\n]*\n  exit 0\n?$/, "hand-over: stop_point, then exit 0");
  const pass = slice("hook_pass() {\n", "\n}\n");
  assert.ok(pass.includes('[ -n "$BERTH_TARGETS" ] || { stop_point; exit 0; }'), "non-landing pass");
  const point = pass.indexOf("  stop_point landing-claim\n");
  assert.ok(point > 0 && point < pass.indexOf("phase_l_claim"), "dev landing: before the dev-integration claim");
  const again = pass.indexOf("  stop_point landing-handoff\n");
  assert.ok(again > pass.indexOf("phase_l_claim") && again < pass.indexOf("phase_l_handoff"), "and again before the hand-off, for an unmarked point");
  assert.ok(!/close --run "\$GATE_LIVE_RUN" \\\n    --outcome "\$\(\[ -n "\$FAILED" \]/.test(HOOK), "a passed gate loop no longer closes the run early");
});

test("a stop after every gate started, a carried one included, leaves the run failed", (t) => {
  const dir = fixture(t);
  writeFileSync(join(dir, "genesis/orchestrator/gate-preflight.mjs"), "process.stdout.write('continue\\n');\n");
  const after = slice('if [ -n "$NOT_RUN" ]; then\n  echo "[pre-push] stopped: $STOP_REASON', 'FAILED="$FAILED$PREGATE_FAILED"');
  for (const stop of [false, true]) {
    const r = spawnSync("bash", ["-c", script(`HOOK_WORK_TREE='${dir}'`, PRELUDE, OPEN_BLOCK, RED_BLOCK,
      // Every gate ran and a carried gate was collected green: nothing failed, nothing was left unrun.
      'FAILED=""; NOT_RUN=""; STOP_REASON=""',
      stop ? 'node genesis/orchestrator/gate-live.mjs stop --wait 0 --run "$GATE_LIVE_RUN" >/dev/null' : 'true',
      after, 'echo "FAILED=[$FAILED]"')],
      { cwd: dir, env: env(), encoding: "utf8", timeout: 15000 });
    assert.equal(r.status, 0, r.stdout + r.stderr);
    assert.ok(r.stdout.includes(stop ? "FAILED=[stopped]" : "FAILED=[]"), r.stdout);
  }
});

// The fleet-roll block: it hands each pushed ref over as "<remote ref> <pushed sha> <destination
// sha>" with the hook's remote and a mark file, runs `--fleet-roll` before the hook's early
// passes, records exit 3 as a pre-gate red, and records a node that could not run as a red only
// when a pushed ref is dev or main.
const fleetBlock = () => slice("# Fleet roll (seconds):", "# ── Fleet roll end");
const STUB_PREFLIGHT = [
  "import { writeFileSync } from 'node:fs';",
  "const argv = process.argv.slice(2);",
  "if (argv.includes('--on-red')) { process.stdout.write('continue\\n'); process.exit(0); }",
  "if (argv.includes('--fleet-roll')) {",
  "  writeFileSync('refs.out', JSON.stringify({ refs: process.env.GATE_PUSH_REFS, remote: process.env.GATE_PUSH_REMOTE, mark: process.env.GATE_FLEET_ROLL_MARK }));",
  "  const code = Number(process.env.STUB_EXIT || 0);",
  "  process.stdout.write(code === 3 ? '[preflight] REFUSED fleet-roll: stub\\n' : '[preflight] ok fleet-roll (stub)\\n');",
  "  process.exit(code);",
  "}",
  "process.exit(2);",
].join("\n");

function fleetFixture(t) {
  const dir = fixture(t);
  writeFileSync(join(dir, "genesis/orchestrator/gate-preflight.mjs"), `${STUB_PREFLIGHT}\n`);
  return { dir, base: "a".repeat(40), tip: "c".repeat(40) };
}

const fleetScript = (dir, lines) => [
  `HOOK_WORK_TREE='${dir}'`, PRELUDE, OPEN_BLOCK, RED_BLOCK,
  "hook_pass() { exit 0; }",
  "PHASE_L_REMOTE=forge",
  `ZERO_SHA=${"0".repeat(40)}`,
  'is_notes_ref() { case "$1" in refs/notes/*) return 0 ;; esac; return 1; }',
  "PUSH_LINES=(", ...lines.map((l) => `  "${l}"`), ")",
  fleetBlock(), "pass_or_refuse",
];
const mixedLines = (base, tip) => [
  `refs/heads/x ${tip} refs/heads/dev ${base}`,
  `refs/notes/brit ${tip} refs/notes/brit/build/x ${base}`,
  `(delete) ${"0".repeat(40)} refs/heads/gone ${base}`,
  `refs/heads/x ${tip} refs/heads/self/new ${"0".repeat(40)}`,
];

test("fleet-roll: the hook hands each eligible ref, its remote and a mark file over, and exit 3 is a live pre-gate red", (t) => {
  const { dir, base, tip } = fleetFixture(t);
  const r = spawnSync("bash", ["-c", script(...fleetScript(dir, mixedLines(base, tip)))],
    { cwd: dir, env: { ...env(), STUB_EXIT: "3" }, encoding: "utf8", timeout: 15000 });
  assert.equal(r.status, 1, r.stdout + r.stderr);
  assert.ok(r.stdout.includes("[preflight] REFUSED fleet-roll: stub"), r.stdout);
  assert.ok(r.stderr.includes("red before the gates: fleet-roll"), r.stderr);
  const seen = JSON.parse(readFileSync(join(dir, "refs.out"), "utf8"));
  assert.equal(seen.refs, [`refs/heads/dev ${tip} ${base}`, `refs/heads/self/new ${tip} ${"0".repeat(40)}`].join("\n"), "notes and deletes are not handed over");
  assert.equal(seen.remote, "forge");
  assert.ok(seen.mark, "a mark file is named");
  assert.equal(existsSync(seen.mark), false, "and removed when the hook ends");
  assert.equal(eventsOf(runsOf(dir)[0]).find((e) => e.kind === "failure").project, "fleet-roll");
});

test("fleet-roll: a node that could not run is a red on a dev or main push, a WARN otherwise; a clear check adds none", (t) => {
  const { dir, base, tip } = fleetFixture(t);
  const run = (lines, extra) => spawnSync("bash", ["-c", script(...fleetScript(dir, lines))],
    { cwd: dir, env: { ...env(), ...extra }, encoding: "utf8", timeout: 15000 });
  const crashed = run(mixedLines(base, tip), { STUB_EXIT: "1" });
  assert.equal(crashed.status, 1, crashed.stdout + crashed.stderr);
  assert.ok(crashed.stderr.includes("the fleet-roll check could not run (node exit 1, above) on a push to dev or main"), crashed.stderr);
  const advisory = run([`refs/heads/x ${tip} refs/heads/self/x ${base}`], { STUB_EXIT: "1" });
  assert.equal(advisory.status, 0, advisory.stdout + advisory.stderr);
  assert.ok(advisory.stdout.includes("[preflight] WARN fleet-roll: the check could not run (node exit 1, above); no pushed ref deploys"), advisory.stdout);
  const clear = run(mixedLines(base, tip), {});
  assert.equal(clear.status, 0, clear.stdout + clear.stderr);
  assert.ok(clear.stdout.includes("[preflight] ok fleet-roll (stub)"), clear.stdout);
});

test("round 5, seen failing first: a node that could not run on a push naming dev first among 800 more refs is a red", (t) => {
  const { dir, base, tip } = fleetFixture(t);
  const lines = [`refs/heads/x ${tip} refs/heads/dev ${base}`,
    ...Array.from({ length: 800 }, (_, i) => `refs/heads/x ${tip} refs/heads/self/handover-${String(i).padStart(4, "0")} ${base}`)];
  const r = spawnSync("bash", ["-c", script(...fleetScript(dir, lines))],
    { cwd: dir, env: { ...env(), STUB_EXIT: "1" }, encoding: "utf8", timeout: 30000 });
  assert.equal(r.status, 1, r.stdout.slice(-400) + r.stderr.slice(-400));
  assert.ok(r.stderr.includes("could not run (node exit 1, above) on a push to dev or main"), r.stderr.slice(-400));
});

test("fleet-roll: a push of notes and deletes only starts no node", (t) => {
  const { dir, base, tip } = fleetFixture(t);
  const r = spawnSync("bash", ["-c", script(...fleetScript(dir, mixedLines(base, tip).slice(1, 3)))],
    { cwd: dir, env: { ...env(), STUB_EXIT: "3" }, encoding: "utf8", timeout: 15000 });
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.equal(existsSync(join(dir, "refs.out")), false);
});

// The real CLI behind the hook block, in this checkout, against a shared clone whose commits carry
// this checkout's HEAD tree plus a one-byte schema edit. The guard reads only git objects, so the
// clone's commits are all it judges.
function realPush(t, message) {
  const dir = mkdtempSync(join(tmpdir(), "prepush-fleet-real-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const gd = join(dir, "fx/.git");
  const id = { GIT_AUTHOR_NAME: "t", GIT_AUTHOR_EMAIL: "t@t", GIT_COMMITTER_NAME: "t", GIT_COMMITTER_EMAIL: "t@t" };
  const git = (args, opts = {}) => {
    const r = spawnSync("git", args, { env: { ...cleanEnv, ...id, ...(opts.env || {}) }, encoding: "utf8", input: opts.input, maxBuffer: 64 * 1024 * 1024 });
    assert.equal(r.status, 0, `git ${args.join(" ")}: ${r.stderr}`);
    return r.stdout.trim();
  };
  const head = git(["-C", ROOT, "rev-parse", "HEAD"]);
  git(["clone", "--shared", "--no-checkout", "--quiet", ROOT, join(dir, "fx")]);
  const idx = { env: { GIT_INDEX_FILE: join(dir, "index") } };
  git(["--git-dir", gd, "read-tree", head], idx);
  const base = git(["--git-dir", gd, "commit-tree", `${head}^{tree}`, "-p", head, "-m", "base"]);
  const schema = "elohim/sdk/schemas/v1/gate-live-event.schema.json";
  const edited = git(["--git-dir", gd, "hash-object", "-w", "--stdin"], { input: `${git(["--git-dir", gd, "show", `${head}:${schema}`])}\n\n` });
  git(["--git-dir", gd, "update-index", "--cacheinfo", `100644,${edited},${schema}`], idx);
  const tip = git(["--git-dir", gd, "commit-tree", git(["--git-dir", gd, "write-tree"], idx), "-p", base, "-m", message]);
  const lines = [`refs/heads/x ${tip} refs/heads/dev ${base}`];
  const body = [`HOOK_WORK_TREE='${ROOT}'`, PRELUDE, "GATE_LIVE_RUN=''", RED_BLOCK, "hook_pass() { exit 0; }", `ZERO_SHA=${"0".repeat(40)}`,
    'is_notes_ref() { case "$1" in refs/notes/*) return 0 ;; esac; return 1; }', "PUSH_LINES=(", ...lines.map((l) => `  "${l}"`), ")",
    "export GATE_PUSH_TARGETS=refs/heads/dev", fleetBlock(), "pass_or_refuse"].join("\n");
  return spawnSync("bash", ["-c", `set -o pipefail\ngate_step_reading() { return "$1"; }\n${body}\n`],
    { cwd: ROOT, env: { ...cleanEnv, GIT_DIR: gd, GIT_WORK_TREE: ROOT, EPR_BIN: "/nonexistent/epr" }, encoding: "utf8", timeout: 60000 });
}

test("fleet-roll, real CLI: the schema edit pushed to dev without a declaration is red and the push is refused", (t) => {
  const r = realPush(t, "feat(sdk): one schema field");
  assert.equal(r.status, 1, r.stdout + r.stderr);
  assert.match(r.stdout, /\[preflight\] REFUSED fleet-roll: CI would select an Edge build and its deploy step for this push to dev: elohim-edge stale steps [^\n]*cargo-build-storage/);
  assert.ok(r.stderr.includes("red before the gates: fleet-roll"), r.stderr);
});

test("fleet-roll, real CLI: the same push with the declaration passes", (t) => {
  const r = realPush(t, "feat(sdk): one schema field\n\nRebuilds storage on purpose. [edge:roll]");
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.match(r.stdout, /\[preflight\] ok fleet-roll \(CI would select an Edge build and its deploy step for this push to dev: .*declared by \[edge:roll\] in [0-9a-f]{12}\)/);
});

test("the hook is one brace group that exits inside it, so an edit during a push cannot reach the run", () => {
  // Bash reads a script as it executes; a same-inode rewrite mid-push (2026-11-01: 6352 s of green
  // gates, then `syntax error near unexpected token )`) is harmless only when everything already parsed.
  const body = HOOK.split("\n").filter((line) => line.trim() && !line.trim().startsWith("#"));
  assert.equal(body[0], "{", "the first executable line opens the group");
  // The last pass is the landing (Phase L, landing queue S4): hook_pass ends in an exit on every
  // path, so the group still exits inside itself.
  assert.deepEqual(body.slice(-2), ["hook_pass", "}"], "the group ends with the landing inside it");
  const pass = HOOK.slice(HOOK.indexOf("\nhook_pass() {"), HOOK.indexOf("\n}\n", HOOK.indexOf("\nhook_pass() {")));
  assert.match(pass, /\[ -n "\$BERTH_TARGETS" \] \|\| \{ stop_point; exit 0; \}/, "a push with no dev/main ref exits at once, after its last stop point");
  assert.match(pass, /\n  exit 0$/, "hook_pass ends in exit");
});
