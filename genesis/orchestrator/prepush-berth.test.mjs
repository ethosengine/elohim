// The pre-push hook's integration claims (spec 2026-10-23-workspaces-cooperate-as-devices-design.md
// §3.5): a push to dev or main claims dev-integration through berth for the life of the push, a
// pushed `[build:edge]` also alpha-fleet, any `[build:*]` also jenkins-dispatch, and every claim the
// push took is released on every exit. Like serving-receipt.test.mjs, this runs the ACTUAL hook's
// block (sliced between its markers), here against a stub `berth` that logs each call and answers
// from a table, inside a scratch repository whose commits carry the build tags.
import { test } from "node:test";
import assert from "node:assert/strict";
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("../../", import.meta.url));
const ZERO = "0000000000000000000000000000000000000000";
const HELD_LINE =
  "dev-integration: REFUSED — held by did:key:z6MkDeviceA/claude-1 since 2026-10-23T15:28:09Z";

function hookBlock() {
  const hook = readFileSync(join(ROOT, ".husky/pre-push.bash"), "utf8");
  const start = hook.indexOf("# ── Berth: integration claims");
  const end = hook.indexOf("# ── Berth: claims taken", start);
  assert.ok(start >= 0 && end > start, "exercise the actual hook's berth block");
  return hook.slice(start, end);
}

// A stub berth: logs `<args>` per call; a claim answers from `answers` (`<resource> <code>...`, the
// nth claim of a resource taking the nth code, the last repeating):
// 0 claimed, R renewed (exit 0), 3 held by device A, 4 no session, I interrupted (Ctrl-C reaches the
// hook while that claim is in flight); anything unlisted is claimed.
const STUB_BERTH = `#!/usr/bin/env bash
here="$(cd "$(dirname "$0")" && pwd)"
echo "$*" >> "$here/calls.log"
if [ "$1" = claim ]; then
  n=$(grep -c "^claim $2 " "$here/calls.log")
  code=$(awk -v r="$2" -v n="$n" '$1 == r { print $((n + 1 <= NF) ? n + 1 : NF) }' "$here/answers" 2>/dev/null | head -1)
  case "\${code:-0}" in
    0) echo "$2: claimed by codex-1 (session from env)"; exit 0 ;;
    R) echo "$2: renewed by codex-1 (session from env)"; exit 0 ;;
    3) echo "$2: REFUSED — held by did:key:z6MkDeviceA/claude-1 since 2026-10-23T15:28:09Z" >&2; exit 3 ;;
    4) echo "berth: no session (pass --session, or set BERTH_SESSION / CLAUDE_SESSION_ID)" >&2; exit 4 ;;
    I) kill -INT "$HOOK_PID"; exit 130 ;;
    U) echo "$2: renewed here, but the renew record did not reach this device's channel store (/x): y" >&2; exit 1 ;;
    E) echo "Traceback (most recent call last): KeyError" >&2; exit 1 ;;
  esac
fi
[ "$1" = release ] && echo "$2: released by codex-1"
exit 0
`;

const cleanEnv = Object.fromEntries(
  Object.entries(process.env).filter(
    ([key]) => !key.startsWith("GIT_") || key.startsWith("GIT_CONFIG_"),
  ),
);

