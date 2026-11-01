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
import { basename, join } from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { MARKERS } from "./gate-landing-markers.mjs";

const ROOT = fileURLToPath(new URL("../../", import.meta.url));
const HOOK = readFileSync(join(ROOT, ".husky/pre-push.bash"), "utf8");
const cleanEnv = Object.fromEntries(
  Object.entries(process.env).filter(
    ([k]) => !k.startsWith("GIT_") || k.startsWith("GIT_CONFIG_"),
  ),
);
function slice(a, b) {
  const x = HOOK.indexOf(a),
    y = HOOK.indexOf(b, x);
  assert.ok(x >= 0 && y > x);
  return HOOK.slice(x, y);
}
const BERTH = slice(
  "# ── Berth: integration claims",
  "# ── Berth: claims taken",
);
const PHASE = slice(
  "# ── Phase L: land in the CAS window",
  "# ── Phase L: end",
);
const STUB = `#!/usr/bin/env bash
root=$(git rev-parse --show-toplevel)
printf '%s|%s|sid=%s|owner=%s|ppid=%s\\n' "$(date +%s.%N)" "$*" "\${ELOHIM_SESSION_ID-}" "\${ELOHIM_SESSION_OWNER_PID-}" "$PPID" >> "$root/calls.log"
case "$1" in
 session-show) echo '{"mooring":{"session":"claude-9"},"leases":{}}';;
 claim) echo "dev-integration: claimed by $ELOHIM_SESSION_ID claimed_at=1001.125";;
 owner-check|moor|unmoor) exit 0;; release) echo released;;
esac
`;
const RUNNER = `import {appendFileSync} from "node:fs";import {join} from "node:path";
const root=process.cwd();const rec=process.env.GATE_EVIDENCE_RECORD??"unset";
appendFileSync(join(root,"events.log"),Date.now()+" gate-runner "+process.argv.slice(2).join(" ")+" GATE_COMMIT="+(process.env.GATE_COMMIT??"")+" GATE_EVIDENCE="+(process.env.GATE_EVIDENCE??"")+" RECORD="+rec+"\\n");
if(process.env.REUSE==="1")console.log("[gate] "+(process.argv.at(-1)??"")+" reused: green");
`;
function cmd(c, args, o = {}) {
  const r = spawnSync(c, args, {
    cwd: o.cwd,
    env: { ...cleanEnv, ...o.env },
    encoding: "utf8",
    timeout: o.timeout ?? 10000,
  });
  assert.equal(r.error, undefined, `${c}: ${r.stderr}`);
  if (!o.allow)
    assert.equal(r.status, 0, `${c} ${args.join(" ")}\n${r.stdout}${r.stderr}`);
  return r;
}
function git(cwd, ...args) {
  return cmd("git", args, { cwd });
}
function sha(cwd, ref = "HEAD") {
  return git(cwd, "rev-parse", ref).stdout.trim();
}
function commit(repo, file, text, msg) {
  writeFileSync(join(repo, file), text);
  git(repo, "add", file);
  git(repo, "commit", "-m", msg);
  return sha(repo);
}
function fixture(t) {
  const dir = mkdtempSync(join(tmpdir(), "prepush-landing-")),
    origin = join(dir, "origin.git"),
    repo = join(dir, "repo"),
    hooks = join(repo, "hooks");
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  git(dir, "init", "--bare", origin);
  git(dir, "init", repo);
  git(repo, "config", "user.email", "fixture@example.test");
  git(repo, "config", "user.name", "Fixture");
  git(repo, "checkout", "-b", "dev");
  commit(repo, "base.txt", "base\n", "base");
  git(repo, "remote", "add", "origin", origin);
  git(repo, "push", "-u", "origin", "dev");
  const base = sha(repo);
  commit(repo, "candidate.txt", "candidate\n", "candidate");
  mkdirSync(join(repo, "genesis/agentic/bin"), { recursive: true });
  writeFileSync(join(repo, "genesis/agentic/bin/berth"), STUB);
  chmodSync(join(repo, "genesis/agentic/bin/berth"), 0o755);
  writeFileSync(
    join(repo, "genesis/agentic/pool-policy.json"),
    JSON.stringify({
      berth: {
        channel: { max_skew_s: 0, fence_margin_s: 0, confirm_wait_s: 0 },
      },
    }),
  );
  mkdirSync(join(repo, "genesis/orchestrator"), { recursive: true });
  writeFileSync(join(repo, "genesis/orchestrator/gate-runner.mjs"), RUNNER);
  mkdirSync(hooks);
  writeFileSync(
    join(hooks, "pre-push"),
    `#!/usr/bin/env bash\nset -o pipefail\nexport HOOK_WORK_TREE=$(git rev-parse --show-toplevel)\nZERO_SHA=${"0".repeat(40)}\nPUSH_LINES=(); while read -r l; do PUSH_LINES+=("$l"); done\n${BERTH}${PHASE}echo GATES-RAN\nexport GATE_EVIDENCE=\${GATE_EVIDENCE:-reuse}\nhook_pass\n`,
  );
  chmodSync(join(hooks, "pre-push"), 0o755);
  git(repo, "config", "core.hooksPath", hooks);
  return { dir, origin, repo, hooks, base };
}
function push(fx, args = ["origin", "HEAD:dev"], env = {}) {
  const started = Date.now();
  const r = cmd("git", ["push", ...args], {
    cwd: fx.repo,
    env: { ELOHIM_DEVICE_KEY_FILE: join(fx.dir, "none"), EPR_BIN: "", ...env },
    allow: true,
    timeout: 20000,
  });
  return { ...r, out: r.stdout + r.stderr, started, ended: Date.now() };
}
function calls(fx) {
  const p = join(fx.repo, "calls.log");
  return existsSync(p)
    ? readFileSync(p, "utf8").trim().split("\n").filter(Boolean)
    : [];
}
const av = (l) => l.split("|")[1],
  ts = (l) => Number(l.split("|")[0]);
