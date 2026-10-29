// Hermetic tests for scripts/ci/rollout-watch.sh: a fake kubectl on PATH plays
// a scripted rollout and a scripted sequence of pod reads. No cluster is read.
import test from "node:test";
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const script = fileURLToPath(new URL("./rollout-watch.sh", import.meta.url));
const repoRoot = fileURLToPath(new URL("../../", import.meta.url));
const jenkinsfile = readFileSync(join(repoRoot, "elohim/holochain/Jenkinsfile"), "utf8");
const US = "\x1f";
const RS = "\x1e";

const FAKE_KUBECTL = `#!${process.execPath}
const fs = require('node:fs');
const args = process.argv.slice(2);
const dir = process.env.FIXTURE;
const scenario = JSON.parse(fs.readFileSync(dir + '/scenario.json', 'utf8'));
fs.appendFileSync(dir + '/calls', JSON.stringify(args) + '\\n');
if (args[0] === 'rollout' && args[1] === 'status') {
  fs.writeFileSync(dir + '/rollout.pid', String(process.pid));
  console.log('Waiting for ' + args[2] + ' rollout to finish');
  const r = scenario.rollout;
  if (r.hang) { setTimeout(() => process.exit(0), 60000); return; }
  setTimeout(() => { console.log(r.code === 0 ? 'rollout complete' : 'error: timed out'); process.exit(r.code); }, r.afterMs);
  return;
}
if (args[0] === 'get' && args[1].startsWith('statefulset/')) {
  if (scenario.stsFails) process.exit(1);
  process.stdout.write(scenario.revision + '\\x1f' + 'app=peer,');
  process.exit(0);
}
if (args[0] === 'get' && args[1] === 'pods') {
  if (scenario.podsFail) { console.error('Forbidden'); process.exit(1); }
  const counter = dir + '/pod-reads';
  const n = existsSync(counter) ? Number(fs.readFileSync(counter, 'utf8')) : 0;
  fs.writeFileSync(counter, String(n + 1));
  const polls = scenario.polls;
  process.stdout.write(polls[Math.min(n, polls.length - 1)]);
  process.exit(0);
}
console.error('unexpected kubectl invocation: ' + args.join(' '));
process.exit(97);
function existsSync(p) { return fs.existsSync(p); }
`;

// One container-status record exactly as the script's go-template emits it.
function rec({ pod = "peer-0", rev = "rev-new", deleted = "<no value>", container = "elohim-node",
  restarts = 0, waiting = "", lastReason = "", lastExit = "", lastMessage = "" } = {}) {
  return [pod, rev, deleted, container, String(restarts), waiting, lastReason, lastExit, lastMessage].join(US) + RS;
}

async function run(t, scenario, { kind = "statefulset", env = {} } = {}) {
  const dir = await mkdtemp(join(tmpdir(), "rollout-watch-"));
  t.after(() => rm(dir, { recursive: true, force: true }));
  await mkdir(join(dir, "bin"));
  await writeFile(join(dir, "bin/kubectl"), FAKE_KUBECTL, { mode: 0o755 });
  await writeFile(join(dir, "scenario.json"), JSON.stringify({ revision: "rev-new", polls: [""], ...scenario }));
  const out = join(dir, "evidence");
  const started = Date.now();
  const child = spawn("bash", [script, kind, "peer", "elohim-alpha", "600s", out], {
    env: { ...process.env, PATH: `${dir}/bin:${process.env.PATH}`, FIXTURE: dir,
      ROLLOUT_FASTFAIL_POLL_SECS: "1", ...env },
  });
  let stdout = "", stderr = "";
  child.stdout.on("data", (d) => { stdout += d; });
  child.stderr.on("data", (d) => { stderr += d; });
  const code = await new Promise((resolve) => child.on("close", resolve));
  const calls = existsSync(join(dir, "calls"))
    ? readFileSync(join(dir, "calls"), "utf8").trim().split("\n").map((l) => JSON.parse(l)) : [];
  const artifact = join(out, `elohim-alpha--${kind}--peer`);
  const read = (f) => (existsSync(join(artifact, f)) ? readFileSync(join(artifact, f), "utf8") : null);
  const rolloutPid = existsSync(join(dir, "rollout.pid")) ? Number(readFileSync(join(dir, "rollout.pid"), "utf8")) : null;
  return { code, stdout, stderr, calls, elapsed: Date.now() - started, reason: read("fast-fail.reason"),
    detail: read("fast-fail.txt"), rolloutPid };
}

const alive = (pid) => { try { process.kill(pid, 0); return true; } catch { return false; } };
const podReads = (calls) => calls.filter((c) => c[0] === "get" && c[1] === "pods").length;

