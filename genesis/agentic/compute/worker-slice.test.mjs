// S4a3 — the k8s-rendered sibling container's declared slice is a hard ceiling on what
// this worker may accept, independent of the grant's own bounds. Kept in its own file
// (not compute.test.mjs / recovery.test.mjs) to avoid write-set collisions with other
// lanes editing those siblings.
import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { executeTask } from "./worker.mjs";

function baseOptions(root, resources, env) {
  return {
    root,
    artifactBase: "http://localhost",
    status: {
      requestActionHash: "request",
      taskCid: "task-cid",
      grantActionHash: "grant",
      envelope: { resources },
    },
    env,
  };
}

test("a task over the CPU slice is declined before any accept call", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "compute-slice-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const options = {
    ...baseOptions(
      root,
      { cpuMillis: 4000, memoryBytes: 1024 },
      { COMPUTE_SLICE_CPU_MILLIS: "2000", COMPUTE_SLICE_MEMORY_BYTES: "2048" },
    ),
    fetchInput: async () => {
      throw new Error("fetchInput must not be called");
    },
    execute: async () => {
      throw new Error("execute must not be called");
    },
    request: async () => {
      throw new Error("request (accept) must not be called");
    },
  };
  // If the ceiling check did not fire first, one of the "must not be called"
  // mocks above would reject instead — the regex below pins it to the ceiling.
  await assert.rejects(executeTask(options), /capacity ceiling: task asks 4000m cpu over slice 2000m/);
});

test("a task over the memory slice is declined before any accept call", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "compute-slice-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const options = {
    ...baseOptions(
      root,
      { cpuMillis: 500, memoryBytes: 4096 },
      { COMPUTE_SLICE_CPU_MILLIS: "2000", COMPUTE_SLICE_MEMORY_BYTES: "2048" },
    ),
    fetchInput: async () => {
      throw new Error("fetchInput must not be called");
    },
    execute: async () => {
      throw new Error("execute must not be called");
    },
    request: async () => {
      throw new Error("request (accept) must not be called");
    },
  };
  await assert.rejects(
    executeTask(options),
    /capacity ceiling: task asks 4096 bytes memory over slice 2048 bytes/,
  );
});

test("a task within the declared slice proceeds past the ceiling check", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "compute-slice-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const calls = [];
  const options = {
    ...baseOptions(
      root,
      { cpuMillis: 1000, memoryBytes: 512 },
      { COMPUTE_SLICE_CPU_MILLIS: "2000", COMPUTE_SLICE_MEMORY_BYTES: "2048" },
    ),
    fetchInput: async () => {},
    execute: async (_program, args) => {
      if (args[0] === "run") return '{"status":"passed"}';
      return args[1] && args[1].endsWith("task.json") ? "task-cid" : "receipt-cid";
    },
    request: async (path) => {
      calls.push(path);
      return {};
    },
  };
  await executeTask(options);
  assert.ok(calls.some((path) => path.endsWith("/accept")));
  assert.ok(calls.some((path) => path.endsWith("/complete")));
});

test("no slice env set performs no ceiling check at all", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "compute-slice-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const calls = [];
  const options = {
    ...baseOptions(root, { cpuMillis: 999999, memoryBytes: 999999999 }, {}),
    fetchInput: async () => {},
    execute: async (_program, args) => {
      if (args[0] === "run") return '{"status":"passed"}';
      return args[1] && args[1].endsWith("task.json") ? "task-cid" : "receipt-cid";
    },
    request: async (path) => {
      calls.push(path);
      return {};
    },
  };
  await executeTask(options);
  assert.ok(calls.some((path) => path.endsWith("/accept")));
  assert.ok(calls.some((path) => path.endsWith("/complete")));
});