async function poll(f, ms = 5000) {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    const v = f();
    if (v) return v;
    await new Promise((r) => setTimeout(r, 25));
  }
  assert.fail("poll timeout");
}
function clear(fx) {
  writeFileSync(join(fx.repo, "calls.log"), "");
  writeFileSync(join(fx.repo, "events.log"), "");
}
function other(fx, name = "other") {
  const p = join(fx.dir, name);
  git(fx.dir, "clone", fx.origin, p);
  git(p, "config", "user.email", "other@example.test");
  git(p, "config", "user.name", "Other");
  git(p, "checkout", "dev");
  return p;
}
function advance(fx, file = "other.txt", text = "other\n") {
  const p = other(fx, `other-${Math.random()}`);
  const s = commit(p, file, text, "other");
  git(p, "push", "origin", "dev");
  return s;
}

test("phase_l_ff_lands_with_zero_gate_calls_inside_the_window", async (t) => {
  const fx = fixture(t),
    r = push(fx);
  assert.equal(r.status, 0, r.out);
  assert.equal(
    existsSync(join(fx.repo, "events.log"))
      ? readFileSync(join(fx.repo, "events.log"), "utf8")
      : "",
    "",
  );
  await poll(() => calls(fx).find((l) => av(l).startsWith("release ")));
});
test("settle_arm_releases_after_the_real_git_push_exits", async (t) => {
  const fx = fixture(t),
    r = push(fx);
  assert.equal(r.status, 0, r.out);
  const rel = await poll(() =>
    calls(fx).find((l) => av(l).startsWith("release ")),
  );
  assert.ok(
    ts(rel) * 1000 >= r.ended,
    `release ${ts(rel) * 1000} >= push exit ${r.ended}`,
  );
  const line = await poll(() => {
    const p = join(fx.repo, ".git/landing/settle.log");
    return (
      existsSync(p) &&
      readFileSync(p, "utf8").match(/settle .* landed dev=/)?.[0]
    );
  });
  assert.match(line, /landed/);
});
test("settle_records_did_not_land_when_origin_rejects", async (t) => {
  const fx = fixture(t),
    p = join(fx.origin, "hooks/pre-receive");
  writeFileSync(p, "#!/bin/sh\nexit 1\n");
  chmodSync(p, 0o755);
  const r = push(fx);
  assert.notEqual(r.status, 0, r.out);
  const line = await poll(() => {
    const q = join(fx.repo, ".git/landing/settle.log");
    return (
      existsSync(q) &&
      readFileSync(q, "utf8").match(/settle .* did-not-land dev=/)?.[0]
    );
  });
  assert.match(line, /did-not-land/);
});
test("moved_base_rebases_in_place_regates_and_refuses", async (t) => {
  const fx = fixture(t),
    newTip = advance(fx),
    r = push(fx, undefined, { PROJECTS: "orchestrator", REUSE: "1" });
  assert.equal(r.status, 1, r.out);
  assert.ok(
    r.out.split("\n").some((l) => MARKERS.moved.re.test(l)),
    r.out,
  );
  const head = sha(fx.repo),
    parent = sha(fx.repo, "HEAD^");
  assert.equal(parent, newTip);
  const ev = readFileSync(join(fx.repo, "events.log"), "utf8").trim();
  assert.match(
    ev,
    new RegExp(
      `gate-runner --target orchestrator GATE_COMMIT=${head} GATE_EVIDENCE=reuse RECORD=unset`,
    ),
  );
  const rel = calls(fx).find((l) => av(l).startsWith("release "));
  assert.ok(
    ts(rel) * 1000 <= Number(ev.split(" ")[0]),
    "release precedes regate",
  );
  clear(fx);
  const again = push(fx);
  assert.equal(again.status, 0, again.out);
  assert.equal(readFileSync(join(fx.repo, "events.log"), "utf8"), "");
  await poll(() => calls(fx).find((l) => av(l).startsWith("release ")));
});
test("moved_base_conflict_aborts_and_refuses", (t) => {
  const fx = fixture(t),
    before = sha(fx.repo);
  writeFileSync(join(fx.repo, "base.txt"), "candidate conflict\n");
  git(fx.repo, "add", "base.txt");
  git(fx.repo, "commit", "--amend", "--no-edit");
  const candidate = sha(fx.repo);
  advance(fx, "base.txt", "other conflict\n");
  const r = push(fx, undefined, { PROJECTS: "" });
  assert.equal(r.status, 1, r.out);
  assert.equal(sha(fx.repo), candidate);
  assert.match(r.out, /base\.txt/);
  assert.notEqual(candidate, before);
});
test("moved_base_never_rebases_a_ref_that_is_not_checked_out", (t) => {
  const fx = fixture(t),
    candidate = sha(fx.repo);
  git(fx.repo, "checkout", "-b", "workspace", fx.base);
  advance(fx);
  const r = push(fx, ["origin", `${candidate}:dev`]);
  assert.equal(r.status, 1, r.out);
  assert.match(r.out, /is not checked out here/);
  assert.equal(sha(fx.repo), fx.base);
});
test("origin_unreadable_for_fetch_lands_on_gits_base_and_says_so", async (t) => {
  const fx = fixture(t);
  git(fx.repo, "config", "remote.origin.pushurl", fx.origin);
  git(fx.repo, "config", "remote.origin.url", join(fx.dir, "missing.git"));
  const r = push(fx);
  assert.equal(r.status, 0, r.out);
  assert.match(r.out, /origin unreadable .*landing on the base git sent/);
  await poll(() => calls(fx).find((l) => av(l).startsWith("release ")));
});
test("every_exit_0_after_the_phase_l_markers_is_hook_pass", () => {
  const tail = HOOK.slice(
    HOOK.indexOf("# ── Phase L: end") + "# ── Phase L: end".length,
  );
  let depth = 0;
  const bad = [];
  for (const [i, line] of tail.split("\n").entries()) {
    if (depth === 0 && /^\s*exit 0\s*(?:#.*)?$/.test(line)) bad.push(i);
    for (const c of line.replace(/#.*$/, "")) {
      if (c === "{") depth++;
      if (c === "}") depth = Math.max(0, depth - 1);
    }
  }
  assert.deepEqual(bad, []);
});
test("household_head_runs_before_the_final_hook_pass", () => {
  const end = HOOK.indexOf("# ── Household head: end"),
    last = HOOK.lastIndexOf("hook_pass");
  assert.ok(end >= 0 && end < last);
  assert.match(HOOK.slice(end, last), /brit_advisory/);
});
