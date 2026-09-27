import test from "node:test";
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { createHash } from "node:crypto";
import { dirname, join } from "node:path";
import { promisify } from "node:util";
import {
  A2O_DIR,
  assessDelegability,
  buildStageTask,
  parseFeatureConcern,
  parseScenarioNames,
  probeRunParser,
  slugify,
} from "./build-stage-task.mjs";

const execFileP = promisify(execFile);

const EXECUTOR =
  process.env.COMPUTE_EXECUTOR ||
  "/projects/.cargo-target-pool/family/dev/elohim__rakia/debug/compute-executor"; // = measure.sh PINNED_EXECUTOR
const FEATURE = "features/dataplane/federation-version-convergence.feature";
const REQUESTER = "uhCAkTestRequesterKey";
const PROVIDER = "uhCAkTestProviderKey";
const DECLARED_LITERAL =
  "feedback_signal::a2o::dataplane::federation-version-convergence::two-doorways-that-disagree-about-a-page-converge-on-the-elected-version-without-anyone-re-upload";

// A fake `compute-executor` for tests whose point is build-stage-task's OWN logic (sut,
// concern, template flags) rather than the pinned executor integration the first test above
// already covers against the real binary — the container this test suite runs in does not
// always have `compute-executor` built (S1's untracked-submodule gap), and these tests must
// not depend on that.
function stubExecute() {
  return async (_program, args) => {
    if (args[0] === "artifact") {
      const bytes = (await readFile(args[1])).length;
      return JSON.stringify({
        cid: `stub-cid:${createHash("sha256").update(args[1]).digest("hex").slice(0, 12)}`,
        sha256: createHash("sha256").update(await readFile(args[1])).digest("hex"),
        bytes,
        chunks: [],
      });
    }
    if (args[0] === "run") {
      throw new Error("runtime image identity unavailable or mismatched");
    }
    throw new Error(`stubExecute: unexpected executor invocation ${args.join(" ")}`);
  };
}

test("build-stage-task builds a valid single-scenario stage envelope", async (t) => {
  const out = await mkdtemp(join(tmpdir(), "stage-build-"));
  t.after(() => rm(out, { recursive: true, force: true }));

  const result = await buildStageTask({
    feature: FEATURE,
    requester: REQUESTER,
    provider: PROVIDER,
    out,
    executor: EXECUTOR,
  });

  assert.equal(result.expectedTests.length, 1, "exactly one scenario in this feature");
  assert.match(result.expectedTests[0], /feedback_signal/);
  assert.equal(
    result.expectedTests[0],
    DECLARED_LITERAL,
    "slug must equal the literal pinned in §3 D6",
  );

  const featureBytes = await readFile(join(A2O_DIR, FEATURE));
  const featureSha256 = createHash("sha256").update(featureBytes).digest("hex");
  assert.equal(result.envelope.dna.sha256, featureSha256);
  assert.equal(result.envelope.binary.bytes > 0, true);

  const envelopeBytes = Buffer.byteLength(JSON.stringify(result.envelope));
  assert.equal(envelopeBytes < 65536, true, "envelope must stay under 64 KiB");

  // §4 constraint 1 / §9 task 1's single most valuable check: `run`'s typed parser must
  // accept the envelope shape and reach PAST task parsing to the runtime-image gate —
  // not fail with `unknown field` or `unsupported task`.
  assert.equal(result.probe.ok, true);
  assert.match(result.probe.message, /runtime image identity unavailable or mismatched/);
});

test("probeRunParser accepts the image gate AND the rebuilt executor's artifact-mismatch gate as past-parsing verdicts", async () => {
  const runProbe = async (message) =>
    probeRunParser({
      taskPath: "/tmp/whatever-task.json",
      executor: "fake-executor",
      execute: async () => {
        throw new Error(message);
      },
    });

  const image = await runProbe("runtime image identity unavailable or mismatched");
  assert.equal(image.ok, true);
  assert.match(image.message, /runtime image identity unavailable or mismatched/);

  // The executor rebuilt from the rakia stash checks quota, then artifact identity, BEFORE
  // the runtime-image gate — a throwaway probe binary/dna now fails there first, but that is
  // still evidence the envelope parsed as a well-formed Task, not an envelope rejection.
  const artifact = await runProbe("artifact mismatch");
  assert.equal(artifact.ok, true);
  assert.match(artifact.message, /artifact mismatch/);
});

