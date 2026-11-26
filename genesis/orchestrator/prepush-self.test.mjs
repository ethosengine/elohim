// The `self/<name>` hand-over lane (backlog row 9, ruling R1, step 1), driven through the WHOLE real
// .husky/pre-push.bash — not a slice — in a throwaway scratch repository whose cheap checks and gate
// runner are logging stubs. A hand-over runs the seconds checks, no project gate, and writes one
// advisory record per pushed head; anything that is not purely self/ destinations is the ordinary
// push; a dev push of a head that was handed over pays full demand and never reads the record.
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import { appendBatch, cidString, handoverLines, pushedHeads } from "./handover-record.mjs";
import * as realFs from "node:fs";
import { rawCid } from "./scripts/brit-attest.mjs";

const ROOT = fileURLToPath(new URL("../../", import.meta.url));
const HOOK_PATH = join(ROOT, ".husky/pre-push.bash");
const ZERO = "0".repeat(40);
const RECORD = ".eprfs/status/handover/handovers.jsonl";

const cleanEnv = Object.fromEntries(
  Object.entries(process.env).filter(([k]) =>
    (!k.startsWith("GIT_") || k.startsWith("GIT_CONFIG_")) &&
    !["HUSKY", "GATE_ON_RED", "GATE_LIVE_RUN", "PUSH_IS_HANDOVER", "RUN_SWEETTEST", "SKIP_SWEETTEST"].includes(k)),
);
// The fixture's git reads no device config: a global or system file can name repository paths
// (core.hooksPath, include.path) the gate key does not cover (E11). Identity is set per fixture.
Object.assign(cleanEnv, { GIT_CONFIG_NOSYSTEM: "1", GIT_CONFIG_GLOBAL: "/dev/null" });

const exe = (path, text) => {
  mkdirSync(join(path, ".."), { recursive: true });
  writeFileSync(path, text);
  chmodSync(path, 0o755);
};

