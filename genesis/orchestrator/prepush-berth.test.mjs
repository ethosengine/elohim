import { test } from "node:test";
import assert from "node:assert/strict";
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { MARKERS } from "./gate-landing-markers.mjs";

const ROOT = fileURLToPath(new URL("../../", import.meta.url));
const ZERO = "0".repeat(40),
  BASE = "1".repeat(40),
  HEAD = "2".repeat(40);
const cleanEnv = Object.fromEntries(
  Object.entries(process.env).filter(
    ([k]) => !k.startsWith("GIT_") || k.startsWith("GIT_CONFIG_"),
  ),
);

function sliceHook(a, b) {
  const s = readFileSync(join(ROOT, ".husky/pre-push.bash"), "utf8");
  const x = s.indexOf(a),
    y = s.indexOf(b, x);
  assert.ok(x >= 0 && y > x, `actual hook slice ${a}`);
  return s.slice(x, y);
}
const hookBlock = () =>
  sliceHook("# ── Berth: integration claims", "# ── Berth: claims taken");
const phaseLBlock = () =>
  sliceHook("# ── Phase L: land in the CAS window", "# ── Phase L: end");

const STUB_GIT = `#!/usr/bin/env bash
here="$(cd "$(dirname "$0")" && pwd)"
case "$1" in
 config) echo fixture@example.test;; cat-file) exit 0;; log) cat "$here/message";;
 fetch) [ -z "\${FETCH_SLEEP-}" ] || sleep "$FETCH_SLEEP"; exit 0;;
 merge-base) exit "\${MOVED:-0}";;
 ls-remote) echo x >> "$here/ls-remote.log"; [ -z "\${LS_REMOTE_HANG-}" ] || { trap '' TERM; sleep 30; }; [ -z "\${LS_REMOTE_RC-}" ] || exit "$LS_REMOTE_RC"; if [ -f "$here/remote-sha" ]; then printf '%s\\trefs/heads/dev\\n' "$(cat "$here/remote-sha")"; else [ -z "\${REMOTE_SHA+x}" ] || printf '%s\\trefs/heads/dev\\n' "$REMOTE_SHA"; fi;;
 rev-parse) case "$*" in *--git-common-dir*) echo "$HOOK_WORK_TREE/common";; *FETCH_HEAD*) echo "\${FETCH_HEAD_SHA:-${BASE}}";; *) echo "\${HEAD_SHA:-${HEAD}}";; esac;;
 diff|rebase) exit 0;; *) echo "unexpected git: $*" >&2; exit 99;;
esac
`;

const STUB_BERTH = `#!/usr/bin/env bash
here="$(cd "$(dirname "$0")" && pwd)"
printf '%s|%s|sid=%s|owner=%s|ppid=%s|self=%s\\n' "$(date +%s.%N)" "$*" "\${ELOHIM_SESSION_ID-}" "\${ELOHIM_SESSION_OWNER_PID-}" "$PPID" "$$" >> "$here/calls.log"
mode(){ node -e 'let j=JSON.parse(process.env.BERTH_MODES||"{}");process.stdout.write(String(j[process.argv[1]]??"0"))' "$1"; }
case "$1" in
 session-show) if [ "\${SESSION_HELD-}" = 1 ]; then echo '{"mooring":{"session":"claude-9"},"leases":{"dev-integration":{}}}'; else echo '{"mooring":{"session":"claude-9"},"leases":{}}'; fi;;
 moor) exit "\${MOOR_RC:-0}";; unmoor) exit 0;;
 owner-check) m=$(mode "owner:$2"); [ "$m" = 0 ] && exit 0; echo "$2: REFUSED — held by a watcher" >&2; exit "$m";;
 release) m=$(mode "release:$2"); [ "$m" = 0 ] || { echo "$2: release failed" >&2; exit 1; }; echo "$2: released"; exit 0;;
 claim)
  [ ! -f "$here/hang" ] || { trap '' TERM; sleep 300; }
  [ ! -f "$here/refuse" ] || { echo "$2: REFUSED — held by did:key:z6MkOther/claude-2" >&2; exit 3; }
  n=$(grep -c "|claim $2 " "$here/calls.log" 2>/dev/null || true); m=$(mode "$2"); token=1001.125
  [ "$m" != N ] || [ "$n" -lt 2 ] || token=1009.500
  case "$m" in
   3) echo "$2: REFUSED — held by did:key:z6MkDeviceA/claude-1" >&2; exit 3;;
   F) if [ "$n" -le 1 ]; then echo "$2: REFUSED — self-demoted: no head from did:key:z6MkDeviceA within 570s" >&2; exit 3; fi;;
   U) echo "$2: renewed here, but the renew record did not reach this device's channel store" >&2; exit 1;;
   L) if [ "$n" -eq 2 ]; then echo "berth: epr session: REFUSED lock-timeout: store lock remained busy for 5s" >&2; exit 1; fi;;
   K) if [ "$n" -ge 2 ]; then echo "berth: epr session: REFUSED lock-timeout: store lock remained busy for 5s" >&2; exit 1; fi;;
   E) echo Traceback >&2; exit 1;;
   W|D)
    f="$here/override-at"; [ -f "$f" ] || printf %s "$(( $(date +%s%3N) + \${STUB_OVERRIDE_IN_MS:-1500} ))" > "$f"; at=$(cat "$f")
    if [ "$(date +%s%3N)" -lt "$at" ]; then extra=""; [ "$m" != D ] || extra=" (this device's peer daemon is not running: start it with epr peer serve)"; echo "$2: REFUSED — unconfirmed: sibling$extra — override on the record" >&2; echo "  override_at_ms=$at claimed_at=1001.125" >&2; exit 3; fi;;
  esac
  [ "$n" -gt 1 ] || [ "$m" = R ] && verb=renewed || verb=claimed
  echo "$2: $verb by \${ELOHIM_SESSION_ID:-claude-9} claimed_at=$token";;
esac
`;

