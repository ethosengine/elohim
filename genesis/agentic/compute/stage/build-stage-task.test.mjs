import test from "node:test";
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { createHash } from "node:crypto";
import { join } from "node:path";
import { promisify } from "node:util";
import {
  A2O_DIR,
  buildStageTask,
  parseFeatureConcern,
  parseScenarioNames,
  probeRunParser,
  slugify,
} from "./build-stage-task.mjs";

const execFileP = promisify(execFile);

const EXECUTOR =
  process.env.COMPUTE_EXECUTOR ||
  "/projects/.cargo-target-pool/family/dev/elohim__rakia/dev/debug/compute-executor";
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
