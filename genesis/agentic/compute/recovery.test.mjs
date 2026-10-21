import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
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

test("a lost launch response consumes admission without launching or inventing completion", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "compute-lost-launch-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const admission = join(root, "provider-admission.json");
  const runtimePath = join(root, "runs", key("request"), "receipt.json");
  const attempts = [];
  let admissions = 0;
  let launches = 0;
  let completions = 0;
  const options = {
    root,
    artifactBase: "http://localhost",
    status: {
      requestActionHash: "request",
      taskCid: "task-cid",
      grantActionHash: "grant",
      envelope: { binary: descriptor("binary"), dna: descriptor("dna") },
    },
    fetchInput: async () => {},
    execute: async (_program, args) => {
      if (args[0] === "receipt-path") return runtimePath;
      if (args[0] === "cid") return "task-cid";
      launches++;
      assert.fail("a consumed but unanswered admission must never launch");
    },
    request: async (path, body) => {
      if (path.endsWith("/accept")) {
        attempts.push(body.attemptId);
        return {};
      }
      if (path.endsWith("/authorize-launch")) {
        // The provider commits its unique admission before sending the response.
        // Retrying the same request cannot insert another admission, even with
        // a reconstructed worker invocation and no runtime receipt yet.
        try {
          await writeFile(admission, JSON.stringify(body), { flag: "wx" });
        } catch (error) {
          if (error.code === "EEXIST")
            throw new Error("Compute API 409: admission consumed");
          throw error;
        }
        admissions++;
        throw new Error("connection lost after admission committed");
      }
      completions++;
      assert.fail(`unexpected provider write ${path}`);
    },
  };
  await assert.rejects(
    executeTask(options),
    /connection lost after admission committed/,
  );
  await assert.rejects(executeTask({ ...options }), /admission consumed/);
  assert.equal(admissions, 1);
  assert.equal(attempts.length, 2);
  assert.equal(attempts[0], attempts[1]);
  assert.equal((await readJson(admission)).attemptId, attempts[0]);
  assert.equal(launches, 0);
  assert.equal(completions, 0);
  const state = await readJson(
    join(root, "jobs", key("request"), "state.json"),
  );
  assert.equal(state.receipt, undefined);
  assert.notEqual(state.completed, true);
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

test("an interrupted attempt seals evidence after revocation without renewed launch authority", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "compute-recovery-interrupted-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const runtimePath = join(root, "runs", key("request"), "receipt.json");
  await save(join(root, "runs", key("request"), "started.json"), {
    taskCid: "task-cid",
    grantAction: "grant",
    requestAction: "request",
    startedAt: 1,
  });
  const receipt = {
    taskCid: "task-cid",
    grantAction: "grant",
    requestAction: "request",
    status: "interrupted",
    logs: [],
  };
  const calls = [];
  let recoveries = 0;
  await executeTask({
    root,
    status: {
      requestActionHash: "request",
      taskCid: "task-cid",
      grantActionHash: "grant",
      envelope: {},
    },
    fetchInput: async () => {
      assert.fail("recovery must not download inputs");
    },
    execute: async (_program, args) => {
      if (args[0] === "receipt-path") return runtimePath;
      if (args[0] === "run") {
        assert.ok(
          args.includes("--recover-only"),
          "recovery must never authorize a new launch",
        );
        recoveries++;
        return JSON.stringify(receipt);
      }
      return args[1].endsWith("task.json") ? "task-cid" : "receipt-cid";
    },
    request: async (path, body) => {
      calls.push(path);
      if (path.endsWith("/accept") || path.endsWith("/authorize-launch"))
        throw new Error("Compute API 403: grant revoked");
      assert.deepEqual(body.receipt, receipt);
      return {};
    },
  });
  assert.equal(recoveries, 1);
  assert.equal(calls.length, 1);
  assert.match(calls[0], /\/complete$/);
  assert.equal(
    (await readJson(join(root, "jobs", key("request"), "state.json")))
      .completed,
    true,
  );
});

test("a missing authoritative attempt cannot turn a stale marker into launch authority", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "compute-recovery-stale-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const runtimePath = join(root, "runs", key("request"), "receipt.json");
  await save(join(root, "runs", key("request"), "started.json"), {});
  let recoveries = 0;
  await assert.rejects(
    executeTask({
      root,
      status: {
        requestActionHash: "request",
        taskCid: "task-cid",
        grantActionHash: "grant",
        envelope: {},
      },
      fetchInput: async () => {
        assert.fail("a denied launch must not download inputs");
      },
      execute: async (_program, args) => {
        if (args[0] === "receipt-path") return runtimePath;
        if (args[0] === "run") {
          assert.ok(args.includes("--recover-only"));
          recoveries++;
          return "null";
        }
        return "task-cid";
      },
      request: async (path) => {
        assert.match(path, /\/accept$/);
        throw new Error("Compute API 403: grant revoked");
      },
    }),
    /403/,
  );
  assert.equal(recoveries, 1);
  assert.equal(
    (await readJson(join(root, "jobs", key("request"), "state.json"))).receipt,
    undefined,
  );
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