test("healthy rollout returns the moment kubectl does — no poll, no added latency", async (t) => {
  const r = await run(t, { rollout: { code: 0, afterMs: 200 } }, { env: { ROLLOUT_FASTFAIL_POLL_SECS: "5" } });
  assert.equal(r.code, 0, r.stderr);
  assert.ok(r.elapsed < 3000, `healthy rollout took ${r.elapsed}ms with a 5s poll`);
  assert.equal(podReads(r.calls), 0);
  assert.equal(r.reason, null);
  assert.match(r.stdout, /rollout complete/);
  assert.deepEqual(r.calls[0], ["rollout", "status", "statefulset/peer", "-n", "elohim-alpha", "--timeout=600s"]);
});

test("healthy pods on the update revision never fast-fail; the rollout's own exit stands", async (t) => {
  const r = await run(t, {
    rollout: { code: 0, afterMs: 2500 },
    polls: [rec({ restarts: 0 }), rec({ restarts: 0 }) + rec({ container: "elohim-conductor", restarts: 1 })],
  });
  assert.equal(r.code, 0, r.stdout + r.stderr);
  assert.ok(podReads(r.calls) >= 1, "pods were never read while the rollout ran");
  assert.equal(r.reason, null);
});

test("CrashLoopBackOff on the update revision fast-fails with waiting and last-terminated reasons", async (t) => {
  const r = await run(t, {
    rollout: { hang: true },
    polls: [rec({ restarts: 3, waiting: "CrashLoopBackOff", lastReason: "Error", lastExit: "1",
      lastMessage: "open /data/blobs: Disk quota exceeded\n(os error 122)" })],
  });
  assert.equal(r.code, 4, r.stdout + r.stderr);
  assert.ok(r.elapsed < 10000, `fast-fail took ${r.elapsed}ms`);
  const expected = "pod peer-0 container elohim-node: CrashLoopBackOff after 3 restarts; last terminated Error (exit 1): open /data/blobs: Disk quota exceeded (os error 122)";
  assert.equal(r.reason, `${expected}\n`);
  assert.match(r.stdout, /FAST-FAIL statefulset\/peer in elohim-alpha after \d+s \(not waiting out 600s\)/);
  assert.match(r.detail, /workload=statefulset\/peer/);
  assert.match(r.detail, /restarts=3 waiting=CrashLoopBackOff lastTerminated=Error\/1/);
  assert.ok(r.rolloutPid && !alive(r.rolloutPid), "the rollout wait was left running");
});

test("CrashLoopBackOff below the restart floor keeps waiting; the rollout timeout stands", async (t) => {
  const r = await run(t, {
    rollout: { code: 1, afterMs: 2500 },
    polls: [rec({ restarts: 1, waiting: "CrashLoopBackOff", lastReason: "Error", lastExit: "1" })],
  });
  assert.equal(r.code, 1);
  assert.equal(r.reason, null);
});

test("restartCount rising on consecutive polls fast-fails without CrashLoopBackOff", async (t) => {
  const r = await run(t, {
    rollout: { hang: true },
    polls: [0, 1, 2, 3].map((n) => rec({ restarts: n, lastReason: "OOMKilled", lastExit: "137" })),
  });
  assert.equal(r.code, 4, r.stdout + r.stderr);
  assert.match(r.reason, /restartCount rose on 3 consecutive polls \(now 3\); last terminated OOMKilled \(exit 137\)/);
});

test("a single restart between polls is not a crashloop", async (t) => {
  const r = await run(t, {
    rollout: { code: 0, afterMs: 3500 },
    polls: [rec({ restarts: 0 }), rec({ restarts: 1 }), rec({ restarts: 1 }), rec({ restarts: 1 })],
  });
  assert.equal(r.code, 0, r.stdout + r.stderr);
  assert.equal(r.reason, null);
});

test("a crashlooping pod of an OLDER revision or a terminating pod is never judged", async (t) => {
  const crash = { restarts: 9, waiting: "CrashLoopBackOff", lastReason: "Error", lastExit: "1" };
  const r = await run(t, {
    rollout: { code: 0, afterMs: 2500 },
    polls: [rec({ ...crash, rev: "rev-old" }) + rec({ ...crash, pod: "peer-1", deleted: "2026-10-29T10:00:00Z" })],
  });
  assert.equal(r.code, 0, r.stdout + r.stderr);
  assert.equal(r.reason, null);
});

test("an init container crashloop fast-fails too", async (t) => {
  const r = await run(t, {
    rollout: { hang: true },
    polls: [rec({ container: "init-data", restarts: 4, waiting: "CrashLoopBackOff", lastReason: "Error", lastExit: "2" })],
  });
  assert.equal(r.code, 4);
  assert.match(r.reason, /container init-data: CrashLoopBackOff after 4 restarts/);
});

