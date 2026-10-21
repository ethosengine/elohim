import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { jsonCid, execute } from "../../agentic/compute/cid-executor.mjs";
import { buildStageTask } from "../../agentic/compute/stage/build-stage-task.mjs";
import {
  runStage,
  validateConfig,
  verifyCompletion,
  verifyGrant,
} from "./runner-stage.mjs";

const feature = "features/dataplane/example.feature";
const config = {
  cpuMillis: 2000,
  memoryBytes: 2147483648,
  timeoutSeconds: 1800,
  provider: "provider",
  runnerGrant: "grant",
  repoRoot: "/provider/repo",
  nodeBin: "/bin/node",
  justBin: "/bin/just",
  writablePaths: ["/provider/reports"],
  runtimeImage: `sha256:${"a".repeat(64)}`,
};
const grant = {
  state: "active",
  forThisRunner: true,
  runner: "runner",
  recipient: "runner",
  provider: "provider",
  scope: "measure-stage",
  grantActionHash: "grant",
};

async function fixture(stepStatus = "passed") {
  const envelope = {
    provider: "provider",
    requester: "runner",
    project: "a2o-stage:example",
    binary: { cid: "binary" },
    dna: { cid: "dna" },
    runtimeImage: config.runtimeImage,
    expectedTests: ["example"],
    taskKind: "feedback_signal",
    retention: { maxAgeSeconds: 60 },
  };
  const stage = {
    feature,
    concern: "example",
    scenarioNames: ["Real outcome"],
  };
  const doc = [
    {
      uri: feature,
      elements: [
        {
          type: "scenario",
          name: "Real outcome",
          steps: [
            { name: "the real observation", result: { status: stepStatus } },
          ],
        },
      ],
    },
  ];
  const stdout = Buffer.from(
    `-----BEGIN ELOHIM STAGE REPORT-----\n${Buffer.from(JSON.stringify(doc)).toString("base64")}\n-----END ELOHIM STAGE REPORT-----`,
  );
  const stderr = Buffer.alloc(0);
  const taskCid = await jsonCid(envelope);
  const receipt = {
    ...envelope,
    schemaVersion: 1,
    taskCid,
    requestAction: "request",
    grantAction: "grant",
    status: stepStatus === "passed" ? "passed" : "failed",
    exitCode: stepStatus === "passed" ? 0 : 101,
    observedTests: ["example"],
    logs: [
      ["stdout.log", stdout],
      ["stderr.log", stderr],
    ].map(([name, data]) => ({
      name,
      bytes: data.length,
      sha256: createHash("sha256").update(data).digest("hex"),
    })),
  };
  const task = {
    taskCid,
    requestActionHash: "request",
    grantActionHash: "grant",
    envelope,
    acceptance: { attemptId: "attempt" },
    completion: { receipt, receiptCid: await jsonCid(receipt) },
  };
  return {
    task,
    envelope,
    stage,
    stdout,
    stderr,
    submitted: {
      requestActionHash: "request",
      taskCid,
      grantActionHash: "grant",
    },
  };
}

test("only the runner's active measure-stage grant from the selected provider is spendable", () => {
  assert.equal(verifyGrant(grant, config), "runner");
  for (const change of [
    { state: "withdrawn" },
    { recipient: "someone" },
    { scope: "sweettest-feedback" },
    { provider: "other" },
    { forThisRunner: false },
  ])
    assert.throws(() => verifyGrant({ ...grant, ...change }, config));
});

test("explicit provider configuration refuses missing pins, traversal and shell-bearing paths", () => {
  assert.equal(validateConfig(config, feature), config);
  for (const change of [
    { runnerGrant: null },
    { runtimeImage: "latest" },
    { repoRoot: "/tmp/'oops" },
    { writablePaths: [] },
  ])
    assert.throws(() => validateConfig({ ...config, ...change }, feature));
  assert.throws(() => validateConfig(config, "features/../../secret.feature"));
});

test("actual failed and skipped steps cannot become green receipts", async () => {
  assert.equal((await verifyCompletion(await fixture())).verdict, "pass");
  for (const status of ["failed", "skipped", "pending", "undefined"])
    assert.equal(
      (await verifyCompletion(await fixture(status))).verdict,
      "fail",
    );
});