test("probeRunParser still rejects envelope-parsing failures and real resource refusals", async () => {
  const runProbe = async (message) =>
    probeRunParser({
      taskPath: "/tmp/whatever-task.json",
      executor: "fake-executor",
      execute: async () => {
        throw new Error(message);
      },
    });

  await assert.rejects(
    runProbe("unknown field `bogus`"),
    /envelope rejected before reaching the runtime-image gate/,
  );
  await assert.rejects(
    runProbe("unsupported task kind"),
    /envelope rejected before reaching the runtime-image gate/,
  );
  // A real resource refusal — never accepted as a past-parsing verdict, artifact-mismatch or not.
  await assert.rejects(
    runProbe("actual cgroup ceiling exceeds offered task bound"),
    /unexpected compute-executor run probe result/,
  );
});

test("slugify matches the verbatim §3 D6 expression on the pinned scenario name", () => {
  const scenario =
    "two doorways that disagree about a page converge on the elected version without anyone re-uploading it";
  assert.equal(
    slugify(scenario),
    "two-doorways-that-disagree-about-a-page-converge-on-the-elected-version-without-anyone-re-upload",
  );
});

test("parseScenarioNames finds Scenario: and Scenario Outline: lines without a Gherkin parser", () => {
  const text = [
    "Feature: x",
    "  Background:",
    "    Given y",
    "  Scenario: first one",
    "    Given z",
    "  Scenario Outline: second <one>",
    "    Given w",
  ].join("\n");
  assert.deepEqual(parseScenarioNames(text), ["first one", "second <one>"]);
});

test("parseFeatureConcern reads only the tag lines above Feature:", () => {
  const text = [
    "@e2e @dataplane @concern:federation-deploy @requires:multi-node @act:i",
    "Feature: x",
    "  Scenario: has a Given with @concern: in prose, must not match",
    "    Given a line mentioning @concern:decoy in a comment-like spot",
  ].join("\n");
  assert.equal(parseFeatureConcern(text), "federation-deploy");
  assert.equal(parseFeatureConcern("Feature: no tags\n  Scenario: x"), null);
});

test("stage.json carries the requester's sut/sutParts and the feature's @concern tag; SUT_EXPECTED lands in the runner", async (t) => {
  const out = await mkdtemp(join(tmpdir(), "stage-build-sut-"));
  t.after(() => rm(out, { recursive: true, force: true }));

  const result = await buildStageTask({
    feature: FEATURE,
    requester: REQUESTER,
    provider: PROVIDER,
    out,
    execute: stubExecute(),
    runtimeImage: "sha256:" + "0".repeat(64),
  });

  const stage = JSON.parse(await readFile(result.stagePath, "utf8"));
  assert.equal(stage.feature, FEATURE);
  assert.equal(stage.concern, "federation-deploy");
  assert.match(stage.sut, /^sha256:[0-9a-f]{16}$/);
  assert.equal(stage.sut, result.sut);
  assert.ok(stage.sutParts && typeof stage.sutParts === "object");
  assert.ok(Object.keys(stage.sutParts).length > 0);
  assert.deepEqual(stage.scenarioNames, result.scenarioNames);
  assert.equal(stage.testPrefix, result.testPrefix);

  const runner = await readFile(result.runnerPath, "utf8");
  assert.match(runner, new RegExp(`SUT_EXPECTED='${stage.sut}'`));
});

