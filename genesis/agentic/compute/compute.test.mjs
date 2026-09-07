import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, rm, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createHash } from "node:crypto";
import { api, save, taskPath, readJson } from "./common.mjs";
import { issueGrant, pollInbox, reviewCommand } from "./workspace.mjs";
import { executeTask, guestEnvironment } from "./worker.mjs";
import { expired } from "./retention.mjs";
import { leaseSeconds, payloadClient, materialize } from "./payloads.mjs";

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
