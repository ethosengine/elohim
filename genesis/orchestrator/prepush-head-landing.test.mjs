// The pre-push hook's household-head landing (spec 2026-10-23-household-follows-latest-design.md
// §10, backlog prepush-head-landing-never-ingests): a green push to dev that gated eprfs lands the
// pushed commit's epr build with no hand step. When pre-push accepted an eprfs gate receipt, no
// build ran at the pushed SHA, so the block builds and ingests it through `_eprfs-deliver` first.
// Like prepush-berth.test.mjs, this runs the ACTUAL hook's block (sliced between its markers), here
// against a stub `epr` store and a stub gate-runner inside a scratch repository.
import { test } from "node:test";
import assert from "node:assert/strict";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("../../", import.meta.url));
const CID = "bafkreiexampleexampleexampleexampleexampleexampleexample";

function hookBlock() {
  const hook = readFileSync(join(ROOT, ".husky/pre-push.bash"), "utf8");
  const start = hook.indexOf("# ── Household head: land the pushed epr");
  const end = hook.indexOf("# ── Household head: end", start);
  assert.ok(start >= 0 && end > start, "exercise the actual hook's landing block");
  return hook.slice(start, end);
}

// A stub epr: logs `<args>` per call. `candidate` (a file) holds the CID ingested at the pushed
// commit; `status --commit` answers from it, `land` succeeds, `resolve --current` names this stub.
const STUB_EPR = `#!/usr/bin/env bash
here="$(cd "$(dirname "$0")" && pwd)"
echo "$*" >> "$here/calls.log"
shift 3 # bin --tool epr
case "$1" in
  status)
    if [ "$2" = --commit ]; then
      cid="$(cat "$here/candidate" 2>/dev/null)"
      if [ -n "$cid" ]; then printf '{"candidates":[{"cid":"%s"}]}\\n' "$cid"; else echo '{"candidates":[]}'; fi
    else
      printf '{"store":"%s/store/epr"}\\n' "$here"
    fi ;;
  land) exit 0 ;;
  resolve) echo "$here/epr" ;;
  install-launcher) exit 0 ;;
esac
`;

// A stub gate-runner: `--print` names the slot; `--recipe _eprfs-deliver` stands for the gate's
// build-and-ingest, ingesting `deliver` (a file) when present and building a new launcher.
const STUB_GATE_RUNNER = `import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync, chmodSync } from 'node:fs';
const root = process.cwd();
const args = process.argv.slice(2);
appendFileSync(root + '/bin/calls.log', 'gate-runner ' + args.join(' ') + '\\n');
if (args.includes('--print')) {
  console.log(JSON.stringify({ name: 'eprfs', resolvedTargetDir: root + '/slot' }));
} else if (args.includes('--recipe')) {
  if (process.env.GATE_RUN_KIND !== 'pre-push') process.exit(3);
  if (existsSync(root + '/bin/deliver')) writeFileSync(root + '/bin/candidate', readFileSync(root + '/bin/deliver'));
  mkdirSync(root + '/slot/debug', { recursive: true });
  writeFileSync(root + '/slot/debug/bin-launch', 'new launcher');
  chmodSync(root + '/slot/debug/bin-launch', 0o755);
}
`;

const cleanEnv = Object.fromEntries(
  Object.entries(process.env).filter(
    ([key]) => !key.startsWith("GIT_") || key.startsWith("GIT_CONFIG_"),
  ),
);