test("an unreadable pod list never fails the rollout", async (t) => {
  const r = await run(t, { rollout: { code: 0, afterMs: 2500 }, podsFail: true });
  assert.equal(r.code, 0);
  assert.match(r.stderr, /pod read for statefulset\/peer failed \(pods get\)/);
  assert.equal(r.stderr.match(/pod read for/g).length, 1, "read failure warned more than once");
});

test("an unresolved update revision skips the poll", async (t) => {
  const r = await run(t, {
    revision: "<no value>",
    rollout: { code: 0, afterMs: 2500 },
    polls: [rec({ rev: "<no value>", restarts: 5, waiting: "CrashLoopBackOff" })],
  });
  assert.equal(r.code, 0);
  assert.equal(podReads(r.calls), 0);
});

test("a deployment, ROLLOUT_FASTFAIL=0 or a malformed knob runs the plain wait", async (t) => {
  const crash = { rollout: { code: 1, afterMs: 1500 },
    polls: [rec({ restarts: 9, waiting: "CrashLoopBackOff" })] };
  for (const [kind, env] of [["deployment", {}], ["statefulset", { ROLLOUT_FASTFAIL: "0" }],
    ["statefulset", { ROLLOUT_FASTFAIL_MIN_RESTARTS: "three" }]]) {
    const r = await run(t, crash, { kind, env });
    assert.equal(r.code, 1, `${kind} ${JSON.stringify(env)}`);
    assert.equal(r.calls.length, 1, `${kind} ${JSON.stringify(env)} read pods`);
    assert.deepEqual(r.calls[0].slice(0, 3), ["rollout", "status", `${kind}/peer`]);
    assert.equal(r.reason, null);
  }
});

test("the watcher is read-only, and its go-templates never read a field under an absent parent", () => {
  const body = readFileSync(script, "utf8");
  // kubectl go-template: `.a.b` errors "nil pointer evaluating interface {}.b"
  // when `.a` is absent; every nested read must sit under a `with`.
  const templates = body.split("\n").filter((l) => /^[A-Z_]+_TEMPLATE=|^CONTAINER_FIELDS=/.test(l)).join("\n");
  assert.ok(templates.includes("PODS_TEMPLATE="), "templates not found");
  assert.doesNotMatch(templates, /\.lastState\.terminated|\.state\.waiting|index \.metadata\.labels|\.spec\.selector\.matchLabels|\.status\.updateRevision/);
  assert.doesNotMatch(body, /kubectl (apply|delete|patch|replace|scale|annotate|label|exec|rollout (restart|undo|pause|resume))/);
});

test("edge Jenkinsfile waits through the watcher and records the fast-fail", () => {
  const def = jenkinsfile.slice(jenkinsfile.indexOf("def waitForRolloutWithEvidence("),
    jenkinsfile.indexOf("def rolloutFastFailFor("));
  assert.match(def, /bash '\$\{env\.WORKSPACE\}\/scripts\/ci\/rollout-watch\.sh' '\$\{resourceType\}' '\$\{resourceName\}' '\$\{namespace\}' '\$\{timeout\}'/);
  assert.doesNotMatch(def, /kubectl rollout status/);
  assert.match(def, /rolloutStatus == 4/);
  assert.match(def, /ROLLOUT_FAST_FAILS\[resourceName\.toString\(\)\] = fastFail/);
  // Evidence capture still runs on every failure, fast-fail included.
  assert.match(def, /capture-rollout-evidence\.sh/);
  assert.match(jenkinsfile, /@groovy\.transform\.Field\s+def ROLLOUT_FAST_FAILS = java\.util\.Collections\.synchronizedMap/);
  const live = jenkinsfile.split("\n").filter((l) => /kubectl rollout status/.test(l) && !/^\s*(\*|\/\/)/.test(l));
  assert.deepEqual(live, [], "a rollout wait bypasses the watcher");
});

test("edge Jenkinsfile skips the roll gate for a fast-failed peer and names it in junit", () => {
  const gate = jenkinsfile.slice(jenkinsfile.indexOf("if (step.gate) {"), jenkinsfile.indexOf("if (!runPeerRollGate("));
  assert.match(gate, /else if \(rolloutFastFailFor\(byName\[rollState\.lastRolled\]\)\)/);
  assert.match(gate, /SKIPPED — .*rollout fast-failed/);
  assert.match(jenkinsfile, /fastFail: rolloutFastFailFor\(hc\)/);
  assert.match(jenkinsfile, /rollout FAST-FAILED on a crashlooping pod/);
  // CPS 64KB method-size ratchet (sequenced-roll.test.sh owns the baseline).
  assert.ok((jenkinsfile.match(/sh """/g) ?? []).length <= 10, "an inline sh heredoc was added");
});
