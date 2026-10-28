import test from "node:test";
import assert from "node:assert/strict";
import vm from "node:vm";
import {
  allowed,
  providerTransport,
  podProgram,
} from "./runner-authority-transport.mjs";
const provider = `uhCAk${"A".repeat(48)}`,
  recipient = `uhCAk${"B".repeat(48)}`;
const ref = `uhCkk${"C".repeat(48)}`;
const activation = {
  provider,
  recipient,
  grantCid: "production-cid",
  grantActionHash: "production-action",
  worker: {
    state: "running",
    command: "/opt/compute/worker.mjs",
    namespace: "elohim-alpha",
    pod: "adam-0",
    podUid: "uid",
    containerId: "containerd://abc",
    restartCount: 0,
    image: `registry/worker@sha256:${"a".repeat(64)}`,
  },
};
const pod = () => ({
  metadata: { uid: "uid" },
  spec: {
    containers: [{ name: "compute-worker", image: activation.worker.image }],
  },
  status: {
    phase: "Running",
    containerStatuses: [
      {
        name: "compute-worker",
        ready: true,
        containerID: "containerd://abc",
        restartCount: 0,
        imageID: "containerd://sha256:actual",
      },
    ],
  },
});
test("transport pins incarnation before/after and sends no local capability from CI", async () => {
  const calls = [];
  const request = providerTransport(activation, async (args, input) => {
    calls.push({ args, input });
    return args.includes("get")
      ? JSON.stringify(pod())
      : '{"status":403,"body":{"reason":"compute-grant-refused"}}';
  });
  const response = await request(
    "POST",
    `/api/v1/compute/tasks/${ref}/authorize-launch`,
    { attemptId: "attempt" },
  );
  assert.equal(response.status, 403);
  assert.equal(calls.length, 3);
  assert(!calls[1].input.includes("ELOHIM_COMPUTE_LOCAL_TOKEN"));
  assert.equal(calls[1].args.at(-1), podProgram);
});
test("incarnation drift refuses a mutation response rather than publishing stale success", async () => {
  let reads = 0;
  const request = providerTransport(activation, async (args) => {
    if (!args.includes("get"))
      return '{"status":403,"body":{"reason":"compute-grant-refused"}}';
    const value = pod();
    if (++reads === 2) value.status.containerStatuses[0].restartCount = 1;
    return JSON.stringify(value);
  });
  await assert.rejects(
    request("POST", `/api/v1/compute/tasks/${ref}/accept`, { attemptId: "x" }),
  );
});
test("exact route allowlist excludes arbitrary paths, query strings and production declarations", () => {
  assert(allowed("GET", `/api/v1/compute/tasks/${ref}`));
  assert(allowed("GET", `/api/v1/economic-events/compute-admission:${ref}`));
  assert(!allowed("GET", `/api/v1/economic-events/compute-admission%3A${ref}`));
  for (const path of [
    "/api/v1/compute/grants?revoke=true",
    "/api/v1/compute/tasks/../grants",
    `/api/v1/compute/tasks/${ref}/complete`,
  ])
    assert(!allowed("POST", path));
});
for (const mode of ["legacy", "supervised", "backoff", "retry"]) {
  test(`fixed in-pod program keeps token local and checks the worker (${mode})`, async () => {
    const secret = "pod-only-capability-never-output-123456789";
    const input = {
      activation,
      method: "POST",
      path: `/api/v1/compute/tasks/${ref}/accept`,
      body: { attemptId: "x" },
      profile: { repoRoot: "/provider/repo" },
    };
    const files = {
      0: JSON.stringify(input),
      "/proc/1/cmdline":
        mode === "legacy"
          ? "node\0/opt/compute/worker.mjs\0"
          : "bash\0-c\0supervisor\0",
      "/proc/1/task/1/children": "42 ",
      "/proc/42/cmdline":
        mode === "backoff" ? "sleep\0" : "node\0/opt/compute/worker.mjs\0",
      "/var/lib/compute/runner-grant.json": JSON.stringify({ ...activation }),
    };
    let output = "",
      writes = 0,
      statReads = 0;
    const context = {
      Buffer,
      URL,
      AbortSignal,
      console: {
        log: (s) => {
          output += s;
        },
        error: () => {},
      },
      process: {
        env: {
          COMPUTE_RUNNER_GRANT_JSON: JSON.stringify({
            enabled: true,
            provider,
            recipient,
            fixtureProvider: "adam",
            stewardOfRecord: "human:matthew",
          }),
          COMPUTE_PERFORMER: provider,
          COMPUTE_RUNTIME_IMAGE: activation.worker.image.split("@")[1],
          ELOHIM_COMPUTE_LOCAL_TOKEN: secret,
        },
      },
      require: () => ({
        readFileSync: (p) =>
          p.endsWith?.("/stat")
            ? "42 (node) S " +
              Array(18).fill("0").join(" ") +
              " " +
              (mode === "retry" ? ++statReads : 100)
            : files[p],
        realpathSync: (p) => {
          assert.ok(
            [
              "/provider/repo",
              `/proc/${mode === "legacy" ? 1 : 42}/cwd`,
            ].includes(p),
          );
          return "/provider/repo";
        },
      }),
      fetch: async (url, options) => {
        const health = url.pathname === "/health";
        if (!health) {
          writes++;
          assert.equal(options.headers["x-elohim-compute-token"], secret);
        }
        const data = health
          ? { dhtParticipation: { agentKeys: { lamad: provider } } }
          : { reason: "compute-grant-refused", token: secret };
        return {
          ok: true,
          status: health ? 200 : 403,
          body: (async function* () {
            yield Buffer.from(JSON.stringify(data));
          })(),
        };
      },
    };
    await vm.runInNewContext(podProgram, context);
    if (["backoff", "retry"].includes(mode)) {
      assert.equal(writes, 0);
      assert.equal(context.process.exitCode, 1);
      assert.equal(output, "");
      return;
    }
    assert.equal(writes, 1);
    assert(!output.includes(secret));
    assert.deepEqual(JSON.parse(output), {
      status: 403,
      body: { reason: "compute-grant-refused" },
    });
  });
}
