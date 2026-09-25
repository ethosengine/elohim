import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, rm, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createHash } from "node:crypto";
import { api, save, taskPath, readJson } from "./common.mjs";
import { issueGrant, pollInbox, reviewCommand } from "./workspace.mjs";
import { executeTask, guestEnvironment } from "./worker.mjs";
import { expired } from "./retention.mjs";
import { leaseSeconds, payloadClient, materialize } from "./payloads.mjs";
import { collectStageEvidence, stageVerdict } from "./stage/collect-stage.mjs";
import {
  checkNameFor,
  cidToString,
  sutArtifactCidBytes,
} from "../../a2o/scripts/lib/household-attestation.ts";

test("retention expires by age or count and scopes count to requester/project", () => {
  const receipt = {
    completedAt: 100,
    requester: "a",
    provider: "adam",
    project: "p",
    taskKind: "feedback_signal",
    retention: { maxAgeSeconds: 100, maxRuns: 1, expireWhen: "either" },
  };
  assert.equal(expired(receipt, [], 199), false);
  assert.equal(expired(receipt, [], 200), true);
  assert.equal(expired(receipt, [{ ...receipt, completedAt: 101 }], 102), true);
  assert.equal(
    expired(receipt, [{ ...receipt, project: "other", completedAt: 101 }], 102),
    false,
  );
});

test("native payload requests bind leases to task and reject undeclared owner", async () => {
  let headers;
  const payload = payloadClient(
    "http://localhost:8090",
    "local-secret",
    async (_url, options) => {
      headers = options.headers;
      return new Response("data");
    },
  );
  await assert.rejects(payload("cid"), /owner/);
  assert.equal(
    (await payload("cid", "GET", undefined, 60, "task-cid")).toString(),
    "data",
  );
  assert.equal(headers["x-compute-owner"], "task-cid");
  assert.equal(headers["x-elohim-compute-token"], "local-secret");
});

test("materialization verifies every native chunk and the full artifact", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "compute-chunk-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const digest = (text) => createHash("sha256").update(text).digest("hex");
  const descriptor = {
    bytes: 4,
    sha256: digest("data"),
    chunks: [{ cid: "chunk", bytes: 4, sha256: digest("data") }],
  };
  let owner;
  await materialize(
    descriptor,
    join(root, "out"),
    async (_cid, _method, _body, _ttl, lease) => {
      owner = lease;
      return Buffer.from("data");
    },
    "task",
  );
  assert.equal(owner, "task");
  await assert.rejects(
    materialize(
      descriptor,
      join(root, "bad"),
      async () => Buffer.from("oops"),
      "task",
    ),
    /digest/,
  );
});

test("local adapter refuses remote control plane URLs", () => {
  assert.throws(() => api("https://adam.example", "agent"), /loopback/);
  assert.doesNotThrow(() => api("http://127.0.0.1:8090", "agent"));
});

test("review delivery retries errors and deduplicates successful adapters across restarts", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "compute-inbox-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  await save(taskPath(root, "request"), { requestActionHash: "request" });
  let codex = 0;
  let claude = 0;
  const options = {
    root,
    adapters: ["codex", "claude"],
    request: async () => ({ completion: { actionHash: "done" } }),
    review: async (program) => {
      if (program === "codex") {
        codex++;
        return "review";
      }
      if (++claude === 1) throw new Error("temporary delivery outage");
      return "review";
    },
  };
  await pollInbox(options);
  await pollInbox(options);
  await pollInbox(options);
  assert.equal(codex, 1);
  assert.equal(claude, 2);
  assert.equal(
    (await readJson(taskPath(root, "request"))).reviews.claude.completion,
    "done",
  );
});

test("review adapters constrain tools and guests cannot inherit signing credentials", () => {
  assert.deepEqual(reviewCommand("codex")[1].slice(0, 3), [
    "exec",
    "--sandbox",
    "read-only",
  ]);
  assert.ok(reviewCommand("claude")[1].includes("Read,Grep,Glob"));
  assert.deepEqual(
    guestEnvironment({
      PATH: "/bin",
      COMPUTE_API_TOKEN: "secret",
      HC_SIGNING_KEY: "key",
    }),
    { PATH: "/bin" },
  );
});

