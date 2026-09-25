// D2 — measure.sh syntax + dry-run behavior for the two defects found during the live rung
// H run (2026-09-25): the feature-path grammar mismatch with build-stage-task.mjs's --feature
// (which resolves against genesis/a2o, the `just test mesh` grammar) and the honest resource
// bound (the household stand-in's real cgroup ceiling, not the stage builder's understating
// 2000m/2GiB defaults). Kept as a plain node:test spawning bash -n / MEASURE_DRY_RUN=1 runs —
// measure.sh has no other test coverage, and the a2o habit checks the live chain, not the
// script's own argument handling.
import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync, mkdtempSync, writeFileSync, chmodSync, rmSync } from "node:fs";
import { dirname, resolve, join } from "node:path";
import { fileURLToPath } from "node:url";
import { tmpdir } from "node:os";

const HERE = dirname(fileURLToPath(import.meta.url));
const MEASURE_SH = resolve(HERE, "measure.sh");
const REPO_ROOT = resolve(HERE, "..", "..", "..");
const FEATURE_A2O_RELATIVE = "features/dataplane/accountable-correction.feature";
const FEATURE_GENESIS_A2O_PREFIXED = `genesis/a2o/${FEATURE_A2O_RELATIVE}`;
const FEATURE_REPO_ABSOLUTE = resolve(REPO_ROOT, "genesis/a2o", FEATURE_A2O_RELATIVE);

// Mirrors measure.sh's own derive_cpu_millis/derive_memory_bytes so the expectation tracks
// whatever THIS host's cgroup actually declares, rather than hardcoding a value that would
// only hold on the one pod the defect was found on.
function expectedCgroupBounds() {
  const cgroupLine = readFileSync("/proc/self/cgroup", "utf8")
    .split("\n")
    .find((line) => line.startsWith("0:"));
  const cgroupPath = cgroupLine.split(":")[2];
  const cpuMax = readFileSync(`/sys/fs/cgroup${cgroupPath}/cpu.max`, "utf8").trim().split(/\s+/);
  const [quota, period] = cpuMax;
  const cpuMillis =
    quota === "max"
      ? Number(execFileSync("nproc").toString().trim()) * 1000
      : Math.floor((Number(quota) * 1000) / Number(period));
  const memValue = readFileSync(`/sys/fs/cgroup${cgroupPath}/memory.max`, "utf8").trim();
  const memoryBytes =
    memValue === "max"
      ? Number(
          readFileSync("/proc/meminfo", "utf8")
            .split("\n")
            .find((line) => line.startsWith("MemTotal:"))
            .match(/(\d+)/)[1],
        ) * 1024
      : Number(memValue);
  return { cpuMillis, memoryBytes };
}

function runDry(args, extraEnv = {}) {
  return execFileSync("bash", [MEASURE_SH, ...args], {
    cwd: REPO_ROOT,
    env: { ...process.env, MEASURE_DRY_RUN: "1", ...extraEnv },
    encoding: "utf8",
  });
}

test("measure.sh parses cleanly", () => {
  execFileSync("bash", ["-n", MEASURE_SH]);
});

test("dry-run derives the task bound from this pod's real cgroup ceiling, not the builder's own understating defaults", () => {
  const { cpuMillis, memoryBytes } = expectedCgroupBounds();
  const output = runDry([FEATURE_GENESIS_A2O_PREFIXED]);
  assert.match(output, new RegExp(`--cpu-millis ${cpuMillis} --memory-bytes ${memoryBytes}\\b`));
  assert.match(
    output,
    new RegExp(`task bound: cpu-millis=${cpuMillis} \\(cgroup\\) memory-bytes=${memoryBytes} \\(cgroup\\)`),
  );
});

test("an explicit MEASURE_CPU_MILLIS/MEASURE_MEMORY_BYTES/MEASURE_TIMEOUT_SECONDS overrides the cgroup derivation and is labelled env", () => {
  const output = runDry([FEATURE_GENESIS_A2O_PREFIXED], {
    MEASURE_CPU_MILLIS: "4000",
    MEASURE_MEMORY_BYTES: "1000000",
    MEASURE_TIMEOUT_SECONDS: "900",
  });
  assert.match(output, /--cpu-millis 4000 --memory-bytes 1000000 --timeout-seconds 900\b/);
  assert.match(
    output,
    /task bound: cpu-millis=4000 \(env\) memory-bytes=1000000 \(env\) timeout-seconds=900 \(env\)/,
  );
});

