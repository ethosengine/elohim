// Exercise the ACTUAL hook function: a tree without an attested submodule's
// checkout must be named before any gate runs, and a present one never is.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("../../", import.meta.url));
const HOOK = readFileSync(join(ROOT, ".husky/pre-push.bash"), "utf8");
const start = HOOK.indexOf("attested_submodules_missing() {\n");
const end = HOOK.indexOf("\n}\n", start);
assert.ok(start >= 0 && end > start, "hook has attested_submodules_missing() function");
const MISSING = HOOK.slice(start, end + "\n}\n".length);

function missing(statusLines) {
  const r = spawnSync("bash", ["-c", [
    "set -o pipefail",
    `git() { printf '%s\\n' ${statusLines.map((l) => `'${l}'`).join(" ")}; }`,
    MISSING,
    "attested_submodules_missing",
  ].join("\n")], { cwd: ROOT, encoding: "utf8", timeout: 10000 });
  assert.equal(r.status, 0, r.stdout + r.stderr);
  return r.stdout.split("\n").filter(Boolean);
}

test("names_only_uninitialized_attested_submodules", () => {
  assert.deepEqual(missing([
    "-76ba95d8a0c774d566d583a42629ac215abc4ba0 elohim/brit",
    " 9db775b3eca070ae89509251b0563b9e4d6518b1 elohim/rakia (heads/main)",
    "-1111111111111111111111111111111111111111 sophia",
  ]), ["elohim/brit", "sophia"]);
});

test("a_modified_or_conflicted_checkout_is_present", () => {
  assert.deepEqual(missing([
    "+76ba95d8a0c774d566d583a42629ac215abc4ba0 elohim/brit (heads/x)",
    "U9db775b3eca070ae89509251b0563b9e4d6518b1 elohim/rakia",
    " 1111111111111111111111111111111111111111 sophia (heads/main)",
  ]), []);
});

// Thin-device station 5 (backlog row 106): the hook asks attestedNeed, as prep does, with the
// push's own change list and selection. The real hook function, the real CLI, this checkout.
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";

const fnText = (name) => {
  const s = HOOK.indexOf(`${name}() {\n`);
  assert.ok(s >= 0, `hook has ${name}()`);
  return HOOK.slice(s, HOOK.indexOf("\n}\n", s) + 3);
};
const DECIDE = fnText("attested_decide");
const ALL = "elohim/brit elohim/rakia sophia";
const REFUSED = (need) => `[pre-push] REFUSED: not checked out: ${need}. Run: git submodule update --init ${need}`;