test("wrong request, forged receipts, changed logs, missing inventory and premature completion refuse", async () => {
  for (const mutate of [
    (f) => {
      f.task.requestActionHash = "another-request";
    },
    (f) => {
      f.task.completion.receipt.provider = "other";
    },
    (f) => {
      f.stdout = Buffer.from("forged success");
    },
    (f) => {
      f.stage.scenarioNames.push("Missing");
    },
    (f) => {
      f.task.refusal = { reason: "compute-grant-refused" };
    },
    (f) => {
      f.task.acceptance = null;
    },
  ]) {
    const f = await fixture();
    mutate(f);
    try {
      assert.notEqual((await verifyCompletion(f)).verdict, "pass");
    } catch (error) {
      assert.notEqual(error.code, "ERR_ASSERTION");
    }
  }
});

test("refusal removes a previous Jenkins snapshot before any network call", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "runner-ci-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const reports = join(root, "genesis/a2o/reports");
  await mkdir(reports, { recursive: true });
  await writeFile(
    join(reports, "cucumber-report-peer-stage.json"),
    "old green",
  );
  await assert.rejects(
    runStage({
      root,
      env: { RUNNER_STAGE_CONFIG: "/no-config" },
      callRunner: () => {
        assert.fail("must not contact provider");
      },
    }),
  );
  await assert.rejects(
    readFile(join(reports, "cucumber-report-peer-stage.json")),
  );
});

test("Jenkins pins its source checkout while the runner names the provider's different path", async (t) => {
  const out = await mkdtemp(join(tmpdir(), "runner-ci-build-"));
  t.after(() => rm(out, { recursive: true, force: true }));
  const sourceRepoRoot = resolve(import.meta.dirname, "../../..");
  const options = {
    feature: "features/dataplane/federation-version-convergence.feature",
    requester: "runner",
    provider: "provider",
    out,
    runtimeImage: config.runtimeImage,
    execute,
    probeRun: false,
    sourceRepoRoot,
  };
  const local = await buildStageTask({ ...options, repoRoot: sourceRepoRoot });
  const remote = await buildStageTask({
    ...options,
    repoRoot: "/different/provider/repo",
  });
  assert.equal(remote.sut, local.sut);
  assert.notEqual(remote.sut, "sha256:e3b0c44298fc1c14");
  assert.match(
    await readFile(remote.runnerPath, "utf8"),
    /REPO='\/different\/provider\/repo'/,
  );
});

test("submission, authenticated reread, and publication preserve real failed outcomes", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "runner-ci-flow-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const configPath = join(root, "provider.json");
  await writeFile(configPath, JSON.stringify(config));
  const f = await fixture("failed");
  const calls = [];
  const build = async ({ out, sourceRepoRoot, repoRoot }) => {
    assert.equal(sourceRepoRoot, root);
    assert.equal(repoRoot, config.repoRoot);
    const taskPath = join(out, "task.json"),
      stagePath = join(out, "stage.json");
    await writeFile(taskPath, JSON.stringify(f.envelope));
    await writeFile(stagePath, JSON.stringify(f.stage));
    return {
      taskPath,
      stagePath,
      runnerPath: "runner.sh",
      dnaPath: "example.feature",
      sut: "sha256:1234567890abcdef",
    };
  };
  let withdrawn = false;
  const callRunner = async (args, { stdout }) => {
    calls.push(args[0]);
    let response;
    if (args[0] === "grant-status")
      response = { ...grant, state: withdrawn ? "withdrawn" : "active" };
    if (args[0] === "submit") response = f.submitted;
    if (args[0] === "poll") {
      const out = args.at(-1);
      const logPaths = [join(out, "stdout.log"), join(out, "stderr.log")];
      await writeFile(logPaths[0], f.stdout);
      await writeFile(logPaths[1], f.stderr);
      response = { task: f.task, logPaths };
    }
    stdout(JSON.stringify(response));
    return 0;
  };
  const options = {
    root,
    build,
    callRunner,
    env: {
      RUNNER_STAGE_CONFIG: configPath,
      RUNNER_STAGE_FEATURE: feature,
      RUNNER_JWT: "test-only",
    },
  };
  assert.equal(await runStage(options), 1);
  assert.deepEqual(calls, ["grant-status", "submit", "poll", "grant-status"]);
  const report = JSON.parse(
    await readFile(
      join(root, "genesis/a2o/reports/cucumber-report-peer-stage.json"),
    ),
  );
  assert.equal(report[0].elements[0].steps[0].result.status, "failed");
  withdrawn = true;
  await assert.rejects(runStage(options), /active measure-stage grant/);
  await assert.rejects(
    readFile(join(root, "genesis/a2o/reports/cucumber-report-peer-stage.json")),
  );
});