test("completion publication retries use durable receipt without executing again", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "compute-worker-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const calls = [];
  let executions = 0;
  let completions = 0;
  const options = {
    root,
    artifactBase: "http://localhost",
    status: {
      requestActionHash: "request",
      taskCid: "task-cid",
      grantActionHash: "grant",
      envelope: {},
    },
    fetchInput: async () => {},
    execute: async (_program, args) => {
      if (args[0] === "run") {
        executions++;
        return '{"status":"passed"}';
      }
      return args[1].endsWith("task.json") ? "task-cid" : "receipt-cid";
    },
    request: async (path) => {
      calls.push(path);
      if (path.endsWith("/accept") && completions > 0)
        throw new Error("grant now revoked");
      if (path.endsWith("/complete") && ++completions === 1)
        throw new Error("offline");
      return {};
    },
  };
  await assert.rejects(executeTask(options), /offline/);
  await executeTask(options);
  assert.equal(executions, 1);
  assert.equal(completions, 2);
  assert.ok(calls.some((path) => path.endsWith("/authorize-launch")));
});

test("attachment leases preserve count-only and combined retention", () => {
  assert.equal(
    leaseSeconds({ maxAgeSeconds: 432000, maxRuns: 5, expireWhen: "either" }),
    432000,
  );
  assert.equal(
    leaseSeconds({ maxAgeSeconds: null, maxRuns: 5, expireWhen: "either" }),
    0,
  );
  assert.equal(
    leaseSeconds({ maxAgeSeconds: 432000, maxRuns: 5, expireWhen: "both" }),
    0,
  );
});

test("count retention breaks completion timestamp ties by signed request reference", () => {
  const receipt = {
    requestAction: "a",
    completedAt: 100,
    requester: "me",
    provider: "adam",
    project: "p",
    taskKind: "feedback_signal",
    retention: { maxAgeSeconds: null, maxRuns: 1, expireWhen: "either" },
  };
  const next = { ...receipt, requestAction: "b" };
  assert.equal(expired(receipt, [receipt, next], 100), true);
  assert.equal(expired(next, [receipt, next], 100), false);
});

test("remote completion repairs local cleanup state without renewed execution authority", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "compute-reconcile-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const receipt = { taskCid: "task", requestAction: "request" };
  await executeTask({
    root,
    status: { requestActionHash: "request", completion: { receipt } },
    request: async () => {
      throw new Error("must not request execution authority");
    },
  });
  const { key } = await import("./common.mjs");
  const state = await readJson(
    join(root, "jobs", key("request"), "state.json"),
  );
  assert.equal(state.completed, true);
  assert.deepEqual(state.receipt, receipt);
});

test("explicit provider grant issuance preserves its signing time across retries", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "compute-grant-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const file = join(root, "grant.json");
  await save(file, {
    recipient: "requester",
    validFrom: "2026-09-07T00:00:00Z",
    validUntil: "2026-09-08T00:00:00Z",
    bounds: { rate_per_hour: 2 },
  });
  const calls = [];
  const request = async (path, body) => {
    calls.push({ path, body: structuredClone(body) });
    return { grantActionHash: "signed-create" };
  };
  await issueGrant(file, request);
  await issueGrant(file, request);
  assert.equal(calls[0].path, "/api/v1/compute/grants");
  assert.equal(calls[0].body.issuedAt, calls[1].body.issuedAt);
  assert.deepEqual(calls[0].body.bounds, { rate_per_hour: 2 });
});

// ─── S3a — collectStageEvidence (genesis/agentic/compute/stage/collect-stage.mjs) ──────────

function cucumberReport({ uri, scenarioName, passed, tags = [] }) {
  return [
    {
      uri,
      elements: [
        {
          type: "scenario",
          name: scenarioName,
          tags: tags.map((name) => ({ name })),
          steps: [{ result: { status: passed ? "passed" : "failed", duration: 1000000 } }],
        },
      ],
    },
  ];
}

function framedStdout(reportDoc) {
  const encoded = Buffer.from(JSON.stringify(reportDoc)).toString("base64");
  return [
    "running 1 test",
    "test x::y ... ok",
    "",
    "-----BEGIN ELOHIM STAGE REPORT-----",
    encoded,
    "-----END ELOHIM STAGE REPORT-----",
    "",
    "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out",
    "",
  ].join("\n");
}