/** A scratch checkout at the pushed commit, with a stub store and gate-runner. */
function fixture(t, { candidate = "", deliver = CID, slotLauncher = null, installed = "old launcher" } = {}) {
  const dir = mkdtempSync(join(tmpdir(), "prepush-head-landing-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const git = (...args) => {
    const r = spawnSync("git", ["-C", dir, ...args], { encoding: "utf8", env: cleanEnv });
    assert.equal(r.status, 0, r.stderr);
    return r.stdout.trim();
  };
  git("init", "--quiet");
  git("-c", "user.name=Fixture", "-c", "user.email=fixture@example.test", "commit", "--quiet", "--allow-empty", "-m", "pushed");
  const head = git("rev-parse", "HEAD");
  const bin = join(dir, "bin");
  mkdirSync(join(bin, "store/bin"), { recursive: true });
  writeFileSync(join(bin, "epr"), STUB_EPR);
  chmodSync(join(bin, "epr"), 0o755);
  writeFileSync(join(bin, "store/bin/bin-launch"), installed);
  if (candidate) writeFileSync(join(bin, "candidate"), candidate);
  if (deliver) writeFileSync(join(bin, "deliver"), deliver);
  mkdirSync(join(dir, "genesis/orchestrator"), { recursive: true });
  writeFileSync(join(dir, "genesis/orchestrator/gate-runner.mjs"), STUB_GATE_RUNNER);
  if (slotLauncher !== null) {
    mkdirSync(join(dir, "slot/debug"), { recursive: true });
    writeFileSync(join(dir, "slot/debug/bin-launch"), slotLauncher);
    chmodSync(join(dir, "slot/debug/bin-launch"), 0o755);
  }
  return { dir, head, bin };
}

function runBlock(fx, { sha = fx.head, projects = "orchestrator eprfs rust-style" } = {}) {
  const preamble = [
    `DEV_LOCAL_SHA='${sha}'`,
    `PROJECTS='${projects}'`,
    `HOOK_WORK_TREE='${fx.dir}'`,
    "",
  ].join("\n");
  const result = spawnSync("bash", ["-c", `${preamble}${hookBlock()}`], {
    cwd: fx.dir,
    timeout: 15000,
    encoding: "utf8",
    env: { ...cleanEnv, EPR_BIN: join(fx.bin, "epr") },
  });
  let calls = [];
  try {
    calls = readFileSync(join(fx.bin, "calls.log"), "utf8").trim().split("\n").filter(Boolean);
  } catch {
    calls = [];
  }
  return { ...result, out: `${result.stdout}${result.stderr}`, calls };
}

test("receipt_attested_push_builds_ingests_and_lands_the_pushed_commit", (t) => {
  const fx = fixture(t);
  const r = runBlock(fx);
  assert.equal(r.status, 0, r.out);
  assert.ok(r.out.includes(`[pre-push] epr household head landed: ${CID}`), r.out);
  assert.ok(!r.out.includes("not landed"), r.out);
  const land = r.calls.find((c) => c.startsWith("bin --tool epr land"));
  assert.equal(land, `bin --tool epr land ${CID} --commit ${fx.head} --repo ${fx.dir}`);
  assert.ok(
    r.calls.indexOf("gate-runner --target eprfs --recipe _eprfs-deliver") < r.calls.indexOf(land),
    `the pushed commit is built and ingested before it lands:\n${r.calls.join("\n")}`,
  );
  // The landing build carried a new launcher, so the landed epr installs it.
  assert.ok(
    r.calls.includes(`bin --tool epr install-launcher --launcher ${fx.dir}/slot/debug/bin-launch --repo ${fx.dir}`),
    r.calls.join("\n"),
  );
  assert.ok(r.out.includes(`[pre-push] epr launcher installed from ${fx.dir}/slot/debug/bin-launch`), r.out);
});

test("a_gate_that_ran_lands_its_own_ingest_without_rebuilding", (t) => {
  const fx = fixture(t, { candidate: CID, deliver: "", slotLauncher: "same", installed: "same" });
  const r = runBlock(fx);
  assert.equal(r.status, 0, r.out);
  assert.ok(r.out.includes(`[pre-push] epr household head landed: ${CID}`), r.out);
  assert.ok(!r.calls.some((c) => c.includes("--recipe")), "the gate's own ingest is landed as is");
  assert.ok(!r.calls.some((c) => c.includes("install-launcher")), "an unchanged launcher is left alone");
});

test("a_delivery_that_ingests_nothing_says_so_and_never_lands", (t) => {
  const fx = fixture(t, { deliver: "" });
  const r = runBlock(fx);
  assert.equal(r.status, 0, "the landing is advisory to the push");
  assert.ok(
    r.out.includes("[pre-push] epr household head not landed: no ingested epr candidate names the SHA targeting dev"),
    r.out,
  );
  assert.ok(!r.calls.some((c) => c.startsWith("bin --tool epr land")), r.calls.join("\n"));
});

test("pushes_that_do_not_land_on_dev_touch_nothing", (t) => {
  const fx = fixture(t);
  const notDev = runBlock(fx, { sha: "" });
  assert.ok(notDev.out.includes("not landed: push does not target refs/heads/dev"), notDev.out);
  assert.deepEqual(notDev.calls, []);

  const noEprfs = runBlock(fx, { projects: "orchestrator" });
  assert.ok(noEprfs.out.includes("not landed: eprfs was not among the green gates"), noEprfs.out);
  assert.deepEqual(noEprfs.calls, []);
  assert.ok(!existsSync(join(fx.dir, "slot")), "no build ran");
});
