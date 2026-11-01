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
 ls-remote) [ -z "\${REMOTE_SHA+x}" ] || printf '%s\\trefs/heads/dev\\n' "$REMOTE_SHA";;
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
 release) echo "$2: released"; exit 0;;
 claim)
  n=$(grep -c "|claim $2 " "$here/calls.log" 2>/dev/null || true); m=$(mode "$2"); token=1001.125
  [ "$m" != N ] || [ "$n" -lt 2 ] || token=1009.500
  case "$m" in
   3) echo "$2: REFUSED — held by did:key:z6MkDeviceA/claude-1" >&2; exit 3;;
   U) echo "$2: renewed here, but the renew record did not reach this device's channel store" >&2; exit 1;;
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
    /settle \S+ (landed|did-not-land) dev=\S* held=\d+s released=ok/,
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