async function makeStageFixture(t, { passed = true, preconditionUnmet = false } = {}) {
  const dir = await mkdtemp(join(tmpdir(), "collect-stage-logs-"));
  t.after(() => rm(dir, { recursive: true, force: true }));

  const stage = {
    feature: "features/dataplane/fixture.feature",
    featureSha256: "f".repeat(64),
    concern: "fixture-concern",
    sut: "sha256:0123456789abcdef",
    sutParts: { a2o: "tree:abc" },
    testPrefix: "feedback_signal::a2o::dataplane::fixture",
    scenarioNames: ["fixture scenario"],
  };

  const reportDoc = cucumberReport({
    uri: stage.feature,
    scenarioName: "fixture scenario",
    passed,
  });
  const stdoutText = framedStdout(reportDoc);
  const stderrText = preconditionUnmet
    ? "STAGE-PRECONDITION-UNMET: not writable: /some/path\n"
    : "";
  const stdoutPath = join(dir, "stdout.log");
  const stderrPath = join(dir, "stderr.log");
  await writeFile(stdoutPath, stdoutText);
  await writeFile(stderrPath, stderrText);

  const entry = {
    requestActionHash: "req-hash-1",
    stage,
    rung: "H",
    logPaths: [stdoutPath, stderrPath],
  };

  const status = {
    requestActionHash: "req-hash-1",
    taskCid: "task-cid-1",
    provider: "uhCAkProviderKeyXXXXXXXXXXXXXXXX",
    requester: "uhCAkRequesterKeyXXXXXXXXXXXXXX",
    grantActionHash: "grant-action-hash",
    envelope: { project: "a2o-stage:fixture", dna: { sha256: stage.featureSha256 } },
    completion: {
      actionHash: "completion-action-hash",
      receiptCid: "receipt-cid-1",
      receipt: {
        logs: [
          { name: "stdout.log", sha256: createHash("sha256").update(stdoutText).digest("hex") },
          { name: "stderr.log", sha256: createHash("sha256").update(stderrText).digest("hex") },
        ],
      },
    },
    observed: {
      verified: true,
      scope: "measure-stage",
      grantCid: "grant-cid-1",
      grantProvider: "uhCAkProviderKeyXXXXXXXXXXXXXXXX",
      grantRecipient: "uhCAkRequesterKeyXXXXXXXXXXXXXX",
      fulfilledEventId: "compute-fulfilled:req-hash-1",
    },
  };

  return { dir, stage, entry, status };
}

function fakeStageRunners({ deliverImpl } = {}) {
  const calls = { putAttestation: [], deliverStageResult: [] };
  const runners = {
    resolveBinary: (name) => (name === "brit-build-ref" ? "/fake/brit-build-ref" : null),
    gitCommonDir: () => "/fake/workspace/.git",
    putAttestation: (brit, workspace, attestation, runFn, cwd) => {
      calls.putAttestation.push({ brit, workspace, attestation, cwd });
      return { status: 0, stdout: "", stderr: "" };
    },
    deliverStageResult: async (args) => {
      calls.deliverStageResult.push(args);
      if (deliverImpl) return deliverImpl(args);
      return {
        reportPath: null,
        fulfill: { skipped: true, reason: "test" },
        delta: { skipped: true, reason: "test" },
      };
    },
    run: () => ({ status: 0, stdout: "", stderr: "" }),
  };
  return { runners, calls };
}

test("stageVerdict reads the decoded report, not receipt.status, and shapes ReceiptScenario rows", () => {
  const stage = {
    feature: "features/x/y.feature",
    concern: "c",
    scenarioNames: ["a scenario"],
  };
  const passing = stageVerdict(
    cucumberReport({ uri: stage.feature, scenarioName: "a scenario", passed: true }),
    stage,
  );
  assert.equal(passing.verdict, "pass");
  assert.deepEqual(passing.scenarios, [
    { name: "a scenario", status: "passed", surface: stage.feature, durationMs: 1 },
  ]);

  const failing = stageVerdict(
    cucumberReport({ uri: stage.feature, scenarioName: "a scenario", passed: false }),
    stage,
  );
  assert.equal(failing.verdict, "fail");

  const mismatchedInventory = stageVerdict(
    cucumberReport({ uri: stage.feature, scenarioName: "a DIFFERENT scenario", passed: true }),
    stage,
  );
  assert.equal(mismatchedInventory.verdict, "fail");
});

