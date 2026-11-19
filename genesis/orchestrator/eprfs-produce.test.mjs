// The eprfs gate's producer leg (backlog steward-device-arrival row 1, native-throughput DoD 1):
// after the gate builds and ingests epr and bin-launch, `_eprfs-ingest-around` offers that release
// to the person's other devices with `epr artifact describe` -> `key` -> `put --release
// --release-bin --cross` of the gate's own bytes. A put never fails the gate. This runs the ACTUAL
// root justfile recipe against a stub `cargo` and a stub `epr` that record their argv, inside a
// scratch repository.
import { test } from "node:test";
import assert from "node:assert/strict";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { createRecognizer } from "./gate-failure-lines.mjs";
import { classifyFailure } from "./gate-cycle.mjs";

const ROOT = fileURLToPath(new URL("../../", import.meta.url));
const RELEASE = "bafyreistubreleasestubreleasestubreleasestubrelease";
const OUTPUTS = { epr: "bafkrstubeproutput", "bin-launch": "bafkrstublaunchoutput" };
const SOURCES = "elohim/eprfs/bin-build-view.json#bin-build";

// A stub epr, also the bytes the stub cargo "builds" into debug/epr. Logs its argv as one JSON
// line. STUB_DIRTY reads the source closure dirty; STUB_PUT_REFUSE refuses every put.
const STUB_EPR = `#!/usr/bin/env node
const { appendFileSync, writeFileSync } = require('node:fs');
const args = process.argv.slice(2);
appendFileSync(process.env.STUB_LOG, JSON.stringify(['epr', ...args]) + '\\n');
const value = flag => args[args.indexOf(flag) + 1];
if (args[0] === 'bin' && args[3] === 'source-state') {
  console.log(JSON.stringify({ clean: !process.env.STUB_DIRTY }));
} else if (args[0] === 'bin' && args[3] === 'ingest') {
  console.log(JSON.stringify({ cid: ${JSON.stringify(RELEASE)}, bins: ${JSON.stringify(OUTPUTS)} }));
} else if (args[0] === 'artifact' && args[1] === 'describe') {
  writeFileSync(value('--out'), '{}\\n');
  console.error('build-inputs.build-scripts: pending, not enforced');
  console.log('bafyreistubmethod');
} else if (args[0] === 'artifact' && args[1] === 'key') {
  console.log('bafkrstubinput');
} else if (args[0] === 'artifact' && args[1] === 'put') {
  console.error('build-inputs.build-scripts: pending, not enforced');
  if (process.env.STUB_PUT_REFUSE) {
    console.log('unrecorded: missing attribution: this workspace has no device key');
    process.exit(3);
  }
  console.log(JSON.stringify({ output: ${JSON.stringify(OUTPUTS)}[value('--release-bin')], crosses: true }));
}
`;

// A stub cargo: logs argv; `build` writes each named --bin (the stub epr for epr) and its dep-info.
const STUB_CARGO = `#!/usr/bin/env node
const { appendFileSync, copyFileSync, chmodSync, mkdirSync, writeFileSync } = require('node:fs');
const args = process.argv.slice(2);
appendFileSync(process.env.STUB_LOG, JSON.stringify(['cargo', ...args]) + '\\n');
if (process.env.STUB_CARGO_FAIL) process.exit(101);
if (args[0] === 'build') {
  const debug = process.env.CARGO_TARGET_DIR + '/debug';
  mkdirSync(debug, { recursive: true });
  args.forEach((arg, i) => {
    if (arg !== '--bin') return;
    const bin = args[i + 1];
    if (bin === 'epr') copyFileSync(process.env.STUB_EPR, debug + '/epr');
    else writeFileSync(debug + '/' + bin, 'launcher');
    chmodSync(debug + '/' + bin, 0o755);
    writeFileSync(debug + '/' + bin + '.d', debug + '/' + bin + ': src/main.rs\\n');
  });
}
`;

// A stub C-baseline check (the real one is genesis/orchestrator/c-baseline.mjs, row 29.2): logs its
// argv; STUB_CHECK_FAIL prints two findings after a blank line and exits 1.
const FINDING = "CFLAGS_x86_64_unknown_linux_gnu is not set: cc builds ring's C at the compiler default, not -march=x86-64";
const STUB_CHECK = `import { appendFileSync } from 'node:fs';
appendFileSync(process.env.STUB_LOG, JSON.stringify(['c-baseline', ...process.argv.slice(2)]) + '\\n');
if (process.env.STUB_CHECK_FAIL) {
  console.log('');
  console.log(${JSON.stringify(FINDING)});
  console.error('CXXFLAGS_x86_64_unknown_linux_gnu is not set');
  process.exit(1);
}
console.log('c-baseline: x86_64-unknown-linux-gnu builds C at -march=x86-64');
`;

