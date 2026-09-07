import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { key, readJson, save } from "./common.mjs";
import { executeTask } from "./worker.mjs";
import { cleanWorker, expired } from "./retention.mjs";

const descriptor = (cid) => ({
  cid,
  bytes: 1,
  sha256: "digest",
  chunks: [{ cid: `${cid}-chunk`, bytes: 1, sha256: "digest" }],
});

test("a denied accept never launches the runtime", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "compute-recovery-denied-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  let runs = 0;
  let authorizations = 0;
  let fetched = 0;

  await assert.rejects(
    executeTask({
      root,
      artifactBase: "http://localhost",
      status: {
        requestActionHash: "request",
        taskCid: "task-cid",
        grantActionHash: "grant",
        envelope: { binary: descriptor("binary"), dna: descriptor("dna") },
      },
      fetchInput: async () => {
        fetched++;
      },
      execute: async (_program, args) => {
        if (args[0] === "run") runs++;
        return args[1].endsWith("task.json") ? "task-cid" : "";
      },
      request: async (path) => {
        if (path.endsWith("/accept")) throw new Error("Compute API 403");
        if (path.endsWith("/authorize-launch")) authorizations++;
        return {};
      },
    }),
    /403/,
  );

  assert.equal(runs, 0);
  assert.equal(authorizations, 0);
  assert.equal(fetched, 0);
});

test("signed refusal cleanup releases both input leases", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "compute-recovery-refusal-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const released = [];
  const status = {
    requestActionHash: "request",
    taskCid: "task-cid",
    refusal: { reason: "compute-grant-refused", actionHash: "refusal" },
    envelope: { binary: descriptor("binary"), dna: descriptor("dna") },
  };

  await executeTask({ root, status });
  await cleanWorker(root, async (cid, method, _body, _retention, owner) => {
    if (method === "DELETE") released.push({ cid, owner });
  });

  assert.deepEqual(released, [
    { cid: "binary-chunk", owner: "task-cid" },
    { cid: "dna-chunk", owner: "task-cid" },
  ]);
  assert.equal(
    (await readJson(join(root, "jobs", key("request"), "state.json")))
      .payloadExpired,
    true,
  );
});

test("remote completion repairs completed state before count cleanup", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "compute-recovery-complete-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const released = [];
  const receipt = {
    requestAction: "a",
    taskCid: "task-a",
    completedAt: 100,
    requester: "requester",
    provider: "provider",
    project: "project",
    taskKind: "task",
    retention: { maxAgeSeconds: null, maxRuns: 1, expireWhen: "either" },
    binary: descriptor("binary-a"),
    dna: descriptor("dna-a"),
  };
  await executeTask({
    root,
    status: { requestActionHash: "a", completion: { receipt } },
    request: async () => {
      throw new Error("must not request execution authority");
    },
  });
  await save(join(root, "jobs", key("b"), "state.json"), {
    completed: true,
    receipt: {
      ...receipt,
      requestAction: "b",
      taskCid: "task-b",
      binary: descriptor("binary-b"),
      dna: descriptor("dna-b"),
    },
  });

  const repaired = await readJson(join(root, "jobs", key("a"), "state.json"));
  assert.equal(repaired.completed, true);
  await cleanWorker(root, async (cid, method, _body, _retention, owner) => {
    if (method === "DELETE") released.push({ cid, owner });
  });

  assert.deepEqual(released, [
    { cid: "binary-a-chunk", owner: "task-a" },
    { cid: "dna-a-chunk", owner: "task-a" },
  ]);
});

test("maxRuns tie ordering is deterministic and independent of input order", () => {
  const base = {
    completedAt: 100,
    requester: "requester",
    provider: "provider",
    project: "project",
    taskKind: "task",
    retention: { maxAgeSeconds: null, maxRuns: 1, expireWhen: "either" },
  };
  const first = { ...base, requestAction: "a" };
  const second = { ...base, requestAction: "b" };

  assert.equal(expired(first, [second, first], 100), true);
  assert.equal(expired(second, [second, first], 100), false);
  assert.equal(expired(first, [first, second], 100), true);
});