test("collectStageEvidence writes one attestation keyed by sut with peer.receiptCid on a pass", async (t) => {
  const cwd = await mkdtemp(join(tmpdir(), "collect-stage-cwd-"));
  t.after(() => rm(cwd, { recursive: true, force: true }));
  const { stage, entry, status } = await makeStageFixture(t, { passed: true });
  const { runners, calls } = fakeStageRunners();

  const evidence = await collectStageEvidence({ entry, status, root: cwd, cwd, runners });

  assert.equal(evidence.done, true);
  assert.equal(evidence.verdict, "pass");
  assert.equal(calls.putAttestation.length, 1);
  const { attestation } = calls.putAttestation[0];
  assert.equal(attestation.check, checkNameFor(stage.concern, stage.sut));
  assert.equal(attestation.artifact, cidToString(sutArtifactCidBytes(stage.sutParts)));
  assert.equal(attestation.result, "pass");
  assert.equal(attestation.summary.peer.receiptCid, status.completion.receiptCid);
  assert.equal(attestation.summary.peer.provider, status.provider);
  assert.equal(attestation.summary.peer.requester, status.requester);
  assert.equal(attestation.summary.peer.completionActionHash, status.completion.actionHash);
  assert.equal(attestation.summary.lane, "household"); // rung H, honestly household stand-in
  assert.equal(attestation.summary.processControl, true);
  assert.equal(calls.deliverStageResult.length, 1);

  const durable = join(cwd, evidence.report);
  const cucumberBytes = await readFile(join(durable, "cucumber.json"));
  assert.ok(cucumberBytes.length > 0);
  const receiptJson = await readJson(join(durable, "receipt.json"));
  assert.deepEqual(receiptJson, status.completion.receipt);
});

test("a receipt claiming success with a report showing a failed step is a fail, never a pass", async (t) => {
  const cwd = await mkdtemp(join(tmpdir(), "collect-stage-cwd-"));
  t.after(() => rm(cwd, { recursive: true, force: true }));
  const { entry, status } = await makeStageFixture(t, { passed: false });
  const { runners, calls } = fakeStageRunners();

  const evidence = await collectStageEvidence({ entry, status, root: cwd, cwd, runners });
  assert.equal(evidence.verdict, "fail");
  assert.equal(calls.putAttestation[0].attestation.result, "fail");
});

test("STAGE-PRECONDITION-UNMET in stderr.log yields a skip verdict and writes no attestation", async (t) => {
  const cwd = await mkdtemp(join(tmpdir(), "collect-stage-cwd-"));
  t.after(() => rm(cwd, { recursive: true, force: true }));
  const { entry, status } = await makeStageFixture(t, { preconditionUnmet: true });
  const { runners, calls } = fakeStageRunners();

  const evidence = await collectStageEvidence({ entry, status, root: cwd, cwd, runners });
  assert.equal(evidence.verdict, "skip");
  assert.match(evidence.reason, /STAGE-PRECONDITION-UNMET/);
  assert.equal(calls.putAttestation.length, 0);
  assert.equal(calls.deliverStageResult.length, 0);
});

test("collectStageEvidence is idempotent across restarts: a second call does nothing further", async (t) => {
  const cwd = await mkdtemp(join(tmpdir(), "collect-stage-cwd-"));
  t.after(() => rm(cwd, { recursive: true, force: true }));
  const { entry, status } = await makeStageFixture(t, { passed: true });
  const { runners, calls } = fakeStageRunners();

  const first = await collectStageEvidence({ entry, status, root: cwd, cwd, runners });
  const second = await collectStageEvidence({ entry, status, root: cwd, cwd, runners });
  assert.equal(first, second);
  assert.equal(calls.putAttestation.length, 1);
  assert.equal(calls.deliverStageResult.length, 1);
});

test("an unobserved grant records a refusal and attempts no attestation", async (t) => {
  const cwd = await mkdtemp(join(tmpdir(), "collect-stage-cwd-"));
  t.after(() => rm(cwd, { recursive: true, force: true }));
  const { entry, status } = await makeStageFixture(t, { passed: true });
  status.observed = { verified: false, refused: "signed grant party or scope mismatch" };
  const { runners, calls } = fakeStageRunners();

  const evidence = await collectStageEvidence({ entry, status, root: cwd, cwd, runners });
  assert.equal(evidence.done, true);
  assert.equal(evidence.verdict, null);
  assert.match(evidence.refused, /signed grant party or scope mismatch/);
  assert.equal(calls.putAttestation.length, 0);
});

test("pollInbox still delivers review when the stage's grant is unobserved", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "compute-inbox-stage-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const { entry, status } = await makeStageFixture(t, { passed: true });
  status.observed = { verified: false, refused: "not observed" };
  await save(taskPath(root, "request-1"), { ...entry, requestActionHash: "request-1" });

  let reviewCalls = 0;
  await pollInbox({
    root,
    adapters: ["codex"],
    request: async () => status,
    review: async () => {
      reviewCalls++;
      return "review output";
    },
    stageRunners: fakeStageRunners().runners,
  });

  assert.equal(reviewCalls, 1);
  const saved = await readJson(taskPath(root, "request-1"));
  assert.equal(saved.evidence.verdict, null);
  assert.match(saved.evidence.refused, /not observed/);
});