function fixture(t) {
  const dir = mkdtempSync(join(tmpdir(), "eprfs-produce-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const repo = join(dir, "repo");
  const stubs = join(dir, "stubs");
  const target = join(dir, "slot");
  mkdirSync(join(repo, "elohim/eprfs"), { recursive: true });
  mkdirSync(stubs);
  writeFileSync(join(repo, "elohim/eprfs/bin-build-view.json"), '{"steps":{"bin-build":{}}}\n');
  const git = (...args) => spawnSync("git", ["-C", repo, ...args], { encoding: "utf8" });
  git("init", "-q");
  git("-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false", "add", ".");
  git("-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false", "commit", "-qm", "fixture", "--no-verify");
  const head = git("rev-parse", "HEAD").stdout.trim();
  assert.match(head, /^[0-9a-f]{40}$/);
  for (const [name, body] of [["epr", STUB_EPR], ["cargo", STUB_CARGO], ["c-baseline.mjs", STUB_CHECK]]) {
    writeFileSync(join(stubs, name), body);
    chmodSync(join(stubs, name), 0o755);
  }
  return { dir, repo, stubs, target, head, log: join(dir, "calls.log") };
}

function run(f, mode, extra = {}) {
  const env = {};
  // A controlled build environment: nothing of the caller's CARGO_*/RUST* leaks in.
  for (const [k, v] of Object.entries(process.env)) {
    if (!/^(CARGO_|RUST|EPRFS_|GATE_|EPR_)/.test(k)) env[k] = v;
  }
  Object.assign(env, {
    PATH: `${f.stubs}:${process.env.PATH}`,
    CARGO_TARGET_DIR: f.target,
    CARGO_HOME: join(f.dir, "cargo-home"),
    CARGO_TARGET_POOL_ROOT: join(f.dir, "pool"),
    RUSTUP_TOOLCHAIN: "1.100.0",
    RUSTFLAGS: "",
    STUB_LOG: f.log,
    STUB_EPR: join(f.stubs, "epr"),
    EPRFS_C_BASELINE_CHECK: join(f.stubs, "c-baseline.mjs"),
    ...extra,
  });
  const r = spawnSync("just", ["--justfile", join(ROOT, "justfile"), "--working-directory", f.repo,
    "_eprfs-ingest-around", mode], { env, encoding: "utf8", timeout: 60000 });
  const calls = existsSync(f.log)
    ? readFileSync(f.log, "utf8").trim().split("\n").filter(Boolean).map((l) => JSON.parse(l)) : [];
  return { status: r.status, stdout: r.stdout, stderr: r.stderr, calls };
}

const verb = (calls, ...prefix) => calls.findIndex((c) => prefix.every((p, i) => c[i] === p));
const flag = (call, name) => call[call.indexOf(name) + 1];
const skipped = (out) => out.split("\n").filter((l) => l.startsWith("[gate] epr producer put"));

function assertOffered(f, r) {
  assert.equal(r.status, 0, r.stderr);
  const ingest = verb(r.calls, "epr", "bin", "--tool", "epr", "ingest");
  assert.ok(ingest >= 0, "the gate ingests first");
  for (const bin of ["epr", "bin-launch"]) {
    const describe = r.calls.findIndex((c) => c[1] === "artifact" && c[2] === "describe" && flag(c, "--bin") === bin);
    const key = r.calls.findIndex((c) => c[1] === "artifact" && c[2] === "key"
      && flag(c, "--method") === join(f.target, `producer/method-${bin}.json`));
    const put = r.calls.findIndex((c) => c[1] === "artifact" && c[2] === "put" && flag(c, "--release-bin") === bin);
    assert.ok(ingest < describe && describe < key && key < put, `${bin}: describe -> key -> put after ingest:\n${r.calls.map((c) => c.join(" ")).join("\n")}`);
    const d = r.calls[describe];
    assert.equal(flag(d, "--sources"), SOURCES);
    assert.equal(flag(d, "--project"), "eprfs");
    assert.equal(flag(d, "--rustflags"), "");
    assert.equal(flag(d, "--crate"), bin === "epr" ? "elohim/eprfs/epr-cli" : "elohim/eprfs/device-runtime");
    // The environment the build ran under is recorded as it is, with no scrub and no second build.
    const recorded = d.filter((_, i) => d[i - 1] === "--env");
    assert.ok(recorded.includes(`CARGO_TARGET_POOL_ROOT=${join(f.dir, "pool")}`), recorded.join(" "));
    assert.ok(recorded.includes("RUSTUP_TOOLCHAIN=1.100.0"), recorded.join(" "));
    assert.ok(!recorded.some((e) => /^(CARGO_HOME|CARGO_TARGET_DIR|RUSTFLAGS)=/.test(e)), recorded.join(" "));
    // `put` refuses a declared target until L2a (K6): the C level is recorded as --env only.
    assert.ok(!d.some((a) => a.startsWith("--target-")), d.join(" "));
    const check = verb(r.calls, "c-baseline", "--triple", "x86_64-unknown-linux-gnu");
    assert.ok(check >= 0 && check < describe, "the C baseline is checked before any describe");
    const p = r.calls[put];
    assert.equal(flag(p, "--release"), RELEASE);
    assert.ok(p.includes("--cross"), "the Produce crosses to siblings");
    assert.equal(flag(p, "--bytes"), join(f.target, `debug/${bin}`), "the gate's own bytes");
    assert.equal(flag(p, "--commit"), f.head);
    assert.equal(flag(p, "--input"), "bafkrstubinput");
  }
  assert.deepEqual(skipped(r.stdout), [
    `[gate] epr producer put: release ${RELEASE} at ${f.head.slice(0, 12)}, epr ${OUTPUTS.epr}, bin-launch ${OUTPUTS["bin-launch"]} (crosses)`,
  ]);
  const builds = r.calls.filter((c) => c[0] === "cargo" && c[1] === "build");
  assert.equal(builds.length, 1, "one build; the producer adds none");
}

test("build_mode_offers_the_gate_bytes_of_both_bins_with_release_and_cross", (t) => {
  const f = fixture(t);
  assertOffered(f, run(f, "build"));
});

test("test_mode_builds_both_bins_after_the_tests_and_offers_them", (t) => {
  const f = fixture(t);
  const r = run(f, "test");
  assert.ok(verb(r.calls, "cargo", "test") >= 0 && verb(r.calls, "cargo", "test") < verb(r.calls, "cargo", "build"));
  assertOffered(f, r);
});

test("a_dirty_source_closure_skips_the_put_with_one_line_and_stays_green", (t) => {
  const f = fixture(t);
  const r = run(f, "build", { STUB_DIRTY: "1" });
  assert.equal(r.status, 0, r.stderr);
  assert.deepEqual(skipped(r.stdout), [
    "[gate] epr producer put skipped (epr's source closure was not clean across the build; gate remains green)",
  ]);
  assert.ok(!r.calls.some((c) => c[1] === "artifact"), "nothing is described, keyed or put");
});

test("a_put_refusal_skips_with_its_reason_and_stays_green", (t) => {
  const f = fixture(t);
  const r = run(f, "build", { STUB_PUT_REFUSE: "1" });
  assert.equal(r.status, 0, r.stderr);
  assert.deepEqual(skipped(r.stdout), [
    "[gate] epr producer put skipped (put epr refused: unrecorded: missing attribution: this workspace has no device key; gate remains green)",
  ]);
});

test("a_head_other_than_the_tested_one_is_not_offered", (t) => {
  const f = fixture(t);
  const r = run(f, "build", { EPRFS_TESTED_HEAD: "0".repeat(40) });
  assert.equal(r.status, 0, r.stderr);
  assert.deepEqual(skipped(r.stdout), [
    `[gate] epr producer put skipped (HEAD ${f.head.slice(0, 12)} is not the tested head 000000000000; gate remains green)`,
  ]);
  assert.ok(!r.calls.some((c) => c[1] === "artifact"));
});

test("negative_control_a_failed_build_fails_the_gate_and_offers_nothing", (t) => {
  const f = fixture(t);
  const r = run(f, "build", { STUB_CARGO_FAIL: "1" });
  assert.equal(r.status, 101, "the build's failure is the gate's verdict");
  assert.ok(!r.calls.some((c) => c[0] === "epr" && (c[1] === "artifact" || c[3] === "ingest")));
  assert.deepEqual(skipped(r.stdout), []);
});

// Row 29.2: the C a gate builds is held at the Rust target's baseline (-march=x86-64), because a
// v3-default gcc put AVX2 into ring's C and a v2 sibling died with SIGILL running a drawn epr.
const C_LEVEL = {
  CFLAGS_x86_64_unknown_linux_gnu: "-march=x86-64",
  CXXFLAGS_x86_64_unknown_linux_gnu: "-march=x86-64",
};

// The withheld line reaches the agent as a live event, but its class names no failure of the gate.
function recognize(stdout, exitCode) {
  const events = [];
  const rec = createRecognizer({ project: "eprfs", emit: (e) => events.push(e) });
  for (const line of stdout.split("\n")) rec.push(line);
  rec.end(exitCode);
  return { failures: events.filter((e) => e.kind === "failure"), signals: rec.signals() };
}

test("a_failing_c_baseline_check_withholds_the_offer_with_its_first_finding_and_never_describes", (t) => {
  const f = fixture(t);
  const r = run(f, "build", { ...C_LEVEL, STUB_CHECK_FAIL: "1" });
  assert.equal(r.status, 0, r.stderr);
  assert.deepEqual(skipped(r.stdout), [`[gate] epr producer put withheld (${FINDING}; gate remains green)`]);
  assert.ok(verb(r.calls, "c-baseline", "--triple", "x86_64-unknown-linux-gnu") >= 0, "the check ran");
  assert.ok(!r.calls.some((c) => c[1] === "artifact"), "nothing is described, keyed or put");
  const heard = recognize(r.stdout, r.status);
  assert.deepEqual(heard.failures.map((e) => e.failure.class), ["env"], "the agent hears the withheld offer");
  assert.equal(classifyFailure(r.status, heard.signals), undefined, "and the gate stays green");
  assert.equal(heard.signals.assertionFailed || heard.signals.compilerError, false);
});

test("a_missing_c_baseline_check_withholds_the_offer", (t) => {
  const f = fixture(t);
  const absent = join(f.dir, "no-such-c-baseline.mjs");
  const r = run(f, "build", { ...C_LEVEL, EPRFS_C_BASELINE_CHECK: absent });
  assert.equal(r.status, 0, r.stderr);
  assert.deepEqual(skipped(r.stdout), [
    `[gate] epr producer put withheld (the C baseline check ${absent} is not there; gate remains green)`,
  ]);
  assert.ok(!r.calls.some((c) => c[1] === "artifact"));
});

test("a_passing_c_baseline_check_describes_with_the_declared_c_flags_only", (t) => {
  const f = fixture(t);
  // The rest of the C family is the checker's to refuse; the producer never records it.
  const r = run(f, "build", { ...C_LEVEL, CFLAGS: "-O2", CXXFLAGS: "-O2", CC: "gcc", TARGET_CFLAGS: "-mavx2" });
  assertOffered(f, r);
  for (const d of r.calls.filter((c) => c[1] === "artifact" && c[2] === "describe")) {
    const recorded = d.filter((_, i) => d[i - 1] === "--env");
    for (const [k, v] of Object.entries(C_LEVEL)) assert.ok(recorded.includes(`${k}=${v}`), recorded.join(" "));
    const cFamily = recorded.filter((e) => !/^(CARGO_|RUST)/.test(e));
    assert.deepEqual(cFamily.sort(), Object.entries(C_LEVEL).map(([k, v]) => `${k}=${v}`).sort());
  }
  assert.deepEqual(recognize(r.stdout, r.status).failures, [], "an offer that crosses raises nothing");
});

test("unset_c_flags_are_not_recorded", (t) => {
  const f = fixture(t);
  const r = run(f, "build");
  assertOffered(f, r);
  const d = r.calls.find((c) => c[1] === "artifact" && c[2] === "describe");
  assert.ok(!d.some((a) => /^C(XX)?FLAGS/.test(a)), d.join(" "));
});

test("the_producer_is_off_when_asked", (t) => {
  const f = fixture(t);
  const r = run(f, "build", { EPRFS_PRODUCER: "off" });
  assert.equal(r.status, 0, r.stderr);
  assert.deepEqual(skipped(r.stdout), ["[gate] epr producer put skipped (EPRFS_PRODUCER=off; gate remains green)"]);
  assert.ok(!r.calls.some((c) => c[1] === "artifact"));
});