// What the CLI reads is a fixture root (GATE_ROOT), never this checkout: the answer must not depend
// on which submodules the device running the test holds. `thin` has no submodule manifest; `full`
// adds the three attested ones, shaped as the real ones are.
const step = (sources, depends = []) => ({ depends, inputs: { sources } });
const justIn = (dir, steps) => ({ dir, steps, run: { kind: "just" } });
const attestedIn = (dir, steps) => ({ dir, steps, run: { kind: "attested" } });
function fixtureRoot(t, { full = false } = {}) {
  const root = mkdtempSync(join(tmpdir(), "prepush-root-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const manifest = (path, content) => { mkdirSync(join(root, path), { recursive: true }); writeFileSync(join(root, path, "build-manifest.json"), JSON.stringify(content)); };
  writeFileSync(join(root, ".gitmodules"), ["elohim/brit", "elohim/rakia", "sophia", "elohim/holochain-conductor"].map((p) => `[submodule "${p}"]\n\tpath = ${p}\n`).join(""));
  manifest("genesis/orchestrator", { pipeline: "elohim-orchestrator", dependsOn: [], steps: { lint: step(["genesis/orchestrator/**"]) },
    gate: { projects: { orchestrator: justIn("genesis/orchestrator", ["lint"]), "orchestrator-tree": justIn("genesis/orchestrator", ["lint"]), "address-profile-lint": justIn(".", ["lint"]) } } });
  manifest("elohim/eprfs", { pipeline: "elohim-eprfs", dependsOn: [], steps: { "rust-build-test": step(["elohim/eprfs/**"]) },
    gate: { projects: { "eprfs-live-corpus": { dir: "elohim/eprfs", steps: [], inputs: { sources: ["genesis/docs/**"] }, run: { kind: "root-just" } } } } });
  manifest("elohim/holochain", { pipeline: "elohim-edge", dependsOn: [], steps: { "build-edge-image": step(["elohim/elohim-storage/**", "elohim/rakia/schemas/**"], ["elohim-brit:brit-ci"]) },
    gate: { projects: { "elohim-storage": justIn("elohim/elohim-storage", ["build-edge-image"]) } } });
  manifest("app/elohim-app", { pipeline: "elohim", dependsOn: ["elohim-sophia"], steps: { "build-angular": step(["app/**"], ["elohim-sophia:build-sophia-umd"]) },
    gate: { projects: { "elohim-app": justIn("app/elohim-app", ["build-angular"]) } } });
  if (full) {
    manifest("sophia", { pipeline: "elohim-sophia", dependsOn: [], steps: { "build-sophia-umd": { depends: [], inputs: { sources: ["sophia", "sophia/**"], buildProcess: ["sophia.Jenkinsfile"] } } }, gate: { projects: { sophia: attestedIn("sophia", ["build-sophia-umd"]) } } });
    manifest("elohim/brit", { pipeline: "elohim-brit", dependsOn: [], steps: { "brit-ci": step(["elohim/brit", "elohim/brit/**"]) }, gate: { projects: { brit: attestedIn("elohim/brit", ["brit-ci"]) } } });
    manifest("elohim/rakia", { pipeline: "elohim-rakia", dependsOn: [], steps: { "rakia-ci": step(["elohim/rakia", "elohim/rakia/**"]) }, gate: { projects: { rakia: attestedIn("elohim/rakia", ["rakia-ci"]) } } });
  }
  return root;
}

function decide(t, { changed = ["genesis/docs/a.md"], absent = ALL, moved = "", projects = "", env = {}, stub = "", full = false, cwd = ROOT } = {}) {
  const dir = mkdtempSync(join(tmpdir(), "prepush-attested-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  env = { GATE_ROOT: fixtureRoot(t, { full }), ...env };
  const list = join(dir, "changed");
  if (changed) writeFileSync(list, `${changed.join("\n")}\n`);
  const r = spawnSync("bash", ["-c", [
    "set -o pipefail",
    `HOOK_WORK_TREE=${JSON.stringify(ROOT)}; SECONDS=0`,
    `ATTESTED_MOVED=${JSON.stringify(moved)}`,
    `ATTESTED_ABSENT=${JSON.stringify(absent)}; ATTESTED_STARTED_S=0; ATTESTED_STARTED_AT=t0; PROJECTS=${JSON.stringify(projects)}`,
    'gate_step_reading() { echo "reading:$*"; }',
    stub,
    DECIDE,
    "attested_decide; echo \"decided rc=$? absent=[$ATTESTED_ABSENT]\"",
  ].join("\n")], {
    cwd, encoding: "utf8", timeout: 60000,
    env: { PATH: process.env.PATH, HOME: process.env.HOME, ...(changed ? { GATE_CHANGED_FILES: list } : {}), GATE_NARROW_BASE: "ok", ...env },
  });
  return { status: r.status, out: r.stdout.split("\n").filter(Boolean), err: r.stderr.split("\n").filter(Boolean) };
}

test("station 5 (a): a push whose changes give no reason passes with one line naming what was not required and why", (t) => {
  for (const projects of ["", "eprfs-live-corpus"]) {
    const r = decide(t, { projects });
    assert.equal(r.status, 0, r.err.join("\n"));
    assert.deepEqual(r.err, [`[pre-push] attested submodules absent or off their pin, and not required by this push: ${ALL} — no changed path or selected project needs them`]);
    assert.match(r.out[0], /^reading:0 submodules ran /, "the leg's reading is a green run");
    assert.equal(r.out.at(-1), "decided rc=0 absent=[]", "decided once");
  }
});

test("station 5 (b): every reason prep honours refuses here, naming only the submodules it applies to", (t) => {
  const cases = [
    [{ projects: "orchestrator" }, ALL, /orchestrator is selected/],
    [{ projects: "orchestrator-tree address-profile-lint" }, ALL, /orchestrator-tree is selected/],
    [{ changed: ["genesis/docs/a.md", "sophia"] }, "sophia", /sophia: the change touches sophia/],
    [{ changed: ["elohim/brit"] }, "elohim/brit", /elohim\/brit: the change touches elohim\/brit/],
    [{ changed: [".gitmodules"] }, ALL, /the change touches \.gitmodules/],
    [{ changed: ["genesis/orchestrator/attested-submodules.txt"] }, ALL, /the change touches genesis\/orchestrator\/attested-submodules\.txt/],
    [{ projects: "elohim-storage", absent: "elohim/rakia" }, "elohim/rakia", /elohim\/rakia: elohim-storage's manifest reads it/],
    [{ projects: "elohim-app" }, ALL, /elohim-app depends on elohim-sophia, which no checked-out manifest declares/],
    // The same questions where the submodule manifests ARE checked out (the orchestrator gate's own
    // view, 2026-11-26): a sophia pin move selects the attested sophia project and elohim-app one
    // hop down, and that is no reason to require brit or rakia.
    [{ full: true, changed: ["genesis/docs/a.md", "sophia"] }, "sophia", /^\[pre-push\] attested submodules required: sophia: the change touches sophia$/],
    [{ full: true, changed: ["sophia.Jenkinsfile"] }, "sophia", /sophia: the change touches sophia\.Jenkinsfile$/],
    [{ full: true, projects: "sophia elohim-app", changed: ["app/a.ts"], absent: "elohim/brit elohim/rakia" }, null, null],
    // A brit pin move there: elohim-storage is one hop down (its step depends on brit-ci) and its manifest reads rakia.
    [{ full: true, changed: ["elohim/brit"] }, "elohim/brit elohim/rakia", /elohim\/brit: the change touches elohim\/brit; elohim\/rakia: elohim-storage's manifest reads it/],
  ];
  for (const [options, need, why] of cases) {
    const r = decide(t, options);
    if (need === null) { assert.equal(r.status, 0, `${JSON.stringify(options)}: ${r.err.join(" | ")}`); continue; }
    assert.equal(r.status, 1, JSON.stringify(options));
    assert.match(r.err[0], /^\[pre-push\] attested submodules required: /);
    assert.match(r.err[0], why);
    assert.equal(r.err.at(-1), REFUSED(need), JSON.stringify(options));
    assert.match(r.out[0], /^reading:1 submodules ran /);
    assert.ok(!r.out.some((line) => line.startsWith("decided")), "the hook exits; nothing after the refusal runs");
  }
  assert.equal(decide(t, { changed: ["sophia"] }).err.at(-1), REFUSED("sophia"));
});

test("station 5 (c): no decision is a refusal of every absent one, in the line the hook printed before", (t) => {
  const closed = [
    ["no change list", { changed: null }],
    ["an unreadable change list", { env: { GATE_CHANGED_FILES: "/nonexistent/changed" } }],
    ["a fallback range", { env: { GATE_NARROW_BASE: "none" } }],
    ["a project the registry does not hold", { projects: "no-such-project" }],
    ["the decision crashed", { stub: "node() { echo boom >&2; return 1; }" }],
    ["node could not start", { stub: "node() { return 127; }" }],
    ["an older gate-preflight that ran its checks instead", { stub: "node() { echo '[preflight] ok epr-bin'; return 0; }" }],
    ["exit 3 naming nothing", { stub: "node() { return 3; }" }],
    // Review (c): xargs emptied these, and an empty answer with exit 0 passed.
    ["exit 0 with an unmatched quote in its output", { stub: "node() { echo \"it's broken\"; return 0; }" }],
    ["exit 0 with an unmatched double quote", { stub: "node() { echo 'say \"what'; return 0; }" }],
  ];
  for (const [what, options] of closed) {
    const r = decide(t, options);
    assert.equal(r.status, 1, what);
    assert.equal(r.err.at(-1), REFUSED(ALL), what);
    assert.match(r.out[0], /^reading:1 submodules ran /, what);
  }
  // The block's own refusals are still made where it stands, before any decision exists.
  const block = HOOK.slice(HOOK.indexOf("# ── Attested submodules checked out"), HOOK.indexOf("attested_decide() {"));
  assert.match(block, /ATTESTED_REFUSAL="the attested list could not be resolved/);
  assert.match(block, /ATTESTED_REFUSAL="git could not read the submodule state/);
  assert.match(block, /echo "\[pre-push\] REFUSED: \$ATTESTED_REFUSAL" >&2\n  gate_step_reading\n  exit 1/);
});

test("station 5 (d): the decision is this checkout's gate-preflight.mjs, resolved as the list resolver is", () => {
  assert.match(DECIDE, /node "\$\{HOOK_WORK_TREE:-\.\}\/genesis\/orchestrator\/gate-preflight\.mjs" --attested-need \$absent --projects \$PROJECTS/);
  assert.match(fnText("attested_submodules_list"), /node "\$\{HOOK_WORK_TREE:-\.\}\/genesis\/orchestrator\/attested-list\.mjs"/);
});

test("station 5 (e): a hand-over and a landing get the same answer, and every pass asks first", (t) => {
  const landing = { GATE_PUSH_TARGETS: "refs/heads/dev", GATE_PUSH_BRANCHES: "dev" };
  const handover = { GATE_PUSH_TARGETS: "refs/heads/self/thin-x", GATE_PUSH_BRANCHES: "self/thin-x", PUSH_IS_HANDOVER: "1" };
  for (const options of [{}, { projects: "orchestrator" }, { changed: ["sophia"] }, { changed: null }]) {
    const a = decide(t, { ...options, env: landing });
    const b = decide(t, { ...options, env: handover });
    const timeless = (lines) => lines.map((line) => line.replace(/^(reading:\d submodules ran) .*/, "$1"));
    assert.deepEqual([b.status, b.err, timeless(b.out)], [a.status, a.err, timeless(a.out)], JSON.stringify(options));
  }
  assert.doesNotMatch(DECIDE, /HANDOVER|GATE_PUSH|BERTH/);
  // hook_pass is the one way out for an early pass, a hand-over and a landing: it asks before either.
  assert.match(fnText("hook_pass"), /^hook_pass\(\) \{\n  attested_decide\n  attested_pins_gated\n  \[ -z "\$PUSH_IS_HANDOVER" \] \|\| handover_finish\n/);
  // And once the selection exists, before the preflight and every project gate.
  const selected = HOOK.indexOf("PROJECTS=$(echo \"$PROJECTS\" | sed 's/^ //')\nattested_decide\n");
  assert.ok(selected > 0 && selected < HOOK.indexOf("# ── Preflight (seconds)"));
  assert.ok(HOOK.indexOf("export GATE_CHANGED_FILES GATE_NARROW_BASE") < selected, "the change list exists by then");
  assert.ok(HOOK.indexOf("pass_or_refuse\nfi") > HOOK.indexOf("export GATE_CHANGED_FILES GATE_NARROW_BASE"), "no early pass comes before the change list");
});

test("station 5: a device that holds every attested submodule is asked nothing and prints nothing", (t) => {
  const r = decide(t, { absent: "", stub: "node() { echo 'node must not run' >&2; exit 9; }" });
  assert.equal(r.status, 0);
  assert.deepEqual(r.err, []);
  assert.deepEqual(r.out, ["decided rc=0 absent=[]"], "no reading from here: the block wrote it");
  assert.match(HOOK, /\[ -n "\$ATTESTED_ABSENT" \] \|\| gate_step_reading "\$READING_STATUS" submodules ran \\\n/);
});

// Review of 42faf6aaa (a): a thin client's FIRST hand-over is a new ref (destination all-zero), which
// the hook marks GATE_NARROW_BASE=none. For this question alone its range is the branch since its
// merge base with origin/dev. A scratch repository stands for the pushing checkout (GATE_ROOT).
import { decisionChanges, pinMoveRefusals } from "./gate-preflight.mjs";

function branchRepo(t, { firstCommit = ["genesis/docs/a.md"], dev = true, unrelated = false } = {}) {
  const dir = mkdtempSync(join(tmpdir(), "prepush-newref-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const git = (...args) => {
    const r = spawnSync("git", ["-c", "user.name=t", "-c", "user.email=t@example.invalid", ...args], { cwd: dir, encoding: "utf8" });
    assert.equal(r.status, 0, r.stderr);
    return r.stdout.trim();
  };
  const commit = (files, message) => {
    for (const file of files) { spawnSync("mkdir", ["-p", join(dir, file, "..")]); writeFileSync(join(dir, file), `${message}\n`); }
    git("add", "-A"); git("commit", "-q", "-m", message);
    return git("rev-parse", "HEAD");
  };
  git("init", "-q", "-b", "main");
  const base = commit(["README.md"], "base");
  if (dev) git("update-ref", "refs/remotes/origin/dev", base);
  if (unrelated) { git("checkout", "-q", "--orphan", "other"); git("rm", "-rqf", "."); }
  commit(firstCommit, "first");
  const tip = commit(["genesis/docs/b.md"], "second");
  return { dir, tip, env: { GATE_ROOT: dir, GATE_NARROW_BASE: "none", GATE_PUSH_REFS: `refs/heads/self/thin-x ${tip} ${"0".repeat(40)}` } };
}

test("review (a): a first hand-over to a new self/ branch is judged by its branch since origin/dev, and passes when that gives no reason", (t) => {
  const repo = branchRepo(t);
  assert.deepEqual(decisionChanges(["genesis/docs/b.md"], { env: repo.env, root: repo.dir, newRefs: true }).sort(), ["genesis/docs/a.md", "genesis/docs/b.md"]);
  assert.equal(decisionChanges(["genesis/docs/b.md"], { env: repo.env, root: repo.dir }), null, "every other check keeps treating a new ref as no list");
  const r = decide(t, { changed: ["genesis/docs/a.md", "genesis/docs/b.md"], env: repo.env });
  assert.equal(r.status, 0, r.err.join("\n"));
  assert.match(r.err[0], /absent or off their pin, and not required by this push: elohim\/brit elohim\/rakia sophia/);
});

test("review (a): the new branch's earlier commits count, and no origin/dev or no shared history requires every one", (t) => {
  // The pin move is in the FIRST commit; the hook's list here names only the tip's path.
  const moved = branchRepo(t, { firstCommit: ["sophia"] });
  const pin = decide(t, { changed: ["genesis/docs/b.md"], env: moved.env });
  assert.equal(pin.status, 1);
  assert.match(pin.err[0], /required: sophia: the change touches sophia/);
  assert.equal(pin.err.at(-1), REFUSED("sophia"));
  for (const [what, repo] of [["no origin/dev", branchRepo(t, { dev: false })], ["no merge base", branchRepo(t, { unrelated: true })]]) {
    assert.equal(decisionChanges(["genesis/docs/b.md"], { env: repo.env, root: repo.dir, newRefs: true }), null, what);
    const r = decide(t, { changed: ["genesis/docs/a.md", "genesis/docs/b.md"], env: repo.env });
    assert.equal(r.status, 1, what);
    assert.match(r.err[0], /no change list to judge it by/, what);
    assert.equal(r.err.at(-1), REFUSED(ALL), what);
  }
  // A destination tip this clone does not hold is no list either.
  const repo = branchRepo(t);
  const gone = { ...repo.env, GATE_NARROW_BASE: "ok", GATE_PUSH_REFS: `refs/heads/dev ${repo.tip} ${"1".repeat(40)}` };
  assert.equal(decisionChanges(["a.md"], { env: gone, root: repo.dir, newRefs: true }), null);
  assert.equal(decide(t, { env: gone }).err.at(-1), REFUSED(ALL));
});

test("review (c): the required names are read by collapsing whitespace, whatever else the lines hold", (t) => {
  const r = decide(t, { stub: "node() { printf 'sophia\\n  elohim/rakia \\n'; return 3; }" });
  assert.equal(r.err.at(-1), REFUSED("sophia elohim/rakia"));
  assert.doesNotMatch(DECIDE, /\| xargs/);
});

// 2026-11-26 01:29Z: a push moving only the elohim/brit gitlink landed on dev in 23 s with no gate.
// brit was checked out (`+c2f8193105`), at a commit older than the manifest that declares its
// attested project; nothing was absent, the oracle could not build its graph, path-only selected
// nothing. Three closures, each fail closed.
const STALE = "c2f8193105aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const PIN = "b52a100700bbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
// A repository whose HEAD pins elohim/brit at PIN: what the hook's `git ls-tree` reads in the pushing checkout.
function pinnedRepo(t) {
  const dir = mkdtempSync(join(tmpdir(), "prepush-pinned-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  for (const args of [["init", "-q", "-b", "main"], ["update-index", "--add", "--cacheinfo", `160000,${PIN},elohim/brit`],
    ["-c", "user.name=t", "-c", "user.email=t@example.invalid", "commit", "-q", "-m", "pin"]]) {
    const r = spawnSync("git", args, { cwd: dir, encoding: "utf8" });
    assert.equal(r.status, 0, r.stderr);
  }
  return dir;
}

test("incident (1): a submodule checked out off its gitlink is found, and judged like an absent one", (t) => {
  const r = spawnSync("bash", ["-c", [
    `git() { printf '%s\\n' '+${STALE} elohim/brit (heads/x)' ' 9db775b3eca070ae89509251b0563b9e4d6518b1 elohim/rakia (heads/main)' 'U1111111111111111111111111111111111111111 sophia'; }`,
    fnText("attested_submodules_moved"), "attested_submodules_moved",
  ].join("\n")], { encoding: "utf8" });
  assert.deepEqual(r.stdout.split("\n").filter(Boolean), [`elohim/brit ${STALE}`, "sophia 1111111111111111111111111111111111111111"]);
  assert.match(HOOK, /ATTESTED_MOVED=\$\(attested_submodules_moved "\$\{ATTESTED_LIST\[@\]\}"\)\nATTESTED_ABSENT=\$\(printf '%s %s' "\$MISSING_SUBMODULES"/);

  // The exact case: brit present at a stale commit, a pin-only change, nothing selected.
  const cwd = pinnedRepo(t);
  const stale = decide(t, { cwd, absent: "elohim/brit", moved: `elohim/brit ${STALE}`, changed: ["elohim/brit"] });
  assert.equal(stale.status, 1);
  assert.deepEqual(stale.err, [
    "[pre-push] attested submodules required: elohim/brit: the change touches elohim/brit",
    `[pre-push] REFUSED: elohim/brit is checked out at c2f8193105aa, not at its pin ${PIN.slice(0, 12)}: its gate would be read from a manifest and bytes the push does not carry. Run: just dev prep, or git submodule update --init elohim/brit`,
  ]);
  // Moved and absent together: one line each kind.
  const both = decide(t, { cwd, absent: "sophia elohim/brit", moved: `elohim/brit ${STALE}`, projects: "orchestrator" });
  assert.match(both.err[1], /^\[pre-push\] REFUSED: elohim\/brit is checked out at c2f8193105aa, not at its pin /);
  assert.equal(both.err[2], REFUSED("sophia"));
  // Someone working inside brit who pushes documents is not stopped, and is told.
  const docs = decide(t, { absent: "elohim/brit", moved: `elohim/brit ${STALE}` });
  assert.equal(docs.status, 0);
  assert.deepEqual(docs.err, ["[pre-push] attested submodules absent or off their pin, and not required by this push: elohim/brit — no changed path or selected project needs them"]);
});

const MODULES = new Set(["elohim/brit", "elohim/rakia", "sophia", "elohim/holochain-conductor"]);
const BRIT = { name: "brit", dir: "elohim/brit", run: { kind: "attested" } };
const pins = (over) => pinMoveRefusals({
  changed: ["elohim/brit"], selected: [], registry: new Map(), modules: MODULES, attested: ["elohim/brit", "elohim/rakia", "sophia"],
  targets: "refs/heads/dev", oracle: () => "unresolved dependency: step 'elohim-edge:build-edge-image' depends on 'elohim-brit:brit-ci' which does not exist", ...over,
});

test("incident (2) and (3): a pin move needs its attested gate in the selection, and on dev or main the oracle", () => {
  assert.deepEqual(pins({}), [
    "elohim/brit: its pin moves and no attested gate project is registered for it here (absent, or checked out at a commit older than the manifest that declares it) — the pin move would land ungated",
    "a push to dev or main that moves a pin (elohim/brit) is selected by the graph oracle or not at all, and the oracle is unavailable (unresolved dependency: step 'elohim-edge:build-edge-image' depends on 'elohim-brit:brit-ci' which does not exist) — path-only selection is not accepted for it",
  ]);
  const registry = new Map([["brit", BRIT], ["elohim-app", { name: "elohim-app", dir: "app/elohim-app", run: { kind: "just" } }]]);
  // A correctly prepared worktree, the same change: the attested gate is selected and the push goes on.
  assert.deepEqual(pins({ registry, selected: ["brit"], oracle: () => null }), []);
  assert.deepEqual(pins({ registry, selected: ["elohim-app"], oracle: () => null }),
    ["elohim/brit: its pin moves and the selection does not contain its attested gate brit — the pin move would land ungated"]);
  // (3) alone: selected by path, but the oracle that adds the pin's consumers was unavailable.
  assert.match(pins({ registry, selected: ["brit"] })[0], /^a push to dev or main that moves a pin \(elohim\/brit\) .* path-only selection is not accepted for it$/);
  assert.equal(pins({ registry, selected: ["brit"], targets: "refs/heads/main refs/heads/self/x" }).length, 1);
  // A hand-over is held to (2), not (3): its dev landing is where the oracle is demanded.
  assert.deepEqual(pins({ registry, selected: ["brit"], targets: "refs/heads/self/thin-x" }), []);
  assert.equal(pins({ targets: "refs/heads/self/thin-x" }).length, 1);
  // Any gitlink counts for (3); only attested ones have a gate to demand for (2).
  assert.equal(pins({ changed: ["elohim/holochain-conductor"] }).length, 1);
  assert.deepEqual(pins({ changed: ["elohim/holochain-conductor"], oracle: () => null }), []);
  // No pin change: nothing is asked, not even the oracle.
  assert.deepEqual(pins({ changed: ["genesis/docs/a.md", "elohim/brit/README.md"], oracle: () => { throw new Error("must not be asked"); } }), []);
  assert.deepEqual(pins({ changed: null }), ["the change list could not be read, so a submodule pin move cannot be ruled out"]);
});

function gated(t, { changed = ["genesis/docs/a.md"], stub = "", env = {}, projects = "", twice = false, full = false } = {}) {
  const dir = mkdtempSync(join(tmpdir(), "prepush-pins-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  env = { GATE_ROOT: fixtureRoot(t, { full }), ...env };
  const list = join(dir, "changed");
  if (changed) writeFileSync(list, `${changed.join("\n")}\n`);
  const r = spawnSync("bash", ["-c", [
    `HOOK_WORK_TREE=${JSON.stringify(ROOT)}; PROJECTS=${JSON.stringify(projects)}; ATTESTED_PINS_JUDGED=""`,
    'gate_step_reading() { :; }', stub, fnText("attested_pins_gated"),
    `attested_pins_gated${twice ? "; attested_pins_gated" : ""}; echo "went on rc=$?"`,
  ].join("\n")], { cwd: ROOT, encoding: "utf8", timeout: 120000, env: { PATH: process.env.PATH, HOME: process.env.HOME, GATE_CHANGED_FILES: list, GATE_NARROW_BASE: "ok", GATE_PUSH_TARGETS: "refs/heads/dev", ...env } });
  return { status: r.status, out: r.stdout.split("\n").filter(Boolean), err: r.stderr.split("\n").filter(Boolean) };
}
const LOUD = "node() { echo 'node ran' >&2; return 0; }";
const LAST = "[pre-push] REFUSED: a submodule pin move in this push is not gated (above). Run: just dev prep, then push again";

test("incident: the hook asks the pin-move question only for a push that moves a pin, once, and any failure of it refuses", (t) => {
  // All submodules at their pins and no pin change: no node, no output, the push goes on.
  assert.deepEqual(gated(t, { stub: LOUD }), { status: 0, out: ["went on rc=0"], err: [] });
  assert.deepEqual(gated(t, { changed: ["elohim/brit/README.md", "sophia.Jenkinsfile"], stub: LOUD }).err, []);
  // A pin move asks; yes goes on, once.
  assert.deepEqual(gated(t, { changed: ["a.md", "elohim/brit"], stub: LOUD, twice: true }), { status: 0, out: ["went on rc=0"], err: ["node ran"] });
  assert.deepEqual(gated(t, { changed: ["elohim/holochain-conductor"], stub: LOUD }).err, ["node ran"]);
  for (const [what, stub] of [["a refusal", "node() { echo '[pre-push] why' >&2; return 3; }"], ["a crash", "node() { return 1; }"], ["node cannot start", "node() { return 127; }"]]) {
    const r = gated(t, { changed: ["elohim/brit"], stub });
    assert.equal(r.status, 1, what);
    assert.equal(r.err.at(-1), LAST, what);
    assert.deepEqual(r.out, [], what);
  }
  // A change list that cannot be compared is asked about, never assumed clean.
  assert.deepEqual(gated(t, { env: { GATE_CHANGED_FILES: "/nonexistent/changed" }, stub: LOUD }).err, ["node ran"]);
  // The real question, this checkout: a brit pin move to dev with nothing selected does not go on.
  const real = gated(t, { changed: ["elohim/brit"] });
  assert.equal(real.status, 1, real.err.join("\n"));
  assert.match(real.err[0], /^\[pre-push\] elohim\/brit: its pin moves and no attested gate project is registered for it here .* — the pin move would land ungated$/);
  assert.equal(real.err.at(-1), LAST);
  // Where brit's manifest is checked out: not selected refuses; selected, a hand-over goes on.
  const handover = { GATE_PUSH_TARGETS: "refs/heads/self/thin-x" };
  assert.deepEqual(gated(t, { full: true, changed: ["elohim/brit"], env: handover }).err,
    ["[pre-push] elohim/brit: its pin moves and the selection does not contain its attested gate brit — the pin move would land ungated", LAST]);
  assert.deepEqual(gated(t, { full: true, changed: ["elohim/brit"], env: handover, projects: "brit elohim-storage" }), { status: 0, out: ["went on rc=0"], err: [] });
  assert.equal(gated(t, { changed: null, env: { GATE_CHANGED_FILES: "/nonexistent/changed" } }).status, 1, "no list, real CLI: refused");
  // Asked where the selection exists and at every pass, as attested_decide is.
  assert.match(fnText("hook_pass"), /^hook_pass\(\) \{\n  attested_decide\n  attested_pins_gated\n/);
  assert.ok(HOOK.includes("PROJECTS=$(echo \"$PROJECTS\" | sed 's/^ //')\nattested_decide\nattested_pins_gated\n"));
});
