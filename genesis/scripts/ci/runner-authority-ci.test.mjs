import test from "node:test";
import assert from "node:assert/strict";
import { captureEnv } from "./runner-authority-ci.mjs";
const provider = "uhCAk" + "A".repeat(48),
  recipient = "uhCAk" + "B".repeat(48);
const worker = {
  enabled: true,
  performer: provider,
  image: "registry/worker@sha256:" + "a".repeat(64),
  slice: { cpuMillis: 8000, memoryBytes: 25769803776 },
  runnerGrant: {
    enabled: true,
    provider,
    recipient,
    stewardOfRecord: "human:matthew",
    fixtureProvider: "adam",
  },
};
const env = {
  RUN_ADAM_AUTHORITY_PROBE: "true",
  ADMIN_KEY: "test-secret",
  DOORWAY_URL: "http://doorway",
  PROBE_WORKLOAD: "elohim-adam-alpha",
  PROBE_NAMESPACE: "elohim-alpha",
};
test("maps reviewed capacity and exact native identities; fixes distinct email service agent", () => {
  const result = captureEnv(worker, env);
  assert.equal(result.COMPUTE_WORKER_PROVIDER, provider);
  assert.equal(result.COMPUTE_WORKER_RECIPIENT, recipient);
  assert.equal(result.COMPUTE_WORKER_MEMORY_BYTES, "25769803776");
  assert.equal(
    result.RUNNER_IDENTIFIER,
    "jenkins-adam-authority-probe@alpha.elohim.host",
  );
  assert.equal(
    result.RUNNER_SERVICE_AGENT_ID,
    "agent:ci-runner-jenkins-adam-authority-probe",
  );
});
test("refuses before effects for missing opt-in, wrong stewardship, aliases and unpinned image", () => {
  for (const [w, e] of [
    [worker, { ...env, RUN_ADAM_AUTHORITY_PROBE: "" }],
    [{ ...worker, performer: recipient }, env],
    [{ ...worker, image: "latest" }, env],
    [
      {
        ...worker,
        runnerGrant: { ...worker.runnerGrant, stewardOfRecord: "human:adam" },
      },
      env,
    ],
    [
      {
        ...worker,
        runnerGrant: { ...worker.runnerGrant, recipient: provider },
      },
      env,
    ],
  ])
    assert.throws(() => captureEnv(w, e));
});

test("CLI refusal replaces stale public success without exposing secrets across Jenkins UIDs", async (t) => {
  const {
    mkdtempSync,
    mkdirSync,
    writeFileSync,
    readFileSync,
    statSync,
    existsSync,
    rmSync,
  } = await import("node:fs");
  const { tmpdir } = await import("node:os");
  const { join } = await import("node:path");
  const { execFileSync } = await import("node:child_process");
  const root = mkdtempSync(join(tmpdir(), "authority-public-refusal-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "genesis/orchestrator/data"), { recursive: true });
  writeFileSync(
    join(root, "genesis/orchestrator/data/adam-compute-worker.json"),
    JSON.stringify(worker),
  );
  for (const name of [
    "adam-authority-probe.json",
    "adam-authority-activation.json",
    "adam-authority-status.txt",
  ])
    writeFileSync(join(root, name), "READY stale");
  assert.throws(() =>
    execFileSync(
      process.execPath,
      [new URL("./runner-authority-ci.mjs", import.meta.url).pathname],
      {
        env: {
          ...process.env,
          WORKSPACE: root,
          RUN_ADAM_AUTHORITY_PROBE: "false",
          ADMIN_KEY: "secret-sentinel",
        },
        stdio: "pipe",
      },
    ),
  );
  assert(!existsSync(join(root, "adam-authority-probe.json")));
  assert(!existsSync(join(root, "adam-authority-activation.json")));
  const status = join(root, "adam-authority-status.txt");
  assert.match(readFileSync(status, "utf8"), /^REFUSED:/);
  assert(!readFileSync(status, "utf8").includes("secret-sentinel"));
  assert.equal(statSync(status).mode & 0o777, 0o644);
});