function fixture(t, message = "feat: land") {
  const dir = mkdtempSync(join(tmpdir(), "prepush-berth-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const bin = join(dir, "genesis/agentic/bin");
  mkdirSync(bin, { recursive: true });
  mkdirSync(join(dir, "common"));
  writeFileSync(join(bin, "message"), message);
  writeFileSync(join(bin, "git"), STUB_GIT);
  writeFileSync(join(bin, "berth"), STUB_BERTH);
  chmodSync(join(bin, "git"), 0o755);
  chmodSync(join(bin, "berth"), 0o755);
  writeFileSync(
    join(dir, "genesis/agentic/pool-policy.json"),
    JSON.stringify({
      berth: {
        channel: { max_skew_s: 0, fence_margin_s: 0, confirm_wait_s: 0 },
      },
    }),
  );
  return { dir, bin, common: join(dir, "common"), base: BASE, head: HEAD };
}
const deadPid = () =>
  spawnSync("sh", ["-c", "echo $$"], { encoding: "utf8" }).stdout.trim();
function scriptFor(fx, o = {}) {
  const ref = o.ref ?? "refs/heads/dev",
    local = o.local ?? fx.head,
    after = o.after ?? "echo GATES-RAN; hook_pass";
  return `set -o pipefail\nexport HOOK_PID=$$\nZERO_SHA=${ZERO}\nexport HOOK_WORK_TREE='${fx.dir}'\nPUSH_LINES=('refs/heads/feature ${local} ${ref} ${fx.base}')\n${hookBlock()}${phaseLBlock()}${after}\n`;
}
function envFor(fx, e = {}) {
  return {
    ...cleanEnv,
    PATH: `${fx.bin}:${cleanEnv.PATH}`,
    ELOHIM_DEVICE_KEY_FILE: join(fx.dir, "none"),
    EPR_BIN: "",
    ELOHIM_SESSION_ID: "",
    ELOHIM_SESSION_OWNER_PID: "",
    BERTH_MODES: "{}",
    PHASE_L_PUSH_PID: deadPid(),
    REMOTE_SHA: fx.head,
    ...e,
  };
}
function calls(fx) {
  return existsSync(join(fx.bin, "calls.log"))
    ? readFileSync(join(fx.bin, "calls.log"), "utf8")
        .trim()
        .split("\n")
        .filter(Boolean)
    : [];
}
const argv = (l) => l.split("|")[1];
const field = (l, k) =>
  l
    .split("|")
    .find((x) => x.startsWith(`${k}=`))
    ?.slice(k.length + 1) ?? "";
function runBlock(fx, o = {}) {
  const r = spawnSync("bash", ["-c", scriptFor(fx, o)], {
    cwd: fx.dir,
    timeout: o.timeout ?? 12000,
    encoding: "utf8",
    env: envFor(fx, o.env),
  });
  return { ...r, out: `${r.stdout}${r.stderr}`, calls: calls(fx) };
}
function runAsync(fx, o = {}) {
  const child = spawn("bash", ["-c", scriptFor(fx, o)], {
    cwd: fx.dir,
    env: envFor(fx, o.env),
    stdio: ["ignore", "pipe", "pipe"],
  });
  let a = "",
    b = "";
  child.stdout.on("data", (x) => (a += x));
  child.stderr.on("data", (x) => (b += x));
  const done = new Promise((resolve) =>
    child.on("close", (status, signal) =>
      resolve({ status, signal, out: a + b, calls: calls(fx) }),
    ),
  );
  return { child, done };
}
async function pollCalls(fx, p, ms = 5000) {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    const c = calls(fx);
    if (p(c)) return c;
    await new Promise((r) => setTimeout(r, 25));
  }
  const c = calls(fx);
  assert.ok(p(c), `poll timeout:\n${c.join("\n")}`);
  return c;
}
async function pollSettle(fx, p, ms = 5000) {
  const path = join(fx.common, "landing/settle.log"),
    end = Date.now() + ms;
  while (Date.now() < end) {
    const s = existsSync(path) ? readFileSync(path, "utf8") : "";
    if (p.test(s)) return s;
    await new Promise((r) => setTimeout(r, 25));
  }
  const s = existsSync(path) ? readFileSync(path, "utf8") : "";
  assert.match(s, p);
  return s;
}
const release = (c) =>
  c.find((l) => argv(l).startsWith("release dev-integration "));
async function stop(child) {
  if (child.exitCode === null) child.kill("SIGTERM");
  await new Promise((r) => child.once("close", r));
}

test("prepush_refuses_at_the_landing_while_a_peer_holds_dev_integration", async (t) => {
  const fx = fixture(t),
    r = runBlock(fx, { env: { BERTH_MODES: '{"dev-integration":"3"}' } });
  assert.equal(r.status, 1, r.out);
  assert.ok(r.out.indexOf("GATES-RAN") < r.out.indexOf("REFUSED"), r.out);
  assert.match(r.out, /Your gates are recorded/);
  assert.equal(
    r.calls.some((l) => argv(l).startsWith("release ")),
    false,
  );
  await pollCalls(fx, (c) =>
    c.some((l) => argv(l).startsWith("unmoor --session pre-push-")),
  );
});
test("prepush_gate_failure_makes_no_berth_call", (t) => {
  const fx = fixture(t),
    r = runBlock(fx, { after: "echo GATE-FAILED; exit 1" });
  assert.equal(r.status, 1, r.out);
  assert.deepEqual(r.calls, []);
});
test("prepush_passing_push_releases_only_after_the_push_exits", async (t) => {
  const fx = fixture(t),
    s = spawn("sleep", ["30"]);
  t.after(() => s.kill("SIGKILL"));
  const r = runBlock(fx, { env: { PHASE_L_PUSH_PID: String(s.pid) } });
  assert.equal(r.status, 0, r.out);
  await new Promise((x) => setTimeout(x, 1100));
  assert.equal(release(calls(fx)), undefined);
  await stop(s);
  const c = await pollCalls(fx, (x) => !!release(x));
  assert.match(argv(release(c)), /--expected-claimed-at 1001\.125$/);
  await pollSettle(
    fx,
    /settle \S+ (landed|did-not-land) dev=\S* tool=git held=\d+s released=ok/,
  );
});
test("prepush_agent_owns_its_session", async (t) => {
  const fx = fixture(t),
    s = spawn("sleep", ["30"]);
  t.after(() => s.kill("SIGKILL"));
  const h = runAsync(fx, { env: { PHASE_L_PUSH_PID: String(s.pid) } }),
    r = await h.done;
  assert.equal(r.status, 0, r.out);
  await stop(s);
  const c = await pollCalls(fx, (x) => !!release(x)),
    m = c.find((l) => argv(l).startsWith("moor --session pre-push-")),
    pid = argv(m).match(/ --pid (\d+) /)[1],
    sid = field(m, "sid"),
    ac = c.filter((l) => field(l, "sid") === sid);
  assert.ok(
    ac.every((l) => field(l, "owner") === pid),
    `phase_l_agent owner must equal moor --pid ${pid}:\n${ac.join("\n")}`,
  );
});
test("build_edge_owner_checks_alpha_fleet_and_jenkins_dispatch", async (t) => {
  let fx = fixture(t, "deploy [build:edge]"),
    r = runBlock(fx, { env: { BERTH_MODES: '{"owner:alpha-fleet":"3"}' } });
  assert.equal(r.status, 1, r.out);
  assert.equal(r.calls.filter((l) => argv(l).startsWith("claim ")).length, 1);
  assert.deepEqual(
    r.calls.filter((l) => argv(l).startsWith("owner-check ")).map(argv),
    ["owner-check alpha-fleet --action push to dev"],
  );
  assert.match(r.out, /is held by a watcher/);
  await pollCalls(fx, (c) => !!release(c));
  fx = fixture(t, "deploy [build:edge]");
  r = runBlock(fx);
  assert.deepEqual(
    r.calls.filter((l) => argv(l).startsWith("owner-check ")).map(argv),
    [
      "owner-check alpha-fleet --action push to dev",
      "owner-check jenkins-dispatch --action push to dev",
    ],
  );
});
test("build_app_owner_checks_only_jenkins_dispatch", (t) => {
  const r = runBlock(fixture(t, "x [build:app]"));
  assert.deepEqual(
    r.calls.filter((l) => argv(l).startsWith("owner-check ")).map(argv),
    ["owner-check jenkins-dispatch --action push to dev"],
  );
});
test("no_build_tag_owner_checks_nothing", (t) => {
  const r = runBlock(fixture(t));
  assert.equal(
    r.calls.some((l) => argv(l).startsWith("owner-check ")),
    false,
  );
});
test("a_feature_branch_push_makes_no_berth_call", (t) => {
  const r = runBlock(fixture(t, "[build:edge]"), { ref: "refs/heads/x" });
  assert.equal(r.status, 0, r.out);
  assert.deepEqual(r.calls, []);
});
test("a_session_that_already_holds_dev_integration_keeps_it", (t) => {
  const r = runBlock(fixture(t), { env: { SESSION_HELD: "1" } });
  assert.equal(r.status, 0, r.out);
  assert.match(r.out, /claimed by this session already/);
  assert.equal(
    r.calls.some((l) => /^(moor|release) /.test(argv(l))),
    false,
  );
});
test("a_hand_held_lease_refused_as_fenced_is_released_and_claimed_afresh", (t) => {
  const r = runBlock(fixture(t), {
    env: { SESSION_HELD: "1", BERTH_MODES: '{"dev-integration":"F"}' },
  });
  assert.equal(r.status, 0, r.out);
  assert.match(r.out, /hand-held dev-integration lease could not be renewed/);
  assert.match(r.out, /self-demoted: no head from did:key:z6MkDeviceA within 570s/);
  assert.match(r.out, /will not hold the lease after the push/);
  assert.doesNotMatch(r.out, /stays held after the push/);
  const v = r.calls
    .map(argv)
    .filter((a) => /^(claim|release) dev-integration/.test(a))
    .map((a) => a.split(" ")[0]);
  assert.deepEqual(v.slice(0, 3), ["claim", "release", "claim"], r.calls.join("\n"));
});
test("a_fenced_hand_held_lease_whose_release_fails_refuses_with_the_reason", (t) => {
  const r = runBlock(fixture(t), {
    env: {
      SESSION_HELD: "1",
      BERTH_MODES: '{"dev-integration":"F","release:dev-integration":"1"}',
    },
  });
  assert.equal(r.status, 1, r.out);
  assert.match(r.out, /REFUSED: this session's dev-integration lease is fenced \(self-demoted: no head from did:key:z6MkDeviceA within 570s\) and releasing it to claim afresh failed/);
  assert.equal(r.calls.filter((l) => argv(l).startsWith("claim ")).length, 1);
});
test("a_hand_held_lease_refused_because_another_device_holds_it_is_refused_as_before", (t) => {
  const r = runBlock(fixture(t), {
    env: { SESSION_HELD: "1", BERTH_MODES: '{"dev-integration":"3"}' },
  });
  assert.equal(r.status, 1, r.out);
  assert.match(r.out, /\[pre-push\] REFUSED: this push needs dev-integration \(reason above\)\. Your gates are recorded; push again when it is free\./);
  assert.equal(r.calls.some((l) => argv(l).startsWith("release ")), false);
});
test("prepush_waits_for_the_override_then_lands", async (t) => {
  const fx = fixture(t),
    r = runBlock(fx, {
      env: {
        BERTH_MODES: '{"dev-integration":"W"}',
        STUB_OVERRIDE_IN_MS: "1500",
      },
    });
  assert.equal(r.status, 0, r.out);
  assert.ok(
    r.out.split("\n").some((l) => MARKERS.overrideWait.re.test(l)),
    r.out,
  );
  assert.ok(r.calls.filter((l) => argv(l).startsWith("claim ")).length >= 2);
  await pollCalls(fx, (c) => !!release(c));
});
test("the_override_wait_is_bounded", async (t) => {
  const fx = fixture(t),
    r = runBlock(fx, {
      env: {
        BERTH_MODES: '{"dev-integration":"W"}',
        STUB_OVERRIDE_IN_MS: "60000",
        BERTH_PREPUSH_OVERRIDE_MAX_S: "2",
      },
    });
  assert.equal(r.status, 1, r.out);
  assert.match(r.out, /one silent sibling/);
  const c = await pollCalls(fx, (x) => !!release(x));
  assert.match(argv(release(c)), /1001\.125$/);
});
test("the_daemon_down_line_is_printed_during_the_override_wait", (t) => {
  const r = runBlock(fixture(t), {
    env: {
      BERTH_MODES: '{"dev-integration":"D"}',
      STUB_OVERRIDE_IN_MS: "1200",
    },
  });
  assert.equal(r.status, 0, r.out);
  assert.ok(
    r.out.split("\n").some((l) => MARKERS.daemonDown.re.test(l)),
    r.out,
  );
});
test("an_interrupt_during_the_override_wait_releases_and_prints_the_resume", async (t) => {
  const fx = fixture(t),
    h = runAsync(fx, {
      env: {
        BERTH_MODES: '{"dev-integration":"W"}',
        STUB_OVERRIDE_IN_MS: "60000",
        BERTH_PREPUSH_OVERRIDE_MAX_S: "30",
      },
    });
  await pollCalls(fx, (c) => c.some((l) => argv(l).startsWith("claim ")));
  h.child.kill("SIGINT");
  const r = await h.done;
  assert.equal(r.status, 130, r.out);
  assert.match(r.out, /interrupted at the landing \(INT\)/);
  await pollCalls(fx, (c) => !!release(c));
});
test("a_deliberate_refusal_prints_no_resume_line", (t) => {
  const r = runBlock(fixture(t), {
    env: { BERTH_MODES: '{"dev-integration":"3"}' },
  });
  assert.equal(r.status, 1, r.out);
  assert.doesNotMatch(r.out, /interrupted at the landing/);
});
test("the_renewer_records_a_new_generation", async (t) => {
  const fx = fixture(t),
    s = spawn("sleep", ["30"]);
  t.after(() => s.kill("SIGKILL"));
  const r = runBlock(fx, {
    env: {
      PHASE_L_PUSH_PID: String(s.pid),
      BERTH_PREPUSH_TTL_S: "3",
      BERTH_MODES: '{"dev-integration":"N"}',
    },
  });
  assert.equal(r.status, 0, r.out);
  await new Promise((x) => setTimeout(x, 3200));
  await stop(s);
  const c = await pollCalls(fx, (x) => !!release(x));
  assert.match(argv(release(c)), /1009\.500$/);
});
// The landing waits (3 s) before it hands over, so the agent renews at least once first.
const slowHandoff = `eval "$(declare -f phase_l_handoff | sed '1s/.*/phase_l_handoff_orig ()/')"
phase_l_handoff() { sleep 3.5; sed 's/^/STATUS: /' "$PHASE_L_DIR/status"; phase_l_handoff_orig; }
echo GATES-RAN; hook_pass`;
const lockEnv = (mode) => ({
  BERTH_PREPUSH_TTL_S: "3",
  BERTH_PREPUSH_RENEW_RETRY_S: "0.05",
  BERTH_MODES: `{"dev-integration":"${mode}"}`,
});
test("a_busy_store_lock_on_renew_is_retried_not_a_refusal", async (t) => {
  const fx = fixture(t),
    s = spawn("sleep", ["30"]);
  t.after(() => s.kill("SIGKILL"));
  const r = runBlock(fx, {
    after: slowHandoff,
    env: { PHASE_L_PUSH_PID: String(s.pid), ...lockEnv("L") },
  });
  assert.equal(r.status, 0, r.out);
  assert.match(r.out, /STATUS: say .*store lock was busy on renew; retrying \(1 of 3\)/);
  assert.doesNotMatch(r.out, /REFUSED|lost renew refused/);
  await stop(s);
});
test("a_store_lock_that_stays_busy_on_renew_still_stops_the_push", (t) => {
  const fx = fixture(t),
    r = runBlock(fx, { after: slowHandoff, env: lockEnv("K") });
  assert.equal(r.status, 1, r.out);
  assert.match(r.out, /retrying \(3 of 3\)/);
  assert.doesNotMatch(r.out, /retrying \(4 of 3\)/);
  assert.match(
    r.out,
    /REFUSED: renewing dev-integration while it waited to land was refused \(above\)\. The push stops\./,
  );
});
test("a_device_without_epr_refuses_before_any_gate", (t) => {
  const fx = fixture(t),
    key = join(fx.dir, "key");
  writeFileSync(key, "x");
  const r = runBlock(fx, {
    env: {
      PATH: `${fx.bin}:/usr/bin:/bin`,
      ELOHIM_DEVICE_KEY_FILE: key,
      EPR_BIN: "",
    },
  });
  assert.equal(r.status, 1, r.out);
  assert.ok(
    r.out.split("\n").some((l) => MARKERS.eprRefused.re.test(l)),
    r.out,
  );
  assert.doesNotMatch(r.out, /GATES-RAN/);
  assert.deepEqual(r.calls, []);
});
test("integration_push_requires_berth", (t) => {
  const fx = fixture(t);
  rmSync(join(fx.bin, "berth"));
  const r = runBlock(fx);
  assert.equal(r.status, 1, r.out);
  assert.doesNotMatch(r.out, /GATES-RAN/);
  assert.match(r.out, /restore executable/);
});
test("a_deletion_push_to_dev_claims_and_settles", async (t) => {
  const fx = fixture(t),
    r = runBlock(fx, { local: ZERO, env: { REMOTE_SHA: "" } });
  assert.equal(r.status, 0, r.out);
  assert.ok(r.calls.some((l) => argv(l).startsWith("claim ")));
  await pollCalls(fx, (c) => !!release(c));
  await pollSettle(fx, /settle none landed dev=/);
});

// ── Which process the landing agent waits on (git push or brit push) ──
// A fake /proc (PHASE_L_PROC) stands in for the hook's ancestry. Its root entry is this test
// runner's own pid, because the hook's $PPID is the node process that spawned `bash -c`. The shapes
// mirror what a probe hook printed on this device (ps -o pid,ppid,comm,args, 2026-11-25): git push
// and brit push are each the hook's nearest pushing ancestor and run the transfer after the hook.
function fakeProc(fx, chain) {
  const root = join(fx.dir, "proc"),
    all = [{ pid: process.pid, comm: "sh", argv: ["sh", ".husky/_/pre-push"] }, ...chain];
  all.forEach((p, i) => {
    const d = join(root, String(p.pid)),
      ppid = all[i + 1]?.pid ?? 1;
    mkdirSync(d, { recursive: true });
    writeFileSync(join(d, "comm"), `${p.comm}\n`);
    writeFileSync(join(d, "cmdline"), `${p.argv.join("\0")}\0`);
    writeProcStat(fx, p.pid, p.comm, ppid, p.start ?? 1000 + i);
    if (p.exe) spawnSync("ln", ["-s", p.exe, join(d, "exe")]);
  });
  return root;
}
function writeProcStat(fx, pid, comm, ppid, start) {
  writeFileSync(
    join(fx.dir, "proc", String(pid), "stat"),
    `${pid} (${comm}) S ${ppid} 0 0 0 -1 0 0 0 0 0 0 0 0 0 20 0 1 0 ${start} 1 1\n`,
  );
}
const exitProc = (fx, pid) =>
  rmSync(join(fx.dir, "proc", String(pid)), { recursive: true, force: true });
const chainEnv = (fx, chain, e = {}) => ({
  PHASE_L_PUSH_PID: "",
  BRIT_GUARD_HELD_BY: "",
  PHASE_L_PROC: fakeProc(fx, chain),
  ...e,
});
const settleLine = (s) => s.split("\n").find((l) => / settle /.test(l));
async function waitsWhileAliveThenSettles(t, fx, r, pid, tool) {
  assert.equal(r.status, 0, r.out);
  await new Promise((x) => setTimeout(x, 1100));
  assert.equal(release(calls(fx)), undefined, "released while the push was alive");
  exitProc(fx, pid);
  const s = await pollSettle(fx, new RegExp(`settle ${HEAD} landed dev=\\S+ tool=${tool} held=\\d+s released=ok`));
  return settleLine(s);
}
const BRIT_EXE =
  "/projects/.claude-config/tools/bin-store/brit/bafyreiee75hqw3d3dcdelaxetz6c7cydw4cm3l4bw4nherbcyeldopikmu/brit";

test("a_git_push_ancestor_is_waited_on_and_recorded_as_git", async (t) => {
  const fx = fixture(t),
    chain = [
      { pid: 90011, comm: "sh", argv: ["sh", "-e", ".husky/pre-push"] },
      { pid: 90012, comm: "git", exe: "/usr/bin/git", argv: ["git", "-c", "core.hooksPath=/w/.husky/_", "push", "origin", "HEAD:dev"] },
      { pid: 90013, comm: "bash", exe: "/usr/bin/bash", argv: ["bash"] },
    ],
    r = runBlock(fx, { env: chainEnv(fx, chain) });
  // Unchanged for git: no new line on the hook's output.
  assert.doesNotMatch(r.out, /the push is|no git or brit push/);
  await waitsWhileAliveThenSettles(t, fx, r, 90012, "git");
});
test("a_brit_push_with_global_options_before_the_verb_is_waited_on", async (t) => {
  const fx = fixture(t),
    chain = [
      { pid: 90021, comm: "sh", argv: ["sh", "-e", ".husky/pre-push"] },
      {
        pid: 90022,
        comm: "brit",
        exe: BRIT_EXE,
        argv: ["brit", "-c", "core.hooksPath=/w/.husky/_", "-C", "/w", "-r", ".", "--threads", "2", "-v", "--config=x=push", "push", "origin", "HEAD:dev"],
      },
      { pid: 90023, comm: "bash", exe: "/usr/bin/bash", argv: ["bash"] },
    ],
    r = runBlock(fx, { env: chainEnv(fx, chain) });
  assert.match(r.out, /Phase L: the push is brit pid 90022; the landing agent waits for it to exit/);
  await waitsWhileAliveThenSettles(t, fx, r, 90022, "brit");
});
test("a_brit_push_reached_through_the_bin_store_launcher_is_waited_on", async (t) => {
  // On this device ~/.local/bin/brit -> bin-store's bin-launch, which execs the real brit: one
  // process, comm brit, exe inside the bin-store. A forking launcher or a wrapper script named brit
  // leaves a parent too; the nearest pushing ancestor is still the one waited on.
  let fx = fixture(t),
    r = runBlock(fx, {
      env: chainEnv(fx, [
        { pid: 90031, comm: "sh", argv: ["sh", "-e", ".husky/pre-push"] },
        { pid: 90032, comm: "brit", exe: BRIT_EXE, argv: ["/home/user/.local/bin/brit", "-c", "a=b", "push", "origin", "HEAD:dev"] },
        { pid: 90033, comm: "brit", exe: "/projects/.claude-config/tools/bin-store/bin/bin-launch", argv: ["brit", "-c", "a=b", "push", "origin", "HEAD:dev"] },
      ]),
    });
  assert.match(r.out, /the push is brit pid 90032;/);
  await waitsWhileAliveThenSettles(t, fx, r, 90032, "brit");
  fx = fixture(t);
  r = runBlock(fx, {
    env: chainEnv(fx, [
      { pid: 90036, comm: "sh", argv: ["sh", "-e", ".husky/pre-push"] },
      { pid: 90037, comm: "brit-0.3.4", exe: "/opt/brit/brit-0.3.4", argv: ["brit-0.3.4", "-c", "a=b", "push", "origin"] },
      { pid: 90038, comm: "brit", exe: "/usr/bin/bash", argv: ["/bin/sh", "/opt/bin/brit", "-c", "a=b", "push", "origin"] },
    ]),
  });
  assert.match(r.out, /the push is brit pid 90038;/);
  await waitsWhileAliveThenSettles(t, fx, r, 90038, "brit");
});
test("brit_guard_held_by_is_reported_only_when_it_names_the_brit_push_already_chosen", async (t) => {
  // brit 0.3.4 exports BRIT_GUARD_HELD_BY="v1 <its pid> <its start time> n:…" to every hook it runs.
  const fx = fixture(t),
    r = runBlock(fx, {
      env: chainEnv(
        fx,
        [
          { pid: 90041, comm: "sh", argv: ["sh", "-e", ".husky/pre-push"] },
          { pid: 90042, comm: "brit", exe: BRIT_EXE, argv: ["brit", "push", "origin"], start: 555 },
        ],
        { BRIT_GUARD_HELD_BY: "v1 90042 555 n:2f:2f" },
      ),
    });
  assert.match(r.out, /the push is brit pid 90042, as BRIT_GUARD_HELD_BY names it;/);
  await waitsWhileAliveThenSettles(t, fx, r, 90042, "brit");
});
test("a_marker_naming_the_hooks_own_shell_is_ignored_and_git_still_found", async (t) => {
  const fx = fixture(t),
    r = runBlock(fx, {
      env: chainEnv(
        fx,
        [
          { pid: 90044, comm: "sh", argv: ["sh", "-e", ".husky/pre-push"], start: 444 },
          { pid: 90045, comm: "git", exe: "/usr/bin/git", argv: ["git", "push", "origin", "HEAD:dev"] },
        ],
        { BRIT_GUARD_HELD_BY: "v1 90044 444 n:2f:2f" },
      ),
    });
  assert.doesNotMatch(r.out, /the push is brit/);
  await waitsWhileAliveThenSettles(t, fx, r, 90045, "git");
});
test("a_marker_naming_a_brit_that_is_not_pushing_is_ignored", (t) => {
  const fx = fixture(t),
    r = runBlock(fx, {
      env: chainEnv(
        fx,
        [
          { pid: 90046, comm: "sh", argv: ["sh", "-e", ".husky/pre-push"] },
          { pid: 90047, comm: "brit", exe: BRIT_EXE, argv: ["brit", "-c", "a=push", "status"], start: 777 },
        ],
        { BRIT_GUARD_HELD_BY: "v1 90047 777 n:2f:2f", PHASE_L_LIMIT_S: "1" },
      ),
    });
  assert.equal(r.status, 0, r.out);
  assert.match(r.out, /no git or brit push found among this hook's ancestors/);
});
test("an_outer_brit_running_a_script_that_runs_git_push_records_git", async (t) => {
  const fx = fixture(t),
    r = runBlock(fx, {
      env: chainEnv(
        fx,
        [
          { pid: 90048, comm: "sh", argv: ["sh", "-e", ".husky/pre-push"] },
          { pid: 90049, comm: "git", exe: "/usr/bin/git", argv: ["git", "push", "origin", "HEAD:dev"] },
          { pid: 90050, comm: "land.sh", exe: "/usr/bin/bash", argv: ["bash", "land.sh"] },
          { pid: 90051, comm: "brit", exe: BRIT_EXE, argv: ["brit", "push", "origin", "HEAD:dev"], start: 888 },
        ],
        { BRIT_GUARD_HELD_BY: "v1 90051 888 n:2f:2f" },
      ),
    });
  assert.doesNotMatch(r.out, /the push is brit/);
  await waitsWhileAliveThenSettles(t, fx, r, 90049, "git");
});
test("an_unknown_pusher_renews_while_it_observes_and_stops_at_the_limit_unconfirmed", async (t) => {
  // brit with a verb other than push, a value that reads "push", and nothing else that pushes.
  const fx = fixture(t),
    chain = [
      { pid: 90052, comm: "sh", argv: ["sh", "-e", ".husky/pre-push"] },
      { pid: 90053, comm: "brit", exe: BRIT_EXE, argv: ["brit", "-c", "push", "rebase", "--exec", "x"] },
      { pid: 90054, comm: "bash", exe: "/usr/bin/bash", argv: ["bash"] },
    ],
    started = Date.now(),
    r = runBlock(fx, {
      env: chainEnv(fx, chain, {
        REMOTE_SHA: BASE,
        BERTH_PREPUSH_TTL_S: "3",
        PHASE_L_LIMIT_S: "4",
        BERTH_PREPUSH_OBSERVE_S: "0.2",
      }),
    });
  assert.equal(r.status, 0, r.out);
  assert.match(r.out, /no git or brit push found among this hook's ancestors/);
  const claimsAtHandoff = calls(fx).filter((l) => argv(l).startsWith("claim ")).length;
  await new Promise((x) => setTimeout(x, 2000));
  assert.equal(release(calls(fx)), undefined, "settled before the limit");
  const s = await pollSettle(fx, / settle /, 6000),
    ended = Date.now();
  assert.match(
    settleLine(s),
    new RegExp(`settle ${HEAD} unconfirmed dev=111111111111 tool=unknown wait=limit reason=no-pusher-identified held=\\d+s released=ok`),
  );
  assert.doesNotMatch(s, /did-not-land/);
  assert.ok(
    calls(fx).filter((l) => argv(l).startsWith("claim ")).length > claimsAtHandoff,
    "the lease was renewed while observing",
  );
  assert.ok(ended - started < 4000 + 4000, `settled ${ended - started} ms after the push began`);
});
test("an_unknown_pusher_settles_landed_once_the_remote_reads_the_landing_sha", async (t) => {
  const fx = fixture(t),
    r = runBlock(fx, {
      env: chainEnv(fx, [{ pid: 90061, comm: "bash", argv: ["bash"] }], {
        REMOTE_SHA: BASE,
        BERTH_PREPUSH_TTL_S: "30",
        BERTH_PREPUSH_OBSERVE_S: "0.2",
      }),
    });
  assert.equal(r.status, 0, r.out);
  await new Promise((x) => setTimeout(x, 600));
  assert.equal(release(calls(fx)), undefined);
  writeFileSync(join(fx.bin, "remote-sha"), HEAD);
  const s = await pollSettle(fx, / settle /, 4000);
  assert.match(settleLine(s), new RegExp(`settle ${HEAD} landed dev=222222222222 tool=unknown wait=observed held=\\d+s released=ok`));
});
test("a_reused_push_pid_is_not_waited_on", async (t) => {
  const fx = fixture(t),
    r = runBlock(fx, {
      env: chainEnv(fx, [
        { pid: 90071, comm: "sh", argv: ["sh", "-e", ".husky/pre-push"] },
        { pid: 90072, comm: "brit", exe: BRIT_EXE, argv: ["brit", "push", "origin", "HEAD:dev"], start: 7000 },
      ]),
    });
  assert.equal(r.status, 0, r.out);
  assert.match(r.out, /the push is brit pid 90072;/);
  await new Promise((x) => setTimeout(x, 700));
  assert.equal(release(calls(fx)), undefined);
  // The brit exits and an unrelated process takes its pid: same pid, another start time.
  writeProcStat(fx, 90072, "sleep", 1, 7999);
  const s = await pollSettle(fx, / settle /, 3000);
  assert.match(settleLine(s), new RegExp(`settle ${HEAD} landed dev=\\S+ tool=brit held=`));
  assert.ok(existsSync(join(fx.dir, "proc/90072/stat")));
});

test("a_remote_that_cannot_be_read_is_unconfirmed_never_did_not_land", async (t) => {
  const fx = fixture(t),
    r = runBlock(fx, {
      env: {
        LS_REMOTE_RC: "128",
        PHASE_L_READ_TRIES: "3",
        PHASE_L_READ_RETRY_S: "0.1",
      },
    });
  assert.equal(r.status, 0, r.out);
  const s = await pollSettle(fx, / settle /, 4000);
  assert.match(settleLine(s), new RegExp(`settle ${HEAD} unconfirmed dev= tool=git reason=remote-unreadable held=`));
  assert.equal(readFileSync(join(fx.bin, "ls-remote.log"), "utf8").trim().split("\n").length, 3);
});
test("a_deletion_whose_remote_read_fails_is_not_landed", async (t) => {
  const fx = fixture(t),
    r = runBlock(fx, { local: ZERO, env: { REMOTE_SHA: "", LS_REMOTE_RC: "2", PHASE_L_READ_RETRY_S: "0.1" } });
  assert.equal(r.status, 0, r.out);
  const s = await pollSettle(fx, / settle /, 4000);
  assert.match(settleLine(s), /settle none unconfirmed dev= tool=git reason=remote-unreadable/);
});
test("a_remote_read_that_ignores_term_is_killed_within_its_bound", async (t) => {
  const fx = fixture(t),
    started = Date.now(),
    r = runBlock(fx, {
      env: { LS_REMOTE_HANG: "1", PHASE_L_READ_S: "1", PHASE_L_KILL_S: "1", PHASE_L_READ_TRIES: "1" },
    });
  assert.equal(r.status, 0, r.out);
  const s = await pollSettle(fx, / settle /, 8000);
  assert.match(settleLine(s), /unconfirmed dev= tool=git reason=remote-unreadable/);
  assert.ok(Date.now() - started < 7000, `settled ${Date.now() - started} ms after the push began`);
});
test("a_known_pusher_alive_at_the_limit_is_released_unconfirmed", async (t) => {
  const fx = fixture(t),
    r = runBlock(fx, {
      env: chainEnv(
        fx,
        [
          { pid: 90081, comm: "sh", argv: ["sh", "-e", ".husky/pre-push"] },
          { pid: 90082, comm: "brit", exe: BRIT_EXE, argv: ["brit", "push", "origin", "HEAD:dev"] },
        ],
        { REMOTE_SHA: BASE, PHASE_L_LIMIT_S: "2" },
      ),
    });
  assert.equal(r.status, 0, r.out);
  const s = await pollSettle(fx, / settle /, 6000);
  assert.match(
    settleLine(s),
    new RegExp(`settle ${HEAD} unconfirmed dev=111111111111 tool=brit wait=limit reason=push-still-running held=`),
  );
  assert.ok(existsSync(join(fx.dir, "proc/90082/stat")));
});

const agentPid = (fx) =>
  Number(argv(calls(fx).find((l) => argv(l).startsWith("moor --session pre-push-"))).match(/ --pid (\d+) /)[1]);
const alive = (pid) => {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
};
const releases = (fx) => calls(fx).filter((l) => argv(l).startsWith("release dev-integration ")).length;
const liveBrit = (fx, pid) => [
  { pid: pid - 1, comm: "sh", argv: ["sh", "-e", ".husky/pre-push"] },
  { pid, comm: "brit", exe: BRIT_EXE, argv: ["brit", "push", "origin", "HEAD:dev"] },
];
test("a_renewal_that_hangs_forever_still_ends_the_agent_by_its_deadline_releasing_once", async (t) => {
  const fx = fixture(t),
    r = runBlock(fx, {
      env: chainEnv(fx, liveBrit(fx, 90092), {
        REMOTE_SHA: BASE,
        BERTH_PREPUSH_TTL_S: "3",
        PHASE_L_LIMIT_S: "3",
        PHASE_L_SETTLE_S: "3",
        PHASE_L_KILL_S: "1",
        PHASE_L_GRACE_S: "5",
      }),
    }),
    handed = Date.now();
  assert.equal(r.status, 0, r.out);
  writeFileSync(join(fx.bin, "hang"), "");
  // Deadline: the hand-over + 3 + 3 s; the hung renewal is cut to end by it, then the release runs.
  const s = await pollSettle(fx, / settle /, 20000),
    pid = agentPid(fx);
  assert.match(settleLine(s), new RegExp(`settle ${HEAD} unconfirmed dev= tool=brit reason=`));
  assert.ok(Date.now() - handed < 6000 + 8000, `settled ${Date.now() - handed} ms after the hand-over`);
  await pollCalls(fx, () => !alive(pid), 10000);
  assert.equal(releases(fx), 1, calls(fx).join("\n"));
});
test("a_refused_renewal_records_renewal_refused_and_releases_nothing_it_does_not_hold", async (t) => {
  const fx = fixture(t),
    r = runBlock(fx, {
      env: chainEnv(fx, liveBrit(fx, 90094), { REMOTE_SHA: BASE, BERTH_PREPUSH_TTL_S: "3" }),
    });
  assert.equal(r.status, 0, r.out);
  writeFileSync(join(fx.bin, "refuse"), "");
  const s = await pollSettle(fx, / settle /, 6000);
  assert.match(
    settleLine(s),
    new RegExp(`settle ${HEAD} unconfirmed dev=111111111111 tool=brit reason=renewal-refused held=\\d+s released=not-held`),
  );
  assert.equal(releases(fx), 0, calls(fx).join("\n"));
});
test("a_terminated_agent_releases_once_and_records_why", async (t) => {
  const fx = fixture(t),
    r = runBlock(fx, { env: chainEnv(fx, liveBrit(fx, 90096), { REMOTE_SHA: BASE }) });
  assert.equal(r.status, 0, r.out);
  const pid = agentPid(fx);
  await new Promise((x) => setTimeout(x, 600));
  process.kill(pid, "SIGTERM");
  const s = await pollSettle(fx, / settle /, 4000);
  assert.match(settleLine(s), new RegExp(`settle ${HEAD} unconfirmed dev= tool=brit reason=terminated:TERM held=\\d+s released=ok`));
  await pollCalls(fx, () => !alive(pid), 4000);
  assert.equal(releases(fx), 1, calls(fx).join("\n"));
});