// Every stub appends one line to stub.log, so a test reads which legs ran and in what order.
function fixture(t, { hook = readFileSync(HOOK_PATH, "utf8") } = {}) {
  const dir = mkdtempSync(join(tmpdir(), "prepush-self-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const git = (...a) => {
    const r = spawnSync("git", a, { cwd: dir, env: cleanEnv, encoding: "utf8" });
    assert.equal(r.status, 0, r.stderr);
    return r.stdout.trim();
  };
  git("init", "-q", "-b", "main");
  git("config", "user.email", "t@example.invalid");
  git("config", "user.name", "t");
  writeFileSync(join(dir, "a.txt"), "a\n");
  git("add", "a.txt");
  git("commit", "-q", "-m", "base");
  const base = git("rev-parse", "HEAD");
  git("update-ref", "refs/remotes/origin/dev", base);
  mkdirSync(join(dir, "src"));
  writeFileSync(join(dir, "src/x.txt"), "x\n");
  git("add", "src/x.txt");
  git("commit", "-q", "-m", "feature one");
  const one = git("rev-parse", "HEAD");
  writeFileSync(join(dir, "src/y.txt"), "y\n");
  git("add", "src/y.txt");
  git("commit", "-q", "-m", "feature two");
  const two = git("rev-parse", "HEAD");

  mkdirSync(join(dir, ".husky"), { recursive: true });
  writeFileSync(join(dir, ".husky/pre-push.bash"), hook);
  mkdirSync(join(dir, "genesis/orchestrator"), { recursive: true });
  // The hook reads each pushed ref's range through this script; the real one, so a malformed line is
  // read exactly as the hook reads it (push-changed-files.test.mjs holds its own parity proofs).
  writeFileSync(join(dir, "genesis/orchestrator/push-changed-files.sh"),
    readFileSync(join(ROOT, "genesis/orchestrator/push-changed-files.sh"), "utf8"));
  // The attested-list resolver has its own tests (attested-list.test.mjs); this fixture has no
  // submodules, so its stand-in resolver names a tracked plain file, which git reports as no missing submodule.
  writeFileSync(join(dir, "genesis/orchestrator/attested-list.mjs"), 'process.stdout.write("src/y.txt\\n");\n');
  const log = `echo "$*" >> "$(git rev-parse --show-toplevel)/stub.log"`;
  exe(join(dir, ".claude/scripts/dependabot-paydown.py"),
    `import os, subprocess, sys\nroot = subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()\nopen(os.path.join(root, "stub.log"), "a").write("PAYDOWN " + " ".join(sys.argv[1:]) + "\\n")\n`);
  exe(join(dir, ".claude/scripts/epr-meta-git-gate.py"),
    `import os, subprocess, sys\nroot = subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()\nopen(os.path.join(root, "stub.log"), "a").write("COMPOSE " + " ".join(sys.argv[1:]) + "\\n")\n`);
  // Like the real wrapper (rust-style.sh: cargo build before the checker), the stub builds first when
  // STUB_STYLE_CARGO=1, so a device's cargo decides whether it gets as far as the check.
  exe(join(dir, "genesis/orchestrator/rust-style.sh"), `#!/usr/bin/env bash\nif [ "\${STUB_STYLE_CARGO:-}" = 1 ]; then cargo build --quiet --locked || exit $?; fi\n${log.replace('"$*"', '"STYLE $*"')}\nexit "\${STUB_STYLE_RC:-0}"\n`);
  exe(join(dir, "genesis/orchestrator/scripts/t2-receipt.sh"), `#!/usr/bin/env bash\n${log.replace('"$*"', '"T2 $*"')}\n`);
  exe(join(dir, "genesis/orchestrator/gate-runner.mjs"), `import {appendFileSync, readFileSync} from "node:fs";
const a = process.argv.slice(2);
if (a.includes("--names")) { readFileSync(0, "utf8"); console.log("alpha"); console.log("beta"); process.exit(0); }
appendFileSync("stub.log", "GATE " + a.join(" ") + "\\n");
`);
  exe(join(dir, "genesis/orchestrator/gate-preflight.mjs"), `import {appendFileSync} from "node:fs";
const a = process.argv.slice(2);
if (a.includes("--on-red")) { console.log("continue"); process.exit(0); }
appendFileSync("stub.log", "PREFLIGHT " + a.join(" ") + "\\n");
process.exit(Number(process.env.STUB_PREFLIGHT_RC || 0));
`);
  // The real recorder, reached through a shim at the path the hook names.
  exe(join(dir, "genesis/orchestrator/handover-record.mjs"),
    `import { main } from ${JSON.stringify(join(ROOT, "genesis/orchestrator/handover-record.mjs"))};\nprocess.exitCode = main(process.argv.slice(2));\n`);
  // gate-live: absent in effect unless STUB_LIVE=1, then a run opens and a stop is requested on it.
  exe(join(dir, "genesis/orchestrator/gate-live.mjs"), `const a = process.argv.slice(2);
if (process.env.STUB_LIVE !== "1") process.exit(1);
if (a[0] === "open") console.log("run-1");
process.exit(0);
`);
  exe(join(dir, "genesis/orchestrator/scripts/brit-helper.sh"), `#!/usr/bin/env bash\n${log.replace('"$*"', '"BRITHELPER $*"')}\n`);
  // berth refuses everything: a dev push runs every gate, then is refused at its landing (Phase L).
  exe(join(dir, "genesis/agentic/bin/berth"), `#!/usr/bin/env bash\n${log.replace('"$*"', '"BERTH $*"')}\nexit 1\n`);
  // brit-build-ref: commit "one" holds alpha green and beta red; gamma's note is unverifiable.
  const verdictBytes = [1, 113, 18, 32, ...createHash("sha256").update("verdict").digest()];
  exe(join(dir, "bin/brit-build-ref"), `#!/usr/bin/env bash
${log.replace('"$*"', '"BUILDREF $*"')}
args=" $* "
case "$args" in
  *" build list "*) printf 'ci:elohim\\ngate:alpha\\ngate:beta\\ngate:gamma\\n'; exit 0 ;;
esac
step=""; commit=""
while [ $# -gt 0 ]; do case "$1" in --step) step=$2; shift ;; --commit) commit=$2; shift ;; esac; shift; done
if [ "$commit" = "${one}" ]; then
  case "$step" in
    gate:alpha) echo '{"agentId":"aa","inputsHash":"closure-alpha","success":true,"outputCid":${JSON.stringify(verdictBytes)}}'; exit 0 ;;
    gate:beta) echo '{"agentId":"aa","inputsHash":"closure-beta","success":false,"outputCid":${JSON.stringify(verdictBytes)}}'; exit 0 ;;
    gate:gamma) echo "unverifiable: ${one} bad signature" >&2; exit 0 ;;
  esac
fi
echo "no build ref found for step=$step commit=$commit" >&2
`);
  // A device with cargo, whatever the host running this test has (the suite's view has none on
  // PATH): the hook asks `command -v cargo` before the style check; thinClient() replaces this.
  exe(join(dir, "bin/cargo"), "#!/usr/bin/env bash\nexit 0\n");
  // Never pause a real rust-analyzer of the workspace running this test.
  exe(join(dir, "bin/pgrep"), "#!/usr/bin/env bash\nexit 1\n");
  const readingEpr = join(dir, "bin/epr-reading");
  exe(readingEpr, `#!/usr/bin/env bash
printf '%s\\n' "$*" >> "${dir}/reading-calls.log"
while [ "$#" -gt 0 ]; do
  if [ "$1" = --from ]; then cp "$2" "${dir}/reading-rows.jsonl"; break; fi
  shift
done
exit "\${STUB_READING_RC:-0}"
`);
  return { dir, git, base, one, two, readingEpr, verdictCid: cidString(verdictBytes) };
}

function push(fx, lines, env = {}) {
  const r = spawnSync("bash", [join(fx.dir, ".husky/pre-push.bash"), "origin", "https://example.invalid/r.git"], {
    cwd: fx.dir,
    input: lines.length ? `${lines.join("\n")}\n` : "",
    encoding: "utf8",
    timeout: 90000,
    env: {
      ...cleanEnv,
      PATH: `${join(fx.dir, "bin")}:${cleanEnv.PATH}`,
      BRIT_BUILD_REF_BIN: join(fx.dir, "bin/brit-build-ref"),
      ELOHIM_DEVICE_KEY_FILE: join(fx.dir, "no-device-key"),
      EPR_BIN: "/nonexistent/epr",
      GATE_READING: "off",
      BERTH_PREPUSH_TTL_S: "30",
      ...env,
    },
  });
  const log = existsSync(join(fx.dir, "stub.log")) ? readFileSync(join(fx.dir, "stub.log"), "utf8") : "";
  rmSync(join(fx.dir, "stub.log"), { force: true });
  const recordPath = join(fx.dir, RECORD);
  const records = existsSync(recordPath)
    ? readFileSync(recordPath, "utf8").split("\n").filter(Boolean).map(l => JSON.parse(l)) : [];
  return { ...r, out: r.stdout + r.stderr, log, records };
}

const gatesRun = log => (log.match(/^GATE --target (\S+)/gm) || []).map(l => l.split(" ").at(-1));

function assertHandOver(r) {
  assert.equal(r.status, 0, `signal=${r.signal} error=${r.error?.message || "none"}\n${r.out}`);
  assert.deepEqual(gatesRun(r.log), [], `no project gate runs on a hand-over\n${r.log}`);
  assert.match(r.log, /^PREFLIGHT --projects alpha beta$/m, "the selected projects' preflight runs");
  // The fixture's own ranges touch no style input: the leg answers without running the checker.
  assert.match(r.log + r.out, /^STYLE |rust-style: no input of the style check changed in [0-9a-f]{12}\.\.[0-9a-f]{12}; not run/m,
    "the style leg answers for every pushed ref");
  assert.match(r.log, /^PAYDOWN /m, "security paydown runs");
  assert.match(r.log, /^COMPOSE /m, ".epr-meta compose gate runs");
  assert.match(r.log, /^T2 /m, "the T2 receipt check runs");
  assert.match(r.out, /hand-over: project gates not run here: alpha beta/);
}

function assertOrdinary(r, { status = 0 } = {}) {
  assert.deepEqual(gatesRun(r.log), ["alpha", "beta"], `the ordinary push runs every selected gate\n${r.out}`);
  assert.equal(r.status, status, r.out);
  assert.equal(r.records.length, 0, "the ordinary push writes no hand-over record");
  assert.doesNotMatch(r.out, /hand-over/);
}

const line = (local, sha, remote, remoteSha = ZERO) => `${local} ${sha} ${remote} ${remoteSha}`;

// One commit on the fixture's current HEAD: write `files`, delete `remove`, return the new sha.
function commitOn(fx, files, message, remove = []) {
  for (const [path, body] of Object.entries(files)) {
    mkdirSync(join(fx.dir, path, ".."), { recursive: true });
    writeFileSync(join(fx.dir, path), body);
    fx.git("add", "--", path);
  }
  for (const path of remove) fx.git("rm", "-q", "--", path);
  fx.git("commit", "-q", "--allow-empty", "-m", message);
  return fx.git("rev-parse", "HEAD");
}

// The hook's paydown leg runs python3 on every push; without it the fixture cannot be driven, and
// that is a red, never a skip: the named half also refuses to start without a working python3.
if (spawnSync("python3", ["-c", ""]).status !== 0) {
  throw new Error("prepush-self needs a python3 that runs `python3 -c ''` (the hook's paydown leg runs it): install one on PATH");
}

test("whole hook appends readings once when on and preserves its exit status", t => {
  const fx = fixture(t);
  const r = push(fx, [line("refs/heads/topic", fx.one, "refs/heads/self/topic")], {
    GATE_READING: "on",
    EPR_BIN: fx.readingEpr,
  });
  assert.equal(r.status, 0, `signal=${r.signal} error=${r.error?.message || "none"}\n${r.out}`);
  const calls = readFileSync(join(fx.dir, "reading-calls.log"), "utf8").trim().split("\n");
  assert.equal(calls.length, 2);
  assert.match(calls[0], /^flow reading append --from .*\/rows\.jsonl --root /);
  assert.match(calls[1], /^flow report reading --crate-shadow --run \S+ --root /);
  const rows = readFileSync(join(fx.dir, "reading-rows.jsonl"), "utf8")
    .trim().split("\n").map(row => JSON.parse(row));
  assert.ok(rows.length > 1);
  assert.equal(rows.filter(row => row.synthesized).length > 0, true);
});

test("whole hook makes no reading append when off", t => {
  const fx = fixture(t);
  const r = push(fx, [line("refs/heads/topic", fx.one, "refs/heads/self/topic")], {
    GATE_READING: "off",
    EPR_BIN: fx.readingEpr,
  });
  assert.equal(r.status, 0, r.out);
  assert.equal(existsSync(join(fx.dir, "reading-calls.log")), false);
});

test("a self-only push runs the seconds checks, no project gate, and records the pushed head", t => {
  const fx = fixture(t);
  const r = push(fx, [line("refs/heads/topic", fx.one, "refs/heads/self/topic")]);
  assertHandOver(r);
  assert.equal(r.records.length, 1, r.out);
  const [rec] = r.records;
  assert.equal(rec.schema, "handover-record@1");
  assert.equal(rec.head, fx.one);
  assert.deepEqual(rec.destinations, ["refs/heads/self/topic"]);
  assert.ok(Number.isFinite(Date.parse(rec.at)), rec.at);
  assert.match(rec.advisory, /never a gate pass/);
  assert.equal(rec.inventory, "read");
  assert.deepEqual(rec.verdicts, [
    { project: "alpha", closure: "closure-alpha", verdict: fx.verdictCid, signer: "aa", verdictOf: "green" },
    { project: "beta", closure: "closure-beta", verdict: fx.verdictCid, signer: "aa", verdictOf: "red" },
  ]);
  assert.deepEqual(rec.unreadable, [{ project: "gamma", state: "unverifiable", reason: "bad signature" }]);
  assert.match(r.out, /hand-over [0-9a-f]{12} → refs\/heads\/self\/topic: verdicts this device holds for this head: alpha green, beta red; UNREADABLE evidence: gamma \(unverifiable: bad signature\)/);
  assert.match(r.out, /landing this head on dev demands every gate/);
});

test("an empty inventory is recorded and printed as none", t => {
  const fx = fixture(t);
  const r = push(fx, [line("refs/heads/topic", fx.two, "refs/heads/self/topic")]);
  assertHandOver(r);
  assert.equal(r.records.length, 1);
  assert.deepEqual(r.records[0].verdicts, []);
  assert.deepEqual(r.records[0].unreadable, []);
  assert.match(r.out, /verdicts this device holds for this head: none/);
});

for (const [name, lines] of [
  ["self then feature", f => [line("refs/heads/a", f.one, "refs/heads/self/a"), line("refs/heads/b", f.two, "refs/heads/feat-b")]],
  ["feature then self", f => [line("refs/heads/b", f.two, "refs/heads/feat-b"), line("refs/heads/a", f.one, "refs/heads/self/a")]],
  ["self then a notes ref", f => [line("refs/heads/a", f.one, "refs/heads/self/a"), line("refs/notes/brit/build/x", f.two, "refs/notes/brit/build/x")]],
  ["a tag then self", f => [line("refs/tags/v1", f.two, "refs/tags/v1"), line("refs/heads/a", f.one, "refs/heads/self/a")]],
  ["a local self/ ref to a feature branch", f => [line("refs/heads/self/a", f.one, "refs/heads/feat-a")]],
  ["a near-miss destination selfie/", f => [line("refs/heads/a", f.one, "refs/heads/selfie/a")]],
  ["a nested destination feat/self/", f => [line("refs/heads/a", f.one, "refs/heads/feat/self/a")]],
  ["a malformed line with three fields", f => [`refs/heads/a ${f.one} refs/heads/self/a`]],
  ["a malformed line with five fields", f => [`${line("refs/heads/a", f.one, "refs/heads/self/a")} extra`]],
  ["a malformed object name", f => [line("refs/heads/a", "HEAD", "refs/heads/self/a"), line("refs/heads/b", f.one, "refs/heads/feat-b")]],
]) {
  test(`mixed or malformed — ${name} — takes the ordinary path`, t => {
    const fx = fixture(t);
    assertOrdinary(push(fx, lines(fx)));
  });
}

for (const [name, lines] of [
  ["self/x:dev (a local self/ ref pushed to dev)", f => [line("refs/heads/self/x", f.one, "refs/heads/dev", f.base)]],
  ["self and dev, self first", f => [line("refs/heads/a", f.one, "refs/heads/self/a"), line("refs/heads/b", f.one, "refs/heads/dev", f.base)]],
  ["dev and self, dev first", f => [line("refs/heads/b", f.one, "refs/heads/dev", f.base), line("refs/heads/a", f.one, "refs/heads/self/a")]],
  ["a protected deletion beside a self push", f => [line("refs/heads/a", f.one, "refs/heads/self/a"), line("(delete)", ZERO, "refs/heads/main", f.base)]],
]) {
  test(`${name} pays full demand and is landed (refused at the landing by the berth stub)`, t => {
    const fx = fixture(t);
    const r = push(fx, lines(fx));
    // The hook runs every gate, then the landing claim refuses: the stub berth refuses everything.
    assertOrdinary(r, { status: 1 });
    assert.match(r.log, /^BERTH /m, "the landing claim was reached");
  });
}

test("a local branch named dev pushed to self/ is a hand-over: only the destination counts", t => {
  const fx = fixture(t);
  const r = push(fx, [line("refs/heads/dev", fx.one, "refs/heads/self/dev")]);
  assertHandOver(r);
  assert.equal(r.records.length, 1);
});

test("a blank line beside a self push is not a hand-over", t => {
  const fx = fixture(t);
  const r = push(fx, ["", line("refs/heads/a", fx.one, "refs/heads/self/a")]);
  assert.equal(r.records.length, 0, r.out);
  assert.doesNotMatch(r.out, /hand-over/);
});

test("empty stdin is the ordinary push: nothing changed, nothing recorded", t => {
  const fx = fixture(t);
  const r = push(fx, []);
  assert.equal(r.status, 0, r.out);
  assert.deepEqual(gatesRun(r.log), []);
  assert.equal(r.records.length, 0);
  assert.doesNotMatch(r.out, /hand-over/);
});

test("deleting a self/ ref is allowed and records nothing", t => {
  const fx = fixture(t);
  const r = push(fx, [line("(delete)", ZERO, "refs/heads/self/old", fx.one)]);
  assert.equal(r.status, 0, r.out);
  assert.deepEqual(gatesRun(r.log), []);
  assert.equal(r.records.length, 0);
  assert.match(r.out, /hand-over: deletion only — nothing recorded/);
});

test("two self/ refs with different heads write two records; a shared head writes one", t => {
  const fx = fixture(t);
  const r = push(fx, [
    line("refs/heads/a", fx.one, "refs/heads/self/a"),
    line("refs/heads/b", fx.two, "refs/heads/self/b"),
    line("refs/heads/c", fx.two, "refs/heads/self/c"),
  ]);
  assert.equal(r.status, 0, r.out);
  assert.deepEqual(gatesRun(r.log), []);
  assert.deepEqual(r.records.map(x => [x.head, x.destinations]), [
    [fx.one, ["refs/heads/self/a"]],
    [fx.two, ["refs/heads/self/b", "refs/heads/self/c"]],
  ]);
  assert.equal((r.out.match(/^\[pre-push\] hand-over [0-9a-f]{12} → /gm) || []).length, 2, r.out);
});

for (const [name, env, red, rs] of [
  ["a red style check", { STUB_STYLE_RC: "1" }, /RED rust-style/, true],
  ["a red preflight", { STUB_PREFLIGHT_RC: "1" }, /RED preflight/, false],
]) {
  test(`${name} refuses the hand-over and writes no record`, t => {
    const fx = fixture(t);
    // The style check runs only for a range that touches one of its inputs.
    const head = rs ? commitOn(fx, { "src/lib.rs": "pub fn f() {}\n" }, "rust") : fx.one;
    const r = push(fx, [line("refs/heads/a", head, "refs/heads/self/a")], env);
    assert.equal(r.status, 1, r.out);
    assert.match(r.out, red);
    assert.match(r.out, /REFUSED: red before the gates/);
    assert.deepEqual(gatesRun(r.log), []);
    assert.equal(r.records.length, 0);
    // The reds accumulate: the checks after the red still ran.
    assert.match(r.log, /^COMPOSE /m);
    assert.match(r.log, /^T2 /m);
  });
}

test("a record that cannot be written refuses the hand-over", t => {
  const fx = fixture(t);
  mkdirSync(join(fx.dir, ".eprfs/status"), { recursive: true });
  writeFileSync(join(fx.dir, ".eprfs/status/handover"), "a file where the log directory belongs\n");
  const r = push(fx, [line("refs/heads/a", fx.one, "refs/heads/self/a")]);
  assert.equal(r.status, 1, r.out);
  assert.match(r.out, /handover-record: could not write/);
  assert.match(r.out, /REFUSED: the hand-over record was not written/);
  assert.deepEqual(gatesRun(r.log), []);
});

test("a dev push of a head already handed over pays full demand and never reads the record", t => {
  const fx = fixture(t);
  const handed = push(fx, [line("refs/heads/a", fx.one, "refs/heads/self/a")]);
  assertHandOver(handed);
  const before = readFileSync(join(fx.dir, RECORD), "utf8");
  for (const env of [{}, { PUSH_IS_HANDOVER: "1" }]) {
    const r = push(fx, [line("refs/heads/a", fx.one, "refs/heads/dev", fx.base)], env);
    assertOrdinary({ ...r, records: [] }, { status: 1 });
    assert.match(r.log, /^BERTH /m, "the landing was reached after every gate ran");
    assert.equal(readFileSync(join(fx.dir, RECORD), "utf8"), before, "the record is untouched");
    assert.doesNotMatch(r.log, /BUILDREF build list/, "no hand-over inventory is read on the dev path");
  }
});

test("the hook names the record only inside the hand-over branch", () => {
  const hook = readFileSync(HOOK_PATH, "utf8");
  const uses = hook.split("\n").filter(l => l.includes("handover-record.mjs") && !l.trimStart().startsWith("#"));
  assert.equal(uses.length, 1, uses.join("\n"));
  assert.match(uses[0], /node "\$HOOK_WORK_TREE\/genesis\/orchestrator\/handover-record\.mjs"/);
  assert.ok(!hook.includes("handovers.jsonl"), "the hook never reads the log");
});

test("negative control: loosening the classifier to 'any destination matches' fails the mixed case", t => {
  const strict = readFileSync(HOOK_PATH, "utf8");
  const tight = '[[ "$remote_ref" == "refs/heads/self/"?* ]] || return 1\n  done\n  return 0';
  assert.ok(strict.includes(tight), "the classifier's all-destinations rule is where this test expects it");
  const loose = strict.replace(tight, '[[ "$remote_ref" == "refs/heads/self/"?* ]] && return 0\n  done\n  return 1');
  const fx = fixture(t, { hook: loose });
  const r = push(fx, [line("refs/heads/a", fx.one, "refs/heads/self/a"), line("refs/heads/b", fx.two, "refs/heads/feat-b")]);
  // The ordinary-path assertion catches the loosened hook: no gate ran for the feature ref.
  assert.throws(() => assertOrdinary(r), /the ordinary push runs every selected gate/);
  try { assertOrdinary(r); } catch (error) { t.diagnostic(`negative control caught: ${error.message.split("\n")[0]}`); }
  assert.deepEqual(gatesRun(r.log), []);
  // And the recorder's own check is a second fence: it refuses a batch that is not all self/.
  assert.equal(r.status, 1, r.out);
  assert.match(r.out, /not a hand-over batch/);
});

test("the recorder's own classification and CID encoding", () => {
  const one = "1".repeat(40);
  assert.equal(handoverLines(""), null);
  assert.equal(handoverLines("\n"), null);
  assert.deepEqual(handoverLines(`refs/heads/a ${one} refs/heads/self/a ${ZERO}\n`).map(l => l.remoteRef), ["refs/heads/self/a"]);
  for (const bad of [
    `refs/heads/a ${one} refs/heads/dev ${ZERO}`,
    `refs/heads/a ${one} refs/heads/self/ ${ZERO}`,
    `refs/heads/a ${one} refs/notes/self/a ${ZERO}`,
    `refs/heads/a ${one} refs/heads/self/a`,
    `refs/heads/a ${one} refs/heads/self/a ${ZERO}\nrefs/heads/b ${one} refs/heads/feat-b ${ZERO}`,
  ]) {
    assert.equal(handoverLines(bad), null, bad);
  }
  assert.deepEqual(pushedHeads(handoverLines(`(delete) ${ZERO} refs/heads/self/a ${one}\n`)), []);
  const digest = createHash("sha256").update("x").digest();
  assert.equal(cidString([0x01, 0x55, 0x12, 0x20, ...digest]), rawCid(Buffer.from("x")));
  assert.equal(cidString("not bytes"), null);
});

test("a destination carrying U+2028 classifies as a hand-over in the hook and the recorder alike", t => {
  const fx = fixture(t);
  const ref = "refs/heads/self/a\u2028b";
  const r = push(fx, [line("refs/heads/a", fx.one, ref)]);
  assertHandOver(r);
  assert.deepEqual(r.records.map(x => x.destinations), [[ref]]);
});

test("a live stop request refuses the hand-over before it is recorded", t => {
  const fx = fixture(t);
  const r = push(fx, [line("refs/heads/a", fx.one, "refs/heads/self/a")], { STUB_LIVE: "1" });
  assert.equal(r.status, 1, r.out);
  assert.match(r.out, /stopped: stop requested on the live run; the hand-over is not recorded/);
  assert.deepEqual(gatesRun(r.log), []);
  assert.equal(r.records.length, 0);
});

test("the warn-only Brit advisory is reachable on a hand-over only when forced", t => {
  const fx = fixture(t);
  const plain = push(fx, [line("refs/heads/a", fx.one, "refs/heads/self/a")]);
  assertHandOver(plain);
  assert.doesNotMatch(plain.log, /^BRITHELPER /m);
  const forced = push(fx, [line("refs/heads/a", fx.one, "refs/heads/self/a")], { BRIT_ADVISORY: "always" });
  assertHandOver(forced);
  assert.match(forced.log, /^BRITHELPER verify$/m);
});

for (const [name, launcher] of [["with its launcher", true], ["without its launcher", false]]) {
  test(`a differing hook copy is named; the hooksPath override is offered only ${name}`, t => {
    const fx = fixture(t);
    // The running copy lives elsewhere and differs from the checkout's .husky/pre-push.bash.
    const other = join(fx.dir, "elsewhere/pre-push.bash");
    mkdirSync(join(fx.dir, "elsewhere"));
    writeFileSync(other, readFileSync(join(fx.dir, ".husky/pre-push.bash"), "utf8") + "\n# a differing copy\n");
    if (launcher) exe(join(fx.dir, ".husky/_/pre-push"), "#!/usr/bin/env sh\n");
    const r = spawnSync("bash", [other, "origin", "u"], {
      cwd: fx.dir, input: "", encoding: "utf8", timeout: 60000,
      env: { ...cleanEnv, PATH: `${join(fx.dir, "bin")}:${cleanEnv.PATH}`, EPR_BIN: "/nonexistent/epr", GATE_READING: "off" },
    });
    assert.equal(r.status, 0, r.stdout + r.stderr);
    assert.match(r.stderr, /note: running .*elsewhere\/pre-push\.bash, which differs/);
    if (launcher) {
      assert.ok(r.stderr.includes(`git -c core.hooksPath='${join(fx.dir, ".husky/_")}' push`), r.stderr);
    } else {
      assert.doesNotMatch(r.stderr, /core\.hooksPath/);
      assert.match(r.stderr, /run pnpm install in it first/);
    }
  });
}

test("a short write is rolled back so the log keeps whole lines, and the writer throws", t => {
  const dir = mkdtempSync(join(tmpdir(), "handover-append-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const log = join(dir, "h/handovers.jsonl");
  appendBatch(log, '{"a":1}\n');
  const short = { ...realFs, writeSync: (fd, buf) => realFs.writeSync(fd, buf, 0, 4) };
  assert.throws(() => appendBatch(log, '{"b":2}\n{"c":3}\n', short), /short write: 4 of 16 bytes/);
  assert.equal(readFileSync(log, "utf8"), '{"a":1}\n');
  appendBatch(log, '{"d":4}\n');
  assert.equal(readFileSync(log, "utf8"), '{"a":1}\n{"d":4}\n');
});

// ── Rust style: a range that cannot change the style verdict needs no Rust toolchain ──────────
// A thin client has no cargo. The stub below stands in for it (exit 127, as `command not found`);
// with STUB_STYLE_CARGO=1 the rust-style.sh stub builds first, as the real wrapper does.
const NOT_RUN = /\[pre-push\] rust-style: no input of the style check changed in ([0-9a-f]{12})\.\.([0-9a-f]{12}); not run/;
const NO_CARGO = { STUB_STYLE_CARGO: "1" };
function thinClient(fx) {
  exe(join(fx.dir, "bin/cargo"), "#!/usr/bin/env bash\necho 'cargo: not installed on this device' >&2\nexit 127\n");
}
function assertStyleNeeded(r, touched) {
  assert.equal(r.status, 1, r.out);
  assert.doesNotMatch(r.out, NOT_RUN, "a range touching a style input is never skipped");
  assert.match(r.out, /RED rust-style: this device cannot run the Rust style check that refs\/heads\/self\/a needs \(no cargo\)/);
  assert.ok(r.out.includes(`(it changes ${touched})`), `the cure line names the input\n${r.out}`);
  assert.match(r.out, /Cure: push it from a device with the pinned Rust toolchain\. A hand-over from here is refused too/);
  assert.match(r.out, /REFUSED: red before the gates/);
  assert.equal(r.records.length, 0, "no hand-over is recorded over a style check nobody ran");
}

test("a docs-only range on a device without cargo: style not applicable, the hand-over passes", t => {
  const fx = fixture(t);
  thinClient(fx);
  const head = commitOn(fx, { "genesis/docs/notes.md": "one line\n" }, "docs");
  const r = push(fx, [line("refs/heads/a", head, "refs/heads/self/a")], NO_CARGO);
  assertHandOver(r);
  const [, base, tip] = r.out.match(NOT_RUN) || [];
  assert.equal(base, fx.base.slice(0, 12), "the range is read from merge-base(origin/dev, head)");
  assert.equal(tip, head.slice(0, 12));
  assert.doesNotMatch(r.log, /^STYLE /m, "the checker is neither built nor run");
  assert.doesNotMatch(r.out, /RED rust-style/);
  assert.equal(r.records.length, 1, r.out);
});

test("a range touching one .rs file on a device without cargo is red and names the cure", t => {
  const fx = fixture(t);
  thinClient(fx);
  const head = commitOn(fx, { "crates/x/src/lib.rs": "pub fn f() {}\n" }, "rust");
  assertStyleNeeded(push(fx, [line("refs/heads/a", head, "refs/heads/self/a")], NO_CARGO), "crates/x/src/lib.rs");
});

test("a range touching one .rs file on a device with cargo runs the check as before", t => {
  const fx = fixture(t);
  const head = commitOn(fx, { "crates/x/src/lib.rs": "pub fn f() {}\n" }, "rust");
  const r = push(fx, [line("refs/heads/a", head, "refs/heads/self/a")]);
  assertHandOver(r);
  assert.ok(r.log.includes(`STYLE --base origin/dev --head ${head}`), r.log);
  assert.doesNotMatch(r.out, NOT_RUN);
});

for (const [name, files, touched] of [
  ["an .epr-meta manifest binding rust-style-strict@1",
    { "crates/x/.epr-meta/manifest.md": "---\nepr-meta-version: 1\nid: strict\ncovers: subtree\nrules:\n  - id: strict\n    policy: rust-style-strict@1\n---\n" },
    "crates/x/.epr-meta/manifest.md"],
  ["the checker's own source (a non-.rs file of it)",
    { "genesis/orchestrator/rust-style/Cargo.lock": "# changed\n" }, "genesis/orchestrator/rust-style/Cargo.lock"],
  ["the checker's build environment", { "genesis/orchestrator/rust-style-build-env.sh": "# changed\n" },
    "genesis/orchestrator/rust-style-build-env.sh"],
  ["a Cargo manifest", { "crates/x/Cargo.toml": "[package]\nname = \"x\"\n" }, "crates/x/Cargo.toml"],
  ["a legacy extensionless .cargo/config the build environment refuses", { ".cargo/config": "[build]\n" }, ".cargo/config"],
]) {
  test(`a range touching only ${name} is an input, never skipped`, t => {
    const fx = fixture(t);
    thinClient(fx);
    const head = commitOn(fx, files, "input");
    assertStyleNeeded(push(fx, [line("refs/heads/a", head, "refs/heads/self/a")], NO_CARGO), touched);
  });
}

test("a discipline object the style policy names is an input; an unnamed document beside it is not", t => {
  const fx = fixture(t);
  thinClient(fx);
  const policy = { version: 1, rules: ["RS-GOV-04"], external_roots: [], discipline: {
    policy: { cid: "x", path: ".epr-meta/rust-style/objects/p.json" },
    guide: { cid: "y", path: "genesis/docs/rust-style-guide.md" },
    method: { cid: "z", path: ".epr-meta/rust-style/objects/m.json" },
  } };
  const landed = commitOn(fx, {
    ".epr-meta/rust-style-policy.json": JSON.stringify(policy),
    "genesis/docs/rust-style-guide.md": "guide\n",
  }, "policy");
  fx.git("update-ref", "refs/remotes/origin/dev", landed);
  const other = commitOn(fx, { "genesis/docs/other.md": "other\n" }, "unrelated doc");
  const quiet = push(fx, [line("refs/heads/a", other, "refs/heads/self/a")], NO_CARGO);
  assertHandOver(quiet);
  assert.match(quiet.out, NOT_RUN);
  rmSync(join(fx.dir, RECORD), { force: true });
  const guide = commitOn(fx, { "genesis/docs/rust-style-guide.md": "guide, revised\n" }, "guide");
  assertStyleNeeded(push(fx, [line("refs/heads/a", guide, "refs/heads/self/a")], NO_CARGO), "genesis/docs/rust-style-guide.md");
});

test("an .rs change made and reverted inside the pushed range still runs the check (per commit, not net)", t => {
  const fx = fixture(t);
  thinClient(fx);
  commitOn(fx, { "crates/x/src/lib.rs": "#[allow(dead_code)] fn f() {}\n" }, "add");
  const head = commitOn(fx, {}, "revert", ["crates/x/src/lib.rs"]);
  assert.equal(fx.git("diff", "--name-only", fx.base, head), "src/x.txt\nsrc/y.txt", "the net diff touches no input");
  assertStyleNeeded(push(fx, [line("refs/heads/a", head, "refs/heads/self/a")], NO_CARGO), "crates/x/src/lib.rs");
});

test("deleting an .rs file is an input", t => {
  const fx = fixture(t);
  thinClient(fx);
  const landed = commitOn(fx, { "crates/x/src/old.rs": "pub fn old() {}\n" }, "old");
  fx.git("update-ref", "refs/remotes/origin/dev", landed);
  const head = commitOn(fx, {}, "delete", ["crates/x/src/old.rs"]);
  assertStyleNeeded(push(fx, [line("refs/heads/a", head, "refs/heads/self/a")], NO_CARGO), "crates/x/src/old.rs");
});

// A new branch is read as origin/dev..tip (the hook's ruling); with no origin/dev there is no base.
test("a new branch with origin/dev missing is never read as untouched", t => {
  const fx = fixture(t);
  thinClient(fx);
  fx.git("update-ref", "-d", "refs/remotes/origin/dev");
  const r = push(fx, [line("refs/heads/a", fx.one, "refs/heads/self/a")], NO_CARGO);
  assert.equal(r.status, 1, r.out);
  assert.doesNotMatch(r.out, NOT_RUN);
  assert.match(r.out, /\(its range could not be read\)/);
});

test("the readings tell a not-applicable style leg from a run that passed", t => {
  const rowOf = r => readFileSync(join(r.fx.dir, "reading-rows.jsonl"), "utf8").trim().split("\n")
    .map(row => JSON.parse(row)).find(row => row.step === "leg:rust-style");
  const fx = fixture(t);
  const env = { GATE_READING: "on", EPR_BIN: fx.readingEpr };
  const quiet = push(fx, [line("refs/heads/a", fx.one, "refs/heads/self/a")], env);
  assert.equal(quiet.status, 0, quiet.out);
  const skipped = rowOf({ fx });
  assert.deepEqual([skipped.outcome, skipped.status, skipped.synthesized], ["skipped", "none", false],
    "written by the leg itself, not the exit-time fill");
  const head = commitOn(fx, { "crates/x/src/lib.rs": "pub fn f() {}\n" }, "rust");
  const ran = push(fx, [line("refs/heads/a", head, "refs/heads/self/a")], env);
  assert.equal(ran.status, 0, ran.out);
  const green = rowOf({ fx });
  assert.deepEqual([green.outcome, green.status, green.synthesized], ["ran", "green", false]);
});

test("the hook's style input set holds every path predicate of the checker's relevant()", () => {
  const source = readFileSync(join(ROOT, "genesis/orchestrator/rust-style/src/repository.rs"), "utf8");
  const body = source.slice(source.indexOf("fn relevant("), source.indexOf("\n}\n", source.indexOf("fn relevant(")));
  const literals = [...body.matchAll(/"([^"]+)"/g)].map(m => m[1]);
  assert.ok(literals.length >= 9 && body.includes("p == POLICY"), `relevant() is where this test expects it\n${body}`);
  const policy = source.match(/pub const POLICY: &str = "([^"]+)"/)[1];
  const hook = readFileSync(HOOK_PATH, "utf8");
  const fn = hook.slice(hook.indexOf("rust_style_input() {"), hook.indexOf("\n}\n", hook.indexOf("rust_style_input() {")));
  for (const literal of [...literals, policy]) {
    assert.ok(fn.includes(literal), `rust_style_input() omits ${literal}, which the checker reads`);
  }
});

test("the hook's style input set holds every build-context path rust-style-build-env.sh's walk refuses", () => {
  const env = readFileSync(join(ROOT, "genesis/orchestrator/rust-style-build-env.sh"), "utf8");
  const checker = env.match(/style_checker_root="\$style_repo_root\/([^"]+)"/)?.[1];
  const names = [...env.matchAll(/="\$style_config_root\/([^"]+)"/g)].map(m => m[1]);
  assert.ok(checker && names.includes(".cargo/config") && names.includes(".cargo/config.toml"),
    `the walk is where this test expects it: checker=${checker} names=${names}`);
  const dirs = [];
  for (let d = checker; d && d !== "."; d = d.includes("/") ? d.slice(0, d.lastIndexOf("/")) : "") dirs.push(d);
  dirs.push("");
  const paths = dirs.flatMap(d => names.map(n => (d ? `${d}/${n}` : n)));
  const hook = readFileSync(HOOK_PATH, "utf8");
  const start = hook.indexOf("rust_style_input() {");
  const fn = hook.slice(start, hook.indexOf("\n}\n", start) + 3);
  const script = `RUST_STYLE_DISCIPLINE=""\n${fn}\nfor p in "$@"; do rust_style_input "$p" || echo "MISSING $p"; done\n`;
  const r = spawnSync("bash", ["-c", script, "_", ...paths], { encoding: "utf8" });
  assert.equal(r.status, 0, r.stderr);
  assert.equal(r.stdout, "", `rust_style_input() omits paths the build environment refuses:\n${r.stdout}`);
  // And it is not vacuous: an unrelated path is not an input.
  const neg = spawnSync("bash", ["-c", script, "_", "genesis/docs/notes.md"], { encoding: "utf8" });
  assert.equal(neg.stdout, "MISSING genesis/docs/notes.md\n");
});