/** A scratch checkout with a stub berth, a base commit and one pushed commit carrying `message`. */
function fixture(t, message, answers = {}) {
  const dir = mkdtempSync(join(tmpdir(), "prepush-berth-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const git = (...args) => {
    const r = spawnSync("git", ["-C", dir, ...args], { encoding: "utf8", env: cleanEnv });
    assert.equal(r.status, 0, r.stderr);
    return r.stdout.trim();
  };
  git("init", "--quiet");
  const id = ["-c", "user.name=Fixture", "-c", "user.email=fixture@example.test"];
  git(...id, "commit", "--quiet", "--allow-empty", "-m", "base");
  const base = git("rev-parse", "HEAD");
  git(...id, "commit", "--quiet", "--allow-empty", "-m", message);
  const head = git("rev-parse", "HEAD");
  const bin = join(dir, "genesis/agentic/bin");
  mkdirSync(bin, { recursive: true });
  writeFileSync(join(bin, "berth"), STUB_BERTH);
  chmodSync(join(bin, "berth"), 0o755);
  writeFileSync(
    join(bin, "answers"),
    Object.entries(answers)
      .map(([resource, codes]) => `${resource} ${[codes].flat().join(" ")}\n`)
      .join(""),
  );
  return { dir, base, head, bin };
}

/** Run the hook's block for one pushed ref, then `after` (the gates that follow it). */
function runBlock(fx, { ref = "refs/heads/dev", after = "echo GATES-RAN", env = {} } = {}) {
  const preamble = [
    // The hook sets pipefail before this block; the block runs under it.
    "set -o pipefail",
    "export HOOK_PID=$$",
    `ZERO_SHA=${ZERO}`,
    `HOOK_WORK_TREE='${fx.dir}'`,
    `PUSH_LINES=('refs/heads/feature ${fx.head} ${ref} ${fx.base}')`,
    "",
  ].join("\n");
  const result = spawnSync("bash", ["-c", `${preamble}${hookBlock()}${after}`], {
    cwd: fx.dir,
    encoding: "utf8",
    env: { ...cleanEnv, BERTH_OFFLINE_OK: "", ...env },
  });
  let calls = [];
  try {
    calls = readFileSync(join(fx.bin, "calls.log"), "utf8").trim().split("\n").filter(Boolean);
  } catch {
    calls = [];
  }
  return { ...result, out: `${result.stdout}${result.stderr}`, calls };
}

const CLAIM = (resource) => `claim ${resource} --class verify --ttl 1800`;

test("prepush_refuses_while_peer_holds_dev_integration", (t) => {
  const fx = fixture(t, "feat: land it", { "dev-integration": 3 });
  const r = runBlock(fx);
  assert.equal(r.status, 1, r.out);
  assert.ok(r.out.split("\n").includes(HELD_LINE), `berth's reason line verbatim:\n${r.out}`);
  assert.ok(!r.out.includes("GATES-RAN"), "the push stops before any check runs");
  assert.deepEqual(
    r.calls.map((c) => c.split(" --note")[0]),
    [CLAIM("dev-integration")],
    "nothing was taken, so nothing is released",
  );
});

test("prepush_releases_on_gate_failure", (t) => {
  const fx = fixture(t, "feat: a build [build:app]");
  const failed = runBlock(fx, { after: "echo GATES-RAN; exit 1" });
  assert.equal(failed.status, 1, failed.out);
  assert.ok(failed.out.includes("GATES-RAN"));
  assert.deepEqual(failed.calls.map((c) => c.split(" --note")[0]), [
    CLAIM("dev-integration"),
    CLAIM("jenkins-dispatch"),
    "release dev-integration",
    "release jenkins-dispatch",
  ]);

  // Ctrl-C mid-gate: the same release, and the push does not go on.
  const fx2 = fixture(t, "feat: a build [build:app]");
  const interrupted = runBlock(fx2, { after: "kill -INT $$; sleep 5; echo NOT-REACHED" });
  assert.notEqual(interrupted.status, 0, interrupted.out);
  assert.ok(!interrupted.out.includes("NOT-REACHED"));
  assert.deepEqual(interrupted.calls.slice(-2), [
    "release dev-integration",
    "release jenkins-dispatch",
  ]);

  // Ctrl-C while a claim is in flight: the lease it may have written is given back too.
  const midClaim = runBlock(fixture(t, "feat: a build [build:app]", { "jenkins-dispatch": "I" }));
  assert.notEqual(midClaim.status, 0, midClaim.out);
  assert.ok(!midClaim.out.includes("GATES-RAN"));
  assert.deepEqual(midClaim.calls.slice(-2).sort(), [
    "release dev-integration",
    "release jenkins-dispatch",
  ]);

  // A passing push releases too: the hook holds its claims only while the push runs.
  const fx3 = fixture(t, "feat: land it");
  const passed = runBlock(fx3);
  assert.equal(passed.status, 0, passed.out);
  assert.deepEqual(passed.calls.slice(-1), ["release dev-integration"]);
});

test("build_edge_requires_alpha_fleet", (t) => {
  const fx = fixture(t, "deploy the fix [build:edge]");
  const r = runBlock(fx);
  assert.equal(r.status, 0, r.out);
  const claims = r.calls.filter((c) => c.startsWith("claim")).map((c) => c.split(" --note")[0]);
  assert.deepEqual(claims, [
    CLAIM("dev-integration"),
    CLAIM("alpha-fleet"),
    CLAIM("jenkins-dispatch"),
  ]);

  // Another device watching the deploy holds alpha-fleet: refused, and dev-integration given back.
  const held = fixture(t, "deploy the fix [build:edge]", { "alpha-fleet": 3 });
  const refused = runBlock(held);
  assert.equal(refused.status, 1, refused.out);
  assert.ok(!refused.out.includes("GATES-RAN"));
  assert.ok(refused.out.includes("alpha-fleet: REFUSED — held by did:key:z6MkDeviceA/claude-1"));
  assert.deepEqual(refused.calls.map((c) => c.split(" --note")[0]), [
    CLAIM("dev-integration"),
    CLAIM("alpha-fleet"),
    "release dev-integration",
  ]);

  // A build tag that names the edge among others, and [build:all], deploy too.
  for (const message of ["[build:app,edge] both", "everything [build:all]"]) {
    const fxm = fixture(t, message);
    const m = runBlock(fxm);
    assert.ok(
      m.calls.some((c) => c.startsWith(CLAIM("alpha-fleet"))),
      `${message}: ${m.calls}`,
    );
  }

  // No build tag: dev-integration alone. A feature-branch push: no claim at all.
  const plain = runBlock(fixture(t, "feat: no tag"));
  assert.deepEqual(
    plain.calls.filter((c) => c.startsWith("claim")).map((c) => c.split(" --note")[0]),
    [CLAIM("dev-integration")],
  );
  const feature = runBlock(fixture(t, "wip [build:edge]"), { ref: "refs/heads/feat/x" });
  assert.equal(feature.status, 0, feature.out);
  assert.deepEqual(feature.calls, []);
});

test("prepush_offline_ok_no_session_and_a_renewed_watch", (t) => {
  // BERTH_OFFLINE_OK=1 makes each claim an explicit --offline-ok; the hook never adds it itself.
  const fx = fixture(t, "deploy [build:edge]");
  const r = runBlock(fx, { env: { BERTH_OFFLINE_OK: "1" } });
  assert.equal(r.status, 0, r.out);
  for (const c of r.calls.filter((call) => call.startsWith("claim"))) {
    assert.ok(c.includes("--offline-ok"), c);
  }
  assert.ok(runBlock(fixture(t, "x")).calls.every((c) => !c.includes("--offline-ok")));

  // Exit 4: no session resolvable — the push proceeds unleased and says so.
  const none = runBlock(fixture(t, "feat: land it", { "dev-integration": 4 }));
  assert.equal(none.status, 0, none.out);
  assert.ok(none.out.includes("GATES-RAN"));
  assert.match(none.out, /UNLEASED/);
  assert.ok(!none.calls.some((c) => c.startsWith("release")));

  // A session already watching a deploy holds alpha-fleet: the push renews it, and does not give
  // back at its end what it did not take.
  const watch = runBlock(fixture(t, "deploy [build:edge]", { "alpha-fleet": "R" }));
  assert.equal(watch.status, 0, watch.out);
  assert.deepEqual(watch.calls.filter((c) => c.startsWith("release")), [
    "release dev-integration",
    "release jenkins-dispatch",
  ]);
});

test("a_large_landing_still_claims_by_its_build_tags", (t) => {
  // Over the 64 KiB pipe buffer, with the tag on the first line: a `grep -q` that stops early
  // must never read as "no tag" (under pipefail its writer's SIGPIPE once did).
  const body = `${"x".repeat(99)}\n`.repeat(800);
  const fx = fixture(t, `deploy the fix [build:edge]\n\n${body}`);
  for (let run = 0; run < 10; run += 1) {
    const r = runBlock(fx);
    assert.equal(r.status, 0, r.out);
    const claims = r.calls.filter((c) => c.startsWith("claim")).map((c) => c.split(" --note")[0]);
    assert.deepEqual(
      claims.slice(-3),
      [CLAIM("dev-integration"), CLAIM("alpha-fleet"), CLAIM("jenkins-dispatch")],
      `run ${run}`,
    );
  }
});

test("prepush_exit_1_is_a_renew_only_on_berths_own_line", (t) => {
  // berth's "renewed here, but …": the session's own lease is still held, so the push goes on.
  const renewed = runBlock(fixture(t, "deploy [build:edge]", { "alpha-fleet": "U" }));
  assert.equal(renewed.status, 0, renewed.out);
  assert.ok(renewed.out.includes("GATES-RAN"));
  // Any other exit 1 (a crash) refuses, and gives back what the push had taken.
  const crashed = runBlock(fixture(t, "deploy [build:edge]", { "alpha-fleet": "E" }));
  assert.equal(crashed.status, 1, crashed.out);
  assert.ok(!crashed.out.includes("GATES-RAN"));
  assert.match(crashed.out, /REFUSED: berth claim alpha-fleet failed/);
  assert.deepEqual(crashed.calls.filter((c) => c.startsWith("release")), [
    "release dev-integration",
    "release alpha-fleet",
  ]);
});

// The hook's lease outlives the gates it protects: while they run, every claim it holds is renewed
// every ttl/3 (here a 3 s ttl, so once a second), and the renewing stops on every exit before the
// release, so nothing renews a lease after the push gave it back.
const RENEW = (resource) => `claim ${resource} --class verify --ttl 3`;
const SHORT_TTL = { BERTH_PREPUSH_TTL_S: "3" };

function settledCalls(fx) {
  // Long enough for a renewer that outlived the hook to have renewed again.
  spawnSync("sleep", ["1.5"]);
  return readFileSync(join(fx.bin, "calls.log"), "utf8").trim().split("\n");
}

test("prepush_renews_its_claims_while_the_gates_run", (t) => {
  const fx = fixture(t, "a build [build:app]");
  const r = runBlock(fx, { after: "sleep 3.5; echo GATES-RAN", env: SHORT_TTL });
  assert.equal(r.status, 0, r.out);
  assert.ok(r.out.includes("GATES-RAN"));
  const calls = r.calls.map((c) => c.split(" --note")[0]);
  assert.deepEqual(calls.slice(0, 2), [RENEW("dev-integration"), RENEW("jenkins-dispatch")]);
  const renews = calls.slice(2, -2);
  assert.ok(renews.length >= 4, `renewed while the gates ran: ${calls}`);
  assert.ok(
    renews.every((c) => c === RENEW("dev-integration") || c === RENEW("jenkins-dispatch")),
    `${calls}`,
  );
  assert.deepEqual(calls.slice(-2), ["release dev-integration", "release jenkins-dispatch"]);
  assert.deepEqual(settledCalls(fx), r.calls, "nothing renews after the push ends");

  // A gate that fails, and an interrupt mid-gate, stop the renewing the same way.
  const interrupt = ["kill", "-INT", "$$"].join(" ");
  for (const after of ["sleep 1.5; exit 1", `sleep 1.5; ${interrupt}; sleep 5`]) {
    const fx2 = fixture(t, "feat: land it");
    const ended = runBlock(fx2, { after, env: SHORT_TTL });
    assert.notEqual(ended.status, 0, ended.out);
    assert.deepEqual(ended.calls.slice(-1), ["release dev-integration"]);
    assert.deepEqual(settledCalls(fx2), ended.calls, `${after}: nothing renews after the push ends`);
  }
});

test("prepush_renew_that_does_not_cross_keeps_going", (t) => {
  const fx = fixture(t, "feat: land it", { "dev-integration": ["0", "U"] });
  const r = runBlock(fx, { after: "sleep 2.5; echo GATES-RAN", env: SHORT_TTL });
  assert.equal(r.status, 0, r.out);
  assert.ok(r.out.includes("GATES-RAN"));
  assert.match(r.out, /dev-integration: renewed here, but the renew record did not reach/);
  assert.ok(r.calls.filter((c) => c.startsWith("claim")).length >= 3, `${r.calls}`);
});

test("prepush_renew_refused_outright_stops_the_push", (t) => {
  const fx = fixture(t, "feat: land it", { "dev-integration": ["0", "3"] });
  const r = runBlock(fx, { after: "sleep 1.5; sleep 1.5; echo PUSHED", env: SHORT_TTL });
  assert.notEqual(r.status, 0, r.out);
  assert.ok(!r.out.includes("PUSHED"), r.out);
  assert.ok(r.out.split("\n").includes(HELD_LINE), `berth's reason line verbatim:\n${r.out}`);
  assert.match(r.out, /REFUSED: renewing dev-integration while the gates ran was refused/);
  assert.deepEqual(r.calls.slice(-1), ["release dev-integration"]);
  assert.deepEqual(settledCalls(fx), r.calls, "nothing renews after the push ends");
});