test("inner household publication is disabled and the inner berth claim is attributable to the stage", async (t) => {
  const out = await mkdtemp(join(tmpdir(), "stage-build-post-report-"));
  t.after(() => rm(out, { recursive: true, force: true }));

  const result = await buildStageTask({
    feature: FEATURE,
    requester: REQUESTER,
    provider: PROVIDER,
    out,
    execute: stubExecute(),
    runtimeImage: "sha256:" + "0".repeat(64),
  });

  const runner = await readFile(result.runnerPath, "utf8");
  assert.match(runner, /export A2O_POST_REPORT=0/);
  // The inner household sprint report stays in guest scratch — never reports/sprint-report-
  // household-*.json, which t2-receipt.sh accepts by mtime as the pre-push T2 receipt.
  assert.match(runner, /export A2O_SPRINT_REPORT_JSON="\$SCRATCH\//);
  assert.match(runner, /export A2O_SPRINT_REPORT_MD="\$SCRATCH\//);
  assert.ok(runner.indexOf("A2O_SPRINT_REPORT_JSON") < runner.indexOf('"$JUST_BIN" test mesh'));
  assert.match(runner, /export BERTH_SESSION="stage:/);
  assert.match(runner, /export BERTH_CLASS="verify"/);

  // The publication guard and berth attribution must land BEFORE `just test mesh` runs.
  const postReportIndex = runner.indexOf("export A2O_POST_REPORT=0");
  const justTestIndex = runner.indexOf('"$JUST_BIN" test mesh');
  assert.ok(postReportIndex > 0 && justTestIndex > postReportIndex);
});

test("--repo-root lands in the rendered runner", async (t) => {
  const out = await mkdtemp(join(tmpdir(), "stage-build-reporoot-"));
  t.after(() => rm(out, { recursive: true, force: true }));

  const customRepoRoot = "/tmp/a-fake-repo-root-for-a-test";
  const result = await buildStageTask({
    feature: FEATURE,
    requester: REQUESTER,
    provider: PROVIDER,
    out,
    execute: stubExecute(),
    runtimeImage: "sha256:" + "0".repeat(64),
    repoRoot: customRepoRoot,
    writablePaths: [join(out, "writable-marker")],
  });

  const runner = await readFile(result.runnerPath, "utf8");
  assert.match(runner, new RegExp(`REPO='${customRepoRoot}'`));
});

test("defaultWritablePaths derives from MESH_DIR, never the retired /tmp/elohim-local-mesh hardcode", async () => {
  const { defaultWritablePaths } = await import("./build-stage-task.mjs");
  const withMeshDir = defaultWritablePaths("/repo", { MESH_DIR: "/custom/mesh" });
  assert.ok(withMeshDir.every((p) => !p.includes("/tmp/elohim-local-mesh")));
  assert.ok(withMeshDir.some((p) => p === "/custom/mesh/matthew/runtime-config.toml"));

  const withoutMeshDir = defaultWritablePaths("/repo", {});
  assert.ok(
    withoutMeshDir.some((p) =>
      p === "/repo/genesis/local-dev/household-dowell/matthew/runtime-config.toml",
    ),
  );
});

test("the git safe.directory export precedes the guest recompute (and the inner just test mesh), and an empty recompute refuses loudly rather than comparing against nothing", async (t) => {
  const out = await mkdtemp(join(tmpdir(), "stage-build-guest-sut-"));
  t.after(() => rm(out, { recursive: true, force: true }));
  const binDir = await mkdtemp(join(tmpdir(), "stage-build-guest-bin-"));
  t.after(() => rm(binDir, { recursive: true, force: true }));

  // A stub `node` on PATH standing in for a guest whose recompute prints nothing at all (the
  // shape a swallowed git failure — or any other silent probe failure — takes): the runner
  // must refuse loudly rather than comparing an empty string against SUT_EXPECTED and reading
  // it as "drifted".
  const fakeNode = join(binDir, "node");
  await writeFile(fakeNode, "#!/bin/sh\nexit 0\n", { mode: 0o755 });

  const result = await buildStageTask({
    feature: FEATURE,
    requester: REQUESTER,
    provider: PROVIDER,
    out,
    execute: stubExecute(),
    runtimeImage: "sha256:" + "0".repeat(64),
    nodeBin: fakeNode,
    justBin: "/bin/true",
    writablePaths: [join(out, "writable-marker")],
  });

  const runner = await readFile(result.runnerPath, "utf8");
  const safeDirIndex = runner.indexOf("GIT_CONFIG_KEY_0=safe.directory");
  const recomputeIndex = runner.indexOf("have_sut=");
  const justTestIndex = runner.indexOf('"$JUST_BIN" test mesh');
  assert.ok(safeDirIndex > 0, "the safe.directory export must be rendered");
  assert.ok(
    recomputeIndex > safeDirIndex,
    "the git safe.directory export must land before the sut recompute",
  );
  assert.ok(
    justTestIndex > safeDirIndex,
    "the git safe.directory export must also precede the inner just test mesh call",
  );

  await assert.rejects(
    execFileP(result.runnerPath, [], { timeout: 60000 }),
    (error) => {
      assert.equal(error.code, 2);
      assert.match(error.stderr, /STAGE-PRECONDITION-UNMET: sut probe failed/);
      return true;
    },
  );
});

test("a provider whose tree has drifted from the pinned sut refuses STAGE-PRECONDITION-UNMET exit 2", async (t) => {
  const out = await mkdtemp(join(tmpdir(), "stage-build-drift-"));
  t.after(() => rm(out, { recursive: true, force: true }));

  const result = await buildStageTask({
    feature: FEATURE,
    requester: REQUESTER,
    provider: PROVIDER,
    out,
    execute: stubExecute(),
    runtimeImage: "sha256:" + "0".repeat(64),
    writablePaths: [join(out, "writable-marker")],
  });

  const runner = await readFile(result.runnerPath, "utf8");
  const drifted = runner.replace(
    `SUT_EXPECTED='${result.sut}'`,
    "SUT_EXPECTED='sha256:deadbeefdeadbeef'",
  );
  assert.notEqual(drifted, runner, "the sut substitution must actually have matched");
  await writeFile(result.runnerPath, drifted, { mode: 0o755 });

  await assert.rejects(
    execFileP(result.runnerPath, [], { timeout: 60000 }),
    (error) => {
      assert.equal(error.code, 2);
      assert.match(error.stderr, /STAGE-PRECONDITION-UNMET: sut drifted:/);
      return true;
    },
  );
});

// Round 2 (2026-09-27, rung H run 1/2 on epr-app-deliverability): the framing step wrote the
// whole base64 report with process.stdout.write and then called process.exit() synchronously.
// On a pipe, node's stdout is asynchronous, so the tail of a large write was dropped at exit —
// the frame stopped at exactly 73728 bytes (72 KiB) with no END sentinel, and the requester
// read "stdout.log did not carry a framed ELOHIM STAGE REPORT". A report well past 72 KiB must
// round-trip byte-for-byte, END marker and summary line included, through a real pipe.
test("a >72 KiB cucumber report round-trips through the framed stage report with its END marker", async (t) => {
  const out = await mkdtemp(join(tmpdir(), "stage-build-bigframe-"));
  t.after(() => rm(out, { recursive: true, force: true }));
  const scratch = await mkdtemp(join(tmpdir(), "stage-build-bigframe-tmp-"));
  t.after(() => rm(scratch, { recursive: true, force: true }));

  const featureText = await readFile(join(A2O_DIR, FEATURE), "utf8");
  const [scenarioName] = parseScenarioNames(featureText);
  // ~256 KiB of step output: comfortably past the 72 KiB truncation point, base64 ~340 KiB.
  const report = JSON.stringify([
    {
      uri: FEATURE,
      elements: [
        {
          type: "scenario",
          name: scenarioName,
          steps: Array.from({ length: 64 }, (_, i) => ({
            name: `step ${i}`,
            result: { status: "passed" },
            output: ["x".repeat(4096)],
          })),
        },
      ],
    },
  ]);
  assert.ok(report.length > 200 * 1024, "the fixture report must exceed the old 72 KiB cut");
  const reportFixture = join(scratch, "fixture-report.json");
  await writeFile(reportFixture, report);
  // Stand-in for `just test mesh <feature>`: writes the fixture to CUCUMBER_JSON_REPORT.
  const fakeJust = join(scratch, "just");
  await writeFile(fakeJust, `#!/bin/sh\ncp '${reportFixture}' "$CUCUMBER_JSON_REPORT"\n`, {
    mode: 0o755,
  });

  const result = await buildStageTask({
    feature: FEATURE,
    requester: REQUESTER,
    provider: PROVIDER,
    out,
    execute: stubExecute(),
    runtimeImage: "sha256:" + "0".repeat(64),
    nodeBin: process.execPath,
    justBin: fakeJust,
    writablePaths: [out],
  });

  const { stdout } = await execFileP(result.runnerPath, [], {
    timeout: 120000,
    maxBuffer: 16 * 1024 * 1024,
    env: { ...process.env, TMPDIR: scratch },
  });
  const begin = "-----BEGIN ELOHIM STAGE REPORT-----\n";
  const end = "\n-----END ELOHIM STAGE REPORT-----\n";
  const from = stdout.indexOf(begin);
  const to = stdout.indexOf(end);
  assert.ok(from >= 0, "the BEGIN sentinel must be present");
  assert.ok(to > from, `the END sentinel must survive the exit (stdout was ${stdout.length} bytes)`);
  const decoded = Buffer.from(stdout.slice(from + begin.length, to), "base64").toString("utf8");
  assert.equal(decoded, report, "the framed report must decode to the exact report bytes");
  assert.match(stdout, /test result: ok\. 1 passed; 0 failed/);
});

// Round 2 (2026-09-27): a stage runs as an unprivileged guest UID. A feature whose step code
// asserts fixture.processControl (owns mongod/doorways) or reads another UID's /proc/<pid>/
// {environ,exe,fd} cannot pass there; the builder refuses it, naming the step evidence, and a
// fixture's scratch root under $MESH_DIR joins the write-set only for the features that use it.
test("assessDelegability: process-owning and /proc-reading lanes refuse; read-only lanes stay delegable", async () => {
  const meshDir = "/mesh";
  const deliverability = await assessDelegability({
    feature: "features/dataplane/epr-app-deliverability.feature",
    meshDir,
  });
  assert.equal(deliverability.delegable, false);
  assert.equal(deliverability.kind, "processControl");
  assert.equal(
    deliverability.headline,
    "features/dataplane/epr-app-deliverability.feature owns processes (processControl) — not delegatable to a guest; run it locally",
  );
  assert.deepEqual(deliverability.fixtureWritable, ["/mesh/scenarios"]);

  // Bound through the hook that names its path (isDeliverabilityFeature), not by file name.
  const isolation = await assessDelegability({
    feature: "features/dataplane/epr-app-channel-isolation.feature",
    meshDir,
  });
  assert.equal(isolation.kind, "processControl");
  assert.ok(isolation.evidence.some((e) => e.file === "steps/dataplane/epr-app-deliverability.steps.ts"));

  const naming = await assessDelegability({ feature: "features/dataplane/name-routing.feature", meshDir });
  assert.equal(naming.kind, "proc-read");
  assert.ok(
    naming.evidence.every((e) => e.file.startsWith("steps/dataplane/")),
    "a same-named module in another steps dir is not bound",
  );

  for (const feature of [
    "features/dataplane/doorway-fixture-readiness.feature",
    "features/dataplane/federation-version-convergence.feature",
  ]) {
    const result = await assessDelegability({ feature, meshDir });
    assert.equal(result.delegable, true, feature);
    assert.deepEqual(result.fixtureWritable, []);
  }
});

test("buildStageTask refuses a process-owning feature before writing anything", async (t) => {
  const out = join(await mkdtemp(join(tmpdir(), "stage-build-refuse-")), "never-created");
  t.after(() => rm(dirname(out), { recursive: true, force: true }));
  await assert.rejects(
    buildStageTask({
      feature: "features/dataplane/epr-app-deliverability.feature",
      requester: REQUESTER,
      provider: PROVIDER,
      out,
      execute: stubExecute(),
      runtimeImage: "sha256:" + "0".repeat(64),
    }),
    /owns processes \(processControl\) — not delegatable to a guest; run it locally\n  evidence: genesis\/a2o\/steps\/dataplane\//,
  );
  await assert.rejects(readFile(join(out, "task.json")), { code: "ENOENT" });
});
