import assert from "node:assert/strict";
import {
  chmodSync,
  closeSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readFileSync,
  readdirSync,
  readlinkSync,
  rmSync,
  statSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { spawn, spawnSync } from "node:child_process";
import { setTimeout as delay } from "node:timers/promises";
import test from "node:test";
import { openRun, readRun } from "./gate-live.mjs";
import { slotGateLockPath } from "./gate-runner.mjs";

const SCRIPT = join(import.meta.dirname, "run-local-gate.sh");
const TAP = join(import.meta.dirname, "gate-tap.mjs");
const BASE_PATH = "/usr/local/bin:/usr/bin:/bin";

function nodeCliEnv(extra = {}) {
  const env = { ...process.env, ...extra };
  delete env.NODE_TEST_CONTEXT;
  delete env.NODE_TEST_WORKER_ID;
  return env;
}

function executable(path, body = "#!/bin/sh\nexit 0\n") {
  mkdirSync(join(path, ".."), { recursive: true });
  writeFileSync(path, body);
  chmodSync(path, 0o755);
  return path;
}

function fixture(
  t,
  recipe = "@git rev-parse --show-toplevel 2>&1 || true\n    @echo RAN",
) {
  const dir = mkdtempSync(join(tmpdir(), "run-local-gate-"));
  const root = join(dir, "root");
  const projectDir = "project";
  mkdirSync(join(root, projectDir), { recursive: true });
  writeFileSync(join(root, "justfile"), `gate:\n    ${recipe}\n`);
  const ark = executable(join(dir, "ark"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  return { dir, root, projectDir, ark };
}

function run(
  fx,
  {
    projectName = "fixture",
    projectDir = fx.projectDir,
    script = SCRIPT,
    env = {},
  } = {},
) {
  const capture = mkdtempSync(join(tmpdir(), "run-local-gate-output-"));
  const stdoutPath = join(capture, "stdout");
  const stderrPath = join(capture, "stderr");
  const stdoutFd = openSync(stdoutPath, "w");
  const stderrFd = openSync(stderrPath, "w");
  const result = spawnSync(
    "bash",
    [script, fx.root, projectName, projectDir, "root-just", "gate"],
    {
      encoding: "utf8",
      env: nodeCliEnv({
        PATH: BASE_PATH,
        RUSTUP_TOOLCHAIN: "",
        ARK_BIN: fx.ark,
        ...env,
      }),
      stdio: ["ignore", stdoutFd, stderrFd],
    },
  );
  closeSync(stdoutFd);
  closeSync(stderrFd);
  result.stdout = readFileSync(stdoutPath, "utf8");
  result.stderr = readFileSync(stderrPath, "utf8");
  rmSync(capture, { recursive: true, force: true });
  return result;
}

function git(cwd, ...args) {
  const result = spawnSync("git", args, {
    cwd,
    encoding: "utf8",
    env: { ...process.env, PATH: BASE_PATH },
  });
  assert.equal(result.status, 0, result.stderr);
  return result;
}

function slotFixture(t) {
  const fx = fixture(t);
  const bin = join(fx.dir, "bin");
  const slotRoot = join(fx.dir, "slots");
  mkdirSync(join(fx.root, "genesis/agentic/bin"), { recursive: true });
  mkdirSync(join(fx.root, "genesis/orchestrator"), { recursive: true });
  copyFileSync(join(import.meta.dirname, "gate-live.mjs"), join(fx.root, "genesis/orchestrator/gate-live.mjs"));
  copyFileSync(join(import.meta.dirname, "file-lock.mjs"), join(fx.root, "genesis/orchestrator/file-lock.mjs"));
  writeFileSync(join(fx.root, "genesis/agentic/bin/pool-lib.sh"), `
detect_family() { printf '%s\n' "\${GATE_TEST_FAMILY:-family}"; }
slot_path() {
  local workspace="\${2//\//_}"
  printf '%s/%s__%s__%s\n' "$GATE_TEST_SLOT_ROOT" "$1" "$workspace" "$3"
}
`);
  executable(join(bin, "just"), `#!/usr/bin/env bash
set -euo pipefail
now() { date +%s%N; }
printf 'enter\\t%s\\t%s\\n' "$GATE_TEST_LABEL" "$(now)" >> "$GATE_TEST_EVENTS"
if [[ "\${GATE_TEST_PROBE_LOCK:-0}" == 1 ]]; then
  if flock -n "\${CARGO_TARGET_DIR}.gate.lock" -c true; then
    echo LOCK_NOT_HELD
    exit 97
  fi
  echo LOCK_HELD
fi
if [[ -n "\${GATE_TEST_TOKEN_FILE:-}" ]]; then
  printf '%s\\n' "$GATE_SLOT_OWNER_TOKEN" > "$GATE_TEST_TOKEN_FILE"
fi
if [[ -n "\${GATE_TEST_STUBBORN_PID:-}" ]]; then
  # A member of the gate's session that SIGTERM does not end, as a recipe child can be.
  ( trap '' TERM; while :; do sleep 1; done ) </dev/null >/dev/null 2>&1 &
  printf '%s\\n' "$!" > "$GATE_TEST_STUBBORN_PID"
fi
if [[ -n "\${GATE_TEST_READY:-}" ]]; then
  printf 'ready\\n' > "$GATE_TEST_READY"
  while [[ ! -e "$GATE_TEST_RELEASE" ]]; do read -r -t 0.02 _ || true; done
fi
if [[ -n "\${GATE_TEST_NESTED:-}" ]]; then
  "$GATE_TEST_NODE" "$GATE_TEST_TAP" --project nested -- bash -c \
    'printf "nested\\n" >> "$GATE_TEST_EVENTS"'
fi
if [[ -n "\${GATE_TEST_BACKGROUND_PID:-}" ]]; then
  ( trap '' TERM; sleep 30 ) </dev/null >/dev/null 2>&1 &
  printf '%s\\n' "$!" > "$GATE_TEST_BACKGROUND_PID"
fi
if [[ -n "\${GATE_TEST_DAEMON_PID:-}" ]]; then
  setsid sleep 30 </dev/null >/dev/null 2>&1 &
  printf '%s\n' "$!" > "$GATE_TEST_DAEMON_PID"
fi
printf 'exit\\t%s\\t%s\\n' "$GATE_TEST_LABEL" "$(now)" >> "$GATE_TEST_EVENTS"
`);
  return { ...fx, bin, slotRoot, events: join(fx.dir, "events.tsv") };
}

/** SIGKILL every live process whose environment carries GATE_TEST_RUN_TAG=<tag>. */
function killTagged(tag) {
  const needle = `GATE_TEST_RUN_TAG=${tag}`;
  let pids;
  try { pids = readdirSync("/proc").filter(name => /^\d+$/.test(name)); } catch { return; }
  for (const pid of pids) {
    if (Number(pid) === process.pid) continue;
    let environ;
    try { environ = readFileSync(`/proc/${pid}/environ`, "latin1"); } catch { continue; }
    if (environ.split("\0").includes(needle)) {
      try { process.kill(Number(pid), "SIGKILL"); } catch { /* already gone */ }
    }
  }
}

/**
 * What a slot test saw, for its failure message: the next red names its own cause. Every process
 * still carrying a gate's tag (pid, state, parent, process group, session, whether it holds the slot
 * lock, command), every process on the device holding that lock open, the holder file, each gate's
 * wait result and the tail of its stderr (the `[gate-slot]` drain lines among them). Read-only.
 */
function slotDiagnosis(label, gates, sinceMs) {
  const lines = [`${label}; ${Date.now() - sinceMs} ms since the kill`];
  const procs = (() => { try { return readdirSync("/proc").filter(name => /^\d+$/.test(name)); } catch { return []; } })();
  const read = (path, encoding = "utf8") => { try { return readFileSync(path, encoding); } catch { return null; } };
  const holdsFile = (pid, path) => {
    let target;
    try { target = statSync(path); } catch { return false; }
    let fds;
    try { fds = readdirSync(`/proc/${pid}/fd`); } catch { return false; }
    return fds.some(fd => {
      try { const st = statSync(`/proc/${pid}/fd/${fd}`); return st.ino === target.ino && st.dev === target.dev; }
      catch { return false; }
    });
  };
  const describe = (pid, lock) => {
    const stat = read(`/proc/${pid}/stat`);
    const fields = stat ? stat.slice(stat.lastIndexOf(")") + 2).split(" ") : [];
    const cmd = (read(`/proc/${pid}/cmdline`) || "").split("\0").filter(Boolean).join(" ").slice(0, 160);
    let cwd = "";
    try { cwd = readlinkSync(`/proc/${pid}/cwd`); } catch { /* gone */ }
    return `pid=${pid} state=${fields[0] ?? "?"} ppid=${fields[1] ?? "?"} pgrp=${fields[2] ?? "?"} sid=${fields[3] ?? "?"}`
      + ` lock-fd=${holdsFile(pid, lock) ? "open" : "-"} cwd=${cwd} cmd=${cmd}`;
  };
  for (const gate of gates) {
    lines.push(`--- ${gate.label} (tag ${gate.tag}) lock ${gate.lock}`);
    lines.push(`holder file: ${(read(gate.holderPath) || "absent").trim()}`);
    lines.push(`wait result: ${(read(gate.waitResult) || "absent").trim()}`);
    const needle = `GATE_TEST_RUN_TAG=${gate.tag}`;
    const tagged = procs.filter(pid => (read(`/proc/${pid}/environ`, "latin1") || "").split("\0").includes(needle));
    lines.push(`tagged processes alive: ${tagged.length}`);
    for (const pid of tagged) lines.push(`  ${describe(pid, gate.lock)}`);
    const lockers = procs.filter(pid => !tagged.includes(pid) && holdsFile(pid, gate.lock));
    for (const pid of lockers) lines.push(`  another gate's process with the lock open: ${describe(pid, gate.lock)}`);
    // The kernel's own answer to who owns the lock (an open descriptor alone does not: a waiting
    // flock holds the file open too). /proc/locks names the owner pid and the file's inode.
    let ino = null;
    try { ino = statSync(gate.lock).ino; } catch { /* absent */ }
    const owners = (read("/proc/locks") || "").split("\n")
      .filter(line => ino !== null && new RegExp(`:${ino} `).test(`${line} `));
    lines.push(`kernel lock entries for this inode: ${owners.length ? "" : "none"}`);
    for (const line of owners) lines.push(`  ${line.trim()}`);
    lines.push("stderr tail:");
    for (const line of (gate.stderr() || "").trimEnd().split("\n").slice(-40)) lines.push(`  ${line}`);
  }
  return lines.join("\n");
}

function startSlotGate(t, fx, {
  projectName,
  workspace = "workspace-a",
  targetDir = "",
  label = projectName,
  env = {},
} = {}) {
  const slot = targetDir || join(fx.slotRoot, `family__${workspace.replaceAll("/", "_")}__dev`);
  const tag = `rlg-${process.pid}-${label}-${Math.random().toString(36).slice(2)}`;
  const lock = slotGateLockPath(slot);
  const backing = lock.slice(0, -".gate.lock".length);
  const waitResult = join(fx.dir, `${label}.slot-wait.tsv`);
  const stdoutPath = join(fx.dir, `${label}.stdout`);
  const stderrPath = join(fx.dir, `${label}.stderr`);
  const stdoutFd = openSync(stdoutPath, "w");
  const stderrFd = openSync(stderrPath, "w");
  const live = env.GATE_LIVE_RUN ? ["--run", env.GATE_LIVE_RUN] : [];
  const child = spawn(process.execPath, [TAP, "--project", projectName, ...live, "--", "bash",
    SCRIPT, fx.root, projectName, fx.projectDir, "root-just", "gate",
    targetDir ? "" : workspace, targetDir, "dev", "", waitResult, lock,
  ], {
    cwd: fx.root,
    env: nodeCliEnv({
      PATH: `${fx.bin}:${BASE_PATH}`,
      RUSTUP_TOOLCHAIN: "",
      GATE_TEST_SLOT_ROOT: fx.slotRoot,
      GATE_TEST_EVENTS: fx.events,
      GATE_TEST_LABEL: label,
      GATE_TEST_NODE: process.execPath,
      GATE_TEST_TAP: TAP,
      GATE_SLOT_LOCK_PATH: lock,
      GATE_SLOT_HOLDER_PATH: join(backing, ".gate.holder"),
      GATE_SLOT_WAIT_RESULT: waitResult,
      GATE_TEST_RUN_TAG: tag,
      ...env,
    }),
    detached: true,
    stdio: ["ignore", stdoutFd, stderrFd, "pipe"],
  });
  closeSync(stdoutFd);
  closeSync(stderrFd);
  let fd3 = "";
  child.stdio[3].setEncoding("utf8");
  child.stdio[3].on("data", chunk => { fd3 += chunk; });
  const stdout = () => readFileSync(stdoutPath, "utf8");
  const stderr = () => readFileSync(stderrPath, "utf8");
  const closed = new Promise(resolve => child.once("close", (status, signal) => {
    resolve({ status, signal, stdout: stdout(), stderr: stderr(),
      summary: fd3.trim() ? JSON.parse(fd3.trim().split("\n").at(-1)) : null });
  }));
  t.after(() => {
    if (child.exitCode === null && child.signalCode === null) {
      try { process.kill(-child.pid, "SIGKILL"); } catch { /* already exited */ }
    }
    // The slot launcher leads its own session (outside the tap's group), and the slot holder runs
    // under flock in another; a TERM-trapping stub left in either outlived the test once and hung a
    // gate for 27 minutes. Every process of this gate carries the tag, so all of them end here.
    killTagged(tag);
  });
  return { child, closed, stdout, stderr, slot, lock, waitResult, tag, label,
    holderPath: join(backing, ".gate.holder") };
}

async function waitUntil(predicate, message, timeoutMs = 15000) {  // setup waits only; a loaded host (gates run beside cargo) needs headroom
  const deadline = Date.now() + timeoutMs;
  while (!predicate()) {
    if (Date.now() >= deadline) assert.fail(message);
    await delay(10);
  }
}

function readEvents(path) {
  return readFileSync(path, "utf8").trim().split("\n").filter(Boolean).map(line => {
    const [kind, label, at] = line.split("\t");
    return { kind, label, at: BigInt(at) };
  });
}

test("two gates on one cargo slot never overlap and the waiter names the holder", { timeout: 60_000 }, async (t) => {
  const fx = slotFixture(t);
  const ready = join(fx.dir, "first.ready");
  const release = join(fx.dir, "first.release");
  const first = startSlotGate(t, fx, {
    projectName: "first",
    env: { GATE_LIVE_RUN: "run-first", GATE_TEST_READY: ready, GATE_TEST_RELEASE: release },
  });
  await waitUntil(() => {
    try { return readFileSync(ready, "utf8").trim() === "ready"; } catch { return false; }
  }, "first gate never entered its recipe");

  const second = startSlotGate(t, fx, { projectName: "second" });
  await waitUntil(() => second.stderr().includes("cargo slot still busy"), "second gate never reported the holder");
  assert.match(second.stderr(), new RegExp(
    `^\\[gate-tap\\] second: cargo slot still busy after [0-9.]+s; holder: project=first worktree=${fx.root.replaceAll("/", "\\/")} `
      + "pid=[0-9]+ .* run=run-first started=[^ ]+$", "m",
  ));
  writeFileSync(release, "release\n");

  const [one, two] = await Promise.all([first.closed, second.closed]);
  assert.equal(one.status, 0, one.stderr);
  assert.equal(two.status, 0, two.stderr);
  const events = readEvents(fx.events);
  assert.deepEqual(events.map(({ kind, label }) => [kind, label]), [
    ["enter", "first"], ["exit", "first"], ["enter", "second"], ["exit", "second"],
  ]);
  assert.ok(events[1].at <= events[2].at, "the second recipe entered before the first exited");
});

test("ending a gate tap releases its outside-the-group slot holder", { timeout: 60_000 }, async (t) => {
  const fx = slotFixture(t);
  const ready = join(fx.dir, "holder.ready");
  const release = join(fx.dir, "never.release");
  const holder = startSlotGate(t, fx, {
    projectName: "holder",
    // A short TERM grace: if a member of the stopped gate's session misses the holder's TERM too, the
    // KILL after this grace is what frees the slot, inside the 10 s bound below.
    env: { GATE_TEST_READY: ready, GATE_TEST_RELEASE: release, GATE_SLOT_DRAIN_TERM_MS: "500" },
  });
  await waitUntil(() => {
    try { return readFileSync(ready, "utf8").trim() === "ready"; } catch { return false; }
  }, "holder never entered its recipe");
  const waiter = startSlotGate(t, fx, { projectName: "waiter" });
  await waitUntil(() => waiter.stderr().includes("cargo slot still busy"), "waiter never observed contention");

  const killedAt = Date.now();
  holder.child.kill("SIGTERM");
  const waited = await Promise.race([
    waiter.closed,
    delay(10_000).then(() => ({ timeout: true })),
  ]);
  if (waited.timeout !== undefined) {
    assert.fail(slotDiagnosis("waiter did not proceed when the killed holder released the lock", [holder, waiter], killedAt));
  }
  assert.equal(waited.status, 0, waited.stderr);
  if (Date.now() - killedAt >= 10_000) {
    assert.fail(slotDiagnosis("the released slot proceeds within a loaded-host margin", [holder, waiter], killedAt));
  }
  writeFileSync(release, "release\n");
  const killed = await holder.closed;
  assert.equal(killed.status, 143);
});

test("a gate stopped from outside frees its slot without the drain, even when a member ignores SIGTERM", { timeout: 60_000 }, async (t) => {
  const fx = slotFixture(t);
  const ready = join(fx.dir, "stubborn.ready");
  const release = join(fx.dir, "stubborn.never");
  const stubbornPid = join(fx.dir, "stubborn.pid");
  const holder = startSlotGate(t, fx, {
    projectName: "stubborn-holder",
    // The drain stays at its 60 s default: only the stop may shorten it. The TERM grace is short.
    env: { GATE_TEST_READY: ready, GATE_TEST_RELEASE: release, GATE_TEST_STUBBORN_PID: stubbornPid,
      GATE_SLOT_DRAIN_TERM_MS: "500" },
  });
  await waitUntil(() => {
    try { return readFileSync(ready, "utf8").trim() === "ready"; } catch { return false; }
  }, "holder never entered its recipe");
  const waiter = startSlotGate(t, fx, { projectName: "stubborn-waiter" });
  await waitUntil(() => waiter.stderr().includes("cargo slot still busy"), "waiter never observed contention");

  const killedAt = Date.now();
  holder.child.kill("SIGTERM");
  // An unref'd timer: the bound must not keep the test file alive after the waiter is admitted.
  const waited = await Promise.race([waiter.closed, delay(30_000, { timeout: true }, { ref: false })]);
  if (waited.timeout !== undefined) {
    assert.fail(slotDiagnosis("a stopped gate's TERM-ignoring member kept the slot for the drain", [holder, waiter], killedAt));
  }
  assert.equal(waited.status, 0, waited.stderr);
  assert.ok(Date.now() - killedAt < 30_000, "the stop freed the slot well inside the 60 s drain");
  assert.match(holder.stderr(), /\[gate-slot\] session \d+ was stopped; sent SIGTERM without the 60000 ms drain/);
  const pid = Number(readFileSync(stubbornPid, "utf8").trim());
  assert.throws(() => process.kill(pid, 0), "the TERM-ignoring member was killed");
});

test("different workspace slots never wait on each other", { timeout: 60_000 }, async (t) => {
  const fx = slotFixture(t);
  const ready = join(fx.dir, "workspace-a.ready");
  const release = join(fx.dir, "workspace-a.release");
  const first = startSlotGate(t, fx, {
    projectName: "workspace-a-holder",
    workspace: "workspace-a",
    env: { GATE_TEST_READY: ready, GATE_TEST_RELEASE: release },
  });
  await waitUntil(() => {
    try { return readFileSync(ready, "utf8").trim() === "ready"; } catch { return false; }
  }, "workspace-a holder never entered its recipe");

  const startedAt = Date.now();
  const second = startSlotGate(t, fx, { projectName: "workspace-b-gate", workspace: "workspace-b" });
  const result = await Promise.race([
    second.closed,
    delay(10_000).then(() => ({ timeout: true })),
  ]);
  if (result.timeout !== undefined) {
    assert.fail(slotDiagnosis("a different workspace waited on the occupied slot", [first, second], startedAt));
  }
  assert.equal(result.status, 0, result.stderr);
  assert.doesNotMatch(result.stdout, /cargo slot busy/);
  writeFileSync(release, "release\n");
  assert.equal((await first.closed).status, 0);
});

test("a cargo slot timeout refuses with the dedicated did-not-run status", { timeout: 60_000 }, async (t) => {
  const fx = slotFixture(t);
  const ready = join(fx.dir, "timeout-holder.ready");
  const release = join(fx.dir, "timeout-holder.release");
  const holder = startSlotGate(t, fx, {
    projectName: "timeout-holder",
    env: { GATE_TEST_READY: ready, GATE_TEST_RELEASE: release },
  });
  await waitUntil(() => {
    try { return readFileSync(ready, "utf8").trim() === "ready"; } catch { return false; }
  }, "timeout holder never entered its recipe");

  const live = openRun({ root: fx.root, sha: "a".repeat(40), target: "just gate", session: "test" });
  const waiter = startSlotGate(t, fx, {
    projectName: "timeout-waiter",
    env: { GATE_SLOT_LOCK_WAIT_S: "0.1", GATE_LIVE_RUN: live.path },
  });
  const result = await waiter.closed;
  assert.equal(result.status, 73, result.stderr);
  assert.match(result.stderr, /^\[gate-tap\] timeout-waiter: cargo slot still busy after [0-9.]+s; holder: project=timeout-holder .*$/m);
  assert.match(result.stderr, /^gate timeout-waiter: REFUSED cargo slot stayed busy for 0\.1s; holder: project=timeout-holder .*$/m);
  const wait = readRun(live.path).events.find(event => event.kind === "wait");
  assert.equal(wait.project, "timeout-waiter");
  assert.match(wait.holder, /^project=timeout-holder /);
  writeFileSync(release, "release\n");
  assert.equal((await holder.closed).status, 0);
});

test("flock owns the cargo slot lock while the descriptor stays out of the recipe", { timeout: 60_000 }, async (t) => {
  const fx = slotFixture(t);
  const live = openRun({ root: fx.root, sha: "b".repeat(40), target: "just gate", session: "uncontended" });
  const gate = startSlotGate(t, fx, {
    projectName: "lock-probe",
    env: { GATE_TEST_PROBE_LOCK: "1", GATE_LIVE_RUN: live.path },
  });
  const result = await gate.closed;
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /^LOCK_HELD$/m, JSON.stringify(result));
  assert.doesNotMatch(result.stdout, /LOCK_NOT_HELD/);
  assert.equal(spawnSync("test", ["-e", gate.waitResult]).status, 1,
    "an uncontended admission records no slot-wait measurement");
  assert.equal(readRun(live.path).events.some(event => event.kind === "wait"), false,
    "an uncontended admission emits no phantom wait event");
});

test("a waiting gate refreshes its holder heartbeat on stderr and the live run", { timeout: 60_000 }, async (t) => {
  const fx = slotFixture(t);
  const ready = join(fx.dir, "heartbeat-holder.ready");
  const release = join(fx.dir, "heartbeat-holder.release");
  t.after(() => { try { writeFileSync(release, "release\n"); } catch { /* fixture gone */ } });
  const holder = startSlotGate(t, fx, {
    projectName: "heartbeat-holder",
    env: { GATE_TEST_READY: ready, GATE_TEST_RELEASE: release },
  });
  await waitUntil(() => existsSync(ready), "heartbeat holder never entered", 60_000);
  const live = openRun({ root: fx.root, sha: "c".repeat(40), target: "just gate", session: "heartbeat" });
  const waiter = startSlotGate(t, fx, {
    projectName: "heartbeat-waiter",
    env: { GATE_LIVE_RUN: live.path, GATE_SLOT_WAIT_HEARTBEAT_S: "0.02" },
  });
  await waitUntil(() => {
    try { return readRun(live.path).events.filter(event => event.kind === "wait").length >= 2; }
    catch { return false; }
  }, "waiting gate did not refresh its live wait event", 30_000);
  writeFileSync(release, "release\n");
  assert.equal((await holder.closed).status, 0);
  const result = await waiter.closed;
  assert.equal(result.status, 0);
  assert.ok((result.stderr.match(/cargo slot still busy/g) || []).length >= 2,
    `waiting gate did not refresh its stderr heartbeat: ${JSON.stringify(result.stderr)}`);
});

test("SIGINT while waiting for a cargo slot aborts admission with exit 130", { timeout: 60_000 }, async (t) => {
  const fx = slotFixture(t);
  const ready = join(fx.dir, "interrupt-holder.ready");
  const release = join(fx.dir, "interrupt-holder.release");
  const holder = startSlotGate(t, fx, {
    projectName: "interrupt-holder",
    env: { GATE_TEST_READY: ready, GATE_TEST_RELEASE: release },
  });
  await waitUntil(() => existsSync(ready), "interrupt holder never entered");
  const waiter = startSlotGate(t, fx, { projectName: "interrupted-waiter" });
  await waitUntil(() => waiter.stderr().includes("cargo slot still busy"), "waiter never blocked");
  waiter.child.kill("SIGINT");
  const result = await waiter.closed;
  assert.equal(result.status, 130, result.stderr);
  assert.deepEqual(result.summary.stopped, { by: null, signal: "SIGINT", outside: true });
  assert.equal(existsSync(waiter.waitResult), false, "an interrupted wait records no completed wait");
  assert.doesNotMatch(readFileSync(fx.events, "utf8"), /interrupted-waiter/);
  writeFileSync(release, "release\n");
  assert.equal((await holder.closed).status, 0);
});

test("an uncontended gate on a symlinked cargo slot runs against the backing lock", { timeout: 60_000 }, async (t) => {
  const fx = slotFixture(t);
  const backing = join(fx.slotRoot, "backing-slot");
  const linked = join(fx.slotRoot, "linked-slot");
  mkdirSync(backing, { recursive: true });
  symlinkSync(backing, linked, "dir");

  const gate = startSlotGate(t, fx, { projectName: "linked-slot", targetDir: linked });
  const result = await gate.closed;

  assert.equal(gate.lock, `${backing}.gate.lock`);
  assert.equal(result.status, 0, result.stderr);
  assert.doesNotMatch(result.stderr, /runner did not name|cargo slot still busy|REFUSED/);
  assert.deepEqual(readEvents(fx.events).map(({ kind, label }) => [kind, label]), [
    ["enter", "linked-slot"], ["exit", "linked-slot"],
  ]);
});

test("SIGKILL of the tap does not release the slot while its recipe session lives", { timeout: 60_000 }, async (t) => {
  const fx = slotFixture(t);
  const ready = join(fx.dir, "killed-tap.ready");
  const release = join(fx.dir, "killed-tap.release");
  const holder = startSlotGate(t, fx, {
    projectName: "killed-tap",
    env: { GATE_TEST_READY: ready, GATE_TEST_RELEASE: release },
  });
  await waitUntil(() => existsSync(ready), "killed tap's recipe never entered");
  holder.child.kill("SIGKILL");
  const waiter = startSlotGate(t, fx, { projectName: "after-kill" });
  await waitUntil(() => waiter.stderr().includes("cargo slot still busy"), "second gate did not see the surviving holder");
  await delay(250);
  assert.doesNotMatch(readFileSync(fx.events, "utf8"), /after-kill/);
  writeFileSync(release, "release\n");
  assert.equal((await waiter.closed).status, 0);
  const killed = await holder.closed;
  assert.equal(killed.signal, "SIGKILL");
});

test("a nested same-slot gate reuses its verified ancestor's ownership", { timeout: 60_000 }, async (t) => {
  const fx = slotFixture(t);
  const gate = startSlotGate(t, fx, {
    projectName: "outer",
    env: { GATE_TEST_NESTED: "1", GATE_SLOT_LOCK_WAIT_S: "0.2" },
  });
  const result = await gate.closed;
  assert.equal(result.status, 0, result.stderr);
  assert.match(readFileSync(fx.events, "utf8"), /^nested$/m);
  assert.doesNotMatch(result.stderr, /cargo slot still busy/);
});

test("wrong pid, wrong start time and a non-ancestor token never bypass a busy slot", { timeout: 60_000 }, async (t) => {
  const fx = slotFixture(t);
  const ready = join(fx.dir, "token-holder.ready");
  const release = join(fx.dir, "token-holder.release");
  const tokenFile = join(fx.dir, "owner.token");
  t.after(() => { try { writeFileSync(release, "release\n"); } catch { /* fixture gone */ } });
  const holder = startSlotGate(t, fx, {
    projectName: "token-holder",
    env: { GATE_TEST_TOKEN_FILE: tokenFile, GATE_TEST_READY: ready, GATE_TEST_RELEASE: release },
  });
  await waitUntil(() => existsSync(ready) && existsSync(tokenFile), "holder did not publish its token");
  const token = JSON.parse(readFileSync(tokenFile, "utf8"));
  const variants = [
    { ...token, holder: { ...token.holder, pid: token.holder.pid + 1 } },
    { ...token, holder: { ...token.holder, start: String(Number(token.holder.start) + 1) } },
    token,
  ];
  for (const [index, forged] of variants.entries()) {
    const gate = startSlotGate(t, fx, {
      projectName: `forged-${index}`,
      label: `forged-${index}`,
      env: { GATE_SLOT_OWNER_TOKEN: JSON.stringify(forged), GATE_SLOT_LOCK_WAIT_S: "0.1" },
    });
    const result = await gate.closed;
    assert.equal(result.status, 73, result.stderr);
  }
  writeFileSync(release, "release\n");
  assert.equal((await holder.closed).status, 0);
});

test("slot drain is bounded, heartbeats, and KILLs a TERM-ignoring session member", { timeout: 60_000 }, async (t) => {
  const fx = slotFixture(t);
  const pidFile = join(fx.dir, "background.pid");
  const started = Date.now();
  const gate = startSlotGate(t, fx, {
    projectName: "bounded-drain",
    env: { GATE_TEST_BACKGROUND_PID: pidFile, GATE_SLOT_DRAIN_MS: "100",
      GATE_SLOT_DRAIN_TERM_MS: "100", GATE_SLOT_DRAIN_HEARTBEAT_MS: "20" },
  });
  const result = await gate.closed;
  const tookMs = Date.now() - started;
  assert.equal(result.status, 0, result.stderr);
  assert.ok(tookMs < 10_000, `drain took ${tookMs} ms`);
  const heartbeatAt = result.stderr.indexOf("draining session");
  const killAt = result.stderr.indexOf("sent SIGKILL");
  assert.ok(heartbeatAt >= 0 && heartbeatAt < killAt,
    `the drain heartbeat must precede TERM grace and KILL: ${JSON.stringify(result.stderr)}`);
  assert.match(result.stderr, /exceeded the 100 ms drain and 100 ms TERM grace; sent SIGKILL/);
  const pid = Number(readFileSync(pidFile, "utf8"));
  assert.notEqual(spawnSync("kill", ["-0", String(pid)]).status, 0, "the session member is gone");
});

test("a non-slot gate starts no slot session scan", { timeout: 60_000 }, (t) => {
  const fx = fixture(t);
  const trace = join(fx.dir, "proc-scans");
  const result = spawnSync(process.execPath, [TAP, "--project", "non-slot", "--", "bash", "-c", "exit 0"], {
    encoding: "utf8", stdio: ["ignore", "pipe", "pipe", "pipe"],
    env: nodeCliEnv({ GATE_SLOT_PROC_TRACE: trace }),
  });
  assert.equal(result.status, 0, result.stderr);
  assert.equal(existsSync(trace), false);
});

test("a_callers_ceiling_is_appended_and_honoured", { timeout: 60_000 }, (t) => {
  const fx = fixture(t);
  git(fx.dir, "init", "--quiet");

  const result = run(fx, { env: { GIT_CEILING_DIRECTORIES: fx.dir } });

  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /not a git repository/);
  assert.doesNotMatch(result.stdout, new RegExp(`^${fx.dir}$`, "m"));
  assert.match(result.stdout, /RAN/);
});

test("the_ceiling_starts_with_tmp_and_var_tmp", { timeout: 60_000 }, (t) => {
  const fx = fixture(t, '@echo "$GIT_CEILING_DIRECTORIES"\n    @echo RAN');

  const result = run(fx, { env: { GIT_CEILING_DIRECTORIES: fx.dir } });

  assert.equal(result.status, 0, result.stderr);
  assert.match(
    result.stdout,
    new RegExp(`/tmp:/var/tmp:${fx.dir.replaceAll("/", "\\/")}`),
  );
  assert.match(result.stdout, /RAN/);
});

test("a_tempdir_repo_still_finds_its_own_git", { timeout: 60_000 }, (t) => {
  const fx = fixture(t);
  git(fx.root, "init", "--quiet");

  const result = run(fx);

  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, new RegExp(`^${fx.root}$`, "m"));
  assert.match(result.stdout, /RAN/);
});