test("genesis/a2o-prefixed, a2o-relative and repo-root-absolute feature paths all resolve to the same --feature line", () => {
  const expected = new RegExp(`--feature ${FEATURE_A2O_RELATIVE.replace(/[.]/g, "\\.")}\\b`);
  for (const form of [FEATURE_GENESIS_A2O_PREFIXED, FEATURE_A2O_RELATIVE, FEATURE_REPO_ABSOLUTE]) {
    const output = runDry([form]);
    assert.match(output, expected, `feature form '${form}' did not print the expected --feature line`);
  }
});

test("a nonexistent feature path refuses exit 2 before printing anything else", () => {
  assert.throws(
    () => runDry(["genesis/a2o/features/dataplane/does-not-exist-measure-test.feature"]),
    (error) => {
      assert.equal(error.status, 2);
      assert.match(error.stderr.toString(), /feature path does not exist/);
      // Refuses before the env block / chmod capacity lines print anything to stdout.
      assert.equal(error.stdout.toString(), "");
      return true;
    },
  );
});

// The guest capacity grant is more than the reports_dir + three runtime-config.toml
// (D2-follow-up, live rung H runs 2 & 3, 2026-09-25): the household `test mesh` lane's inner
// run REWRITES pre-existing, root-owned files under genesis/a2o/reports/ every pass. A
// world-writable reports_dir does not make an existing file inside it writable — opening an
// existing file for write needs the write bit on the FILE itself. MEASURE_REPORTS_DIR lets
// this test point the check at a disposable temp dir instead of the real repo's reports_dir.
function assertCapacityGrantCoversRewrittenFile(filename) {
  const tempReports = mkdtempSync(join(tmpdir(), "measure-reports-"));
  chmodSync(tempReports, 0o777);
  const blockedPath = join(tempReports, filename);
  writeFileSync(blockedPath, "// stand-in for a root-owned file the lane rewrites\n", {
    mode: 0o644,
  });
  const chmodLine = new RegExp(`chmod o\\+w ${blockedPath.replace(/[.]/g, "\\.")}\\b`);

  try {
    assert.throws(
      () => runDry([FEATURE_GENESIS_A2O_PREFIXED], { MEASURE_REPORTS_DIR: tempReports }),
      (error) => {
        assert.equal(error.status, 2);
        assert.match(error.stdout.toString(), chmodLine);
        assert.match(error.stderr.toString(), /run the chmod lines above, then retry/);
        return true;
      },
    );

    // Simulate the operator running the printed chmod line.
    chmodSync(blockedPath, 0o646);
    const output = runDry([FEATURE_GENESIS_A2O_PREFIXED], { MEASURE_REPORTS_DIR: tempReports });
    assert.doesNotMatch(output, chmodLine);
  } finally {
    rmSync(tempReports, { recursive: true, force: true });
  }
}

test("a pre-existing root-owned-shaped 0644 cucumber-mesh-scoped.mjs is named in the capacity grant and refuses until writable", () => {
  assertCapacityGrantCoversRewrittenFile("cucumber-mesh-scoped.mjs");
});

// live rung H run 3: cucumber.mjs's htmlReportPath default (reports/cucumber-report.html) has
// no env override anywhere in the justfile or stage-runner.template.sh, so the html formatter
// rewrites this exact path every run — a root-owned 0644 copy threw EACCES there and the run
// came back with an empty scenario selection.
test("a pre-existing root-owned-shaped 0644 cucumber-report.html is named in the capacity grant and refuses until writable", () => {
  assertCapacityGrantCoversRewrittenFile("cucumber-report.html");
});

test("--on adam still refuses (unprovisioned) and now names COMPUTE_SLICE_* as where its bound would come from", () => {
  assert.throws(
    () => runDry([FEATURE_GENESIS_A2O_PREFIXED, "--on", "adam"]),
    (error) => {
      assert.equal(error.status, 2);
      assert.match(error.stderr.toString(), /COMPUTE_SLICE_CPU_MILLIS/);
      assert.match(error.stderr.toString(), /COMPUTE_SLICE_MEMORY_BYTES/);
      return true;
    },
  );
});