// E12 (b): cc reads the C flags for one target from a triple-suffixed variable
// (CFLAGS_x86_64_unknown_linux_gnu), so a declared cargo env key may carry lowercase after its
// leading capital. Anything else still refuses before the recipe runs.
test("a_triple_suffixed_c_flags_key_is_exported_to_the_recipe", { timeout: 60_000 }, (t) => {
  const fx = fixture(t, '@echo "C=$CFLAGS_x86_64_unknown_linux_gnu"\n    @echo RAN');

  const result = run(fx, {
    env: { GATE_CARGO_ENV: JSON.stringify({ CFLAGS_x86_64_unknown_linux_gnu: "-march=x86-64" }) },
  });

  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /^C=-march=x86-64$/m);
  assert.match(result.stdout, /RAN/);
});

test("a_malformed_cargo_env_key_still_refuses", { timeout: 60_000 }, (t) => {
  for (const key of ["lowercase_first", "1LEADING_DIGIT", "HAS-DASH", "HAS SPACE", "_LEADING"]) {
    const fx = fixture(t);
    const result = run(fx, { env: { GATE_CARGO_ENV: JSON.stringify({ [key]: "1" }) } });
    assert.equal(result.status, 2, `${key}: ${result.stderr}`);
    assert.match(result.stderr, /refusing malformed cargo\.env key/);
    assert.doesNotMatch(result.stdout, /RAN/);
  }
});

test("documented limit: a double-forked setsid escapee is outside slot custody", { timeout: 60_000 }, async (t) => {
  const fx = slotFixture(t);
  const daemonPid = join(fx.dir, "daemon.pid");
  const gate = startSlotGate(t, fx, {
    projectName: "daemon",
    env: { GATE_TEST_DAEMON_PID: daemonPid },
  });
  const result = await gate.closed;
  assert.equal(result.status, 0, result.stderr);
  await waitUntil(() => {
    try { return readFileSync(daemonPid, "utf8").trim().length > 0; } catch { return false; }
  }, "the detached child did not publish its pid");
  const pid = Number(readFileSync(daemonPid, "utf8").trim());
  t.after(() => { try { process.kill(pid, "SIGKILL"); } catch { /* already exited */ } });
  assert.equal(spawnSync("kill", ["-0", String(pid)]).status, 0, "the detached child still lives");
  assert.equal(spawnSync("flock", ["-n", gate.lock, "true"]).status, 0,
    "the independent session is deliberately outside the slot lifetime");
  process.kill(pid, "SIGKILL");
});
