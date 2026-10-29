import test from "node:test";
import { execFileSync, spawn } from "node:child_process";
import { once } from "node:events";
import { setTimeout as sleep } from "node:timers/promises";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import YAML from "yaml";
import { renderPoolCompute } from "./scripts/render-pool-compute.mjs";
import { quantity, renderWorker } from "./scripts/render-compute-worker.mjs";

const source = await readFile(
  new URL("./manifests/humans/adam-firstman.yaml", import.meta.url),
  "utf8",
);

test("disabled worker leaves Adam manifest byte-identical", () => {
  assert.equal(renderWorker(source, { enabled: false }), source);
});

test("enabled worker rejects floating image tags", () => {
  assert.throws(
    () =>
      renderWorker(source, {
        enabled: true,
        image: "worker:latest",
        performer: "adam",
      }),
    /immutable/,
  );
});

test("enabled worker without a declared slice refuses, naming S4a2", async () => {
  const image = `registry/worker@sha256:${"a".repeat(64)}`;
  await assert.rejects(
    renderWorker(source, { enabled: true, image, performer: "adam-key" }),
    /declared slice.*S4a2/,
  );
});

test("a synthetic undersized envelope refuses the worker", async (t) => {
  const repoRoot = await writeFittingFixtureRepo();
  t.after(() => rm(repoRoot, { recursive: true, force: true }));
  const file = join(
    repoRoot,
    "genesis/orchestrator/manifests/runtime/adam.manifest.json",
  );
  const manifest = JSON.parse(await readFile(file, "utf8"));
  manifest.envelope.bound.memory_bytes = 1024;
  await writeFile(file, JSON.stringify(manifest));
  await assert.rejects(
    renderWorker(
      source,
      {
        enabled: true,
        image: `registry/worker@sha256:${"a".repeat(64)}`,
        performer: "adam-key",
        slice: { cpuMillis: 500, memoryBytes: 512 * 1024 * 1024 },
      },
      { repoRoot },
    ),
    /Σ limits .* > envelope.bound/,
  );
});

test("the declared Adam 24Gi worker fits its current full envelope", async () => {
  const config = JSON.parse(
    await readFile(
      new URL("./data/adam-compute-worker.json", import.meta.url),
      "utf8",
    ),
  );
  const output = await renderWorker(
    renderPoolCompute(source, "adam", "alpha"),
    {
      ...config,
      enabled: true,
      image: config.image || `registry/worker@sha256:${"a".repeat(64)}`,
      performer: config.performer || "adam-key",
      // The envelope does not depend on the grant's window; the test below owns that.
      runnerGrant: { ...config.runnerGrant, enabled: false },
    },
  );
  assert.match(output, /memory: 24576Mi/);
});

test("the declared Adam runner grant is inside its window, or withdrawn", async (t) => {
  const { runnerGrant } = JSON.parse(
    await readFile(
      new URL("./data/adam-compute-worker.json", import.meta.url),
      "utf8",
    ),
  );
  if (runnerGrant?.enabled !== true) return;
  const { validUntil } = runnerGrant.grant;
  const left = Date.parse(validUntil) - Date.now();
  assert.ok(
    left > 0,
    `adam's runner grant expired at ${validUntil}: renew the window or withdraw it ` +
      "through a reviewed deploy-only commit (genesis/agentic/compute/operations.md)",
  );
  if (left < 48 * 3600 * 1000)
    t.diagnostic(`adam's runner grant expires at ${validUntil}, in under 48 h`);
});

test("quantity() mirrors bridges/k8s/src/lib.rs's parser over the same examples", () => {
  assert.equal(quantity("8Gi", "memory"), quantity("8192Mi", "memory"));
  assert.equal(quantity("1.5", "cpu"), quantity("1500m", "cpu"));
  for (const value of [
    "-1Mi",
    "NaNMi",
    "1e3Mi",
    "1.2.3Mi",
    "Mi",
    "8G",
    "18446744073709551615Ti",
  ]) {
    assert.throws(() => quantity(value, "memory"), Error, value);
  }
});

async function writeFittingFixtureRepo() {
  // A synthetic repo tree with a generous envelope bound and an empty-container
  // conductor manifest, isolating renderer invariants from the deployed configuration.
  const root = await mkdtemp(join(tmpdir(), "compute-worker-fixture-"));
  const orchestratorDir = join(root, "genesis", "orchestrator");
  await mkdir(join(orchestratorDir, "data"), { recursive: true });
  await mkdir(join(orchestratorDir, "manifests", "runtime"), {
    recursive: true,
  });
  await mkdir(join(orchestratorDir, "manifests", "humans"), {
    recursive: true,
  });
  await writeFile(
    join(orchestratorDir, "data", "deployments.json"),
    JSON.stringify({
      humans: [
        {
          name: "adam",
          runtimeManifest: {
            path: "genesis/orchestrator/manifests/runtime/adam.manifest.json",
          },
          conductorManifest:
            "genesis/orchestrator/manifests/humans/adam-firstman-conductor.yaml",
        },
      ],
    }),
  );
  await writeFile(
    join(orchestratorDir, "manifests", "runtime", "adam.manifest.json"),
    JSON.stringify({
      processes: [
        {
          name: "compute-worker",
          kind: "delegated",
          quota: { memory_max_bytes: 512 * 1024 * 1024, cpu_share_millis: 500 },
        },
      ],
      envelope: {
        bound: { memory_bytes: 100 * 1024 * 1024 * 1024, cpu_millis: 100000 },
      },
    }),
  );
  await writeFile(
    join(
      orchestratorDir,
      "manifests",
      "humans",
      "adam-firstman-conductor.yaml",
    ),
    "apiVersion: apps/v1\nkind: StatefulSet\nmetadata:\n  name: adam-conductor-fixture\nspec:\n  template:\n    spec:\n      containers: []\n",
  );
  return root;
}

test("a slice that fits renders the worker with the slice as limits and slice env", async (t) => {
  const repoRoot = await writeFittingFixtureRepo();
  t.after(() => rm(repoRoot, { recursive: true, force: true }));
  const image = `registry/worker@sha256:${"a".repeat(64)}`;
  const slice = { cpuMillis: 500, memoryBytes: 512 * 1024 * 1024 };
  const output = YAML.parseAllDocuments(
    await renderWorker(
      source,
      { enabled: true, image, performer: "adam-key", slice },
      { repoRoot },
    ),
  )
    .map((document) => document.toJS())
    .filter(Boolean);
  const original = YAML.parseAllDocuments(source)
    .map((document) => document.toJS())
    .find((doc) => doc.kind === "StatefulSet");
  const manifest = output.find((doc) => doc.kind === "StatefulSet");
  const pod = manifest.spec.template.spec;
  const worker = pod.containers.find(
    (container) => container.name === "compute-worker",
  );
  assert.equal(pod.automountServiceAccountToken, false);
  assert.equal(
    worker.env.find((item) => item.name === "COMPUTE_API_URL").value,
    "http://127.0.0.1:8090",
  );
  assert.equal(
    worker.env.find((item) => item.name === "COMPUTE_SLICE_CPU_MILLIS").value,
    "500",
  );
  assert.equal(
    worker.env.find((item) => item.name === "COMPUTE_SLICE_MEMORY_BYTES").value,
    String(512 * 1024 * 1024),
  );
  assert.deepEqual(worker.resources, {
    requests: { cpu: "500m", memory: "512Mi" },
    limits: { cpu: "500m", memory: "512Mi" },
  });
  assert.equal(
    output.find((doc) => doc.kind === "PersistentVolumeClaim").spec.resources
      .requests.storage,
    "24Gi",
  );
  assert.deepEqual(
    pod.containers.find((container) => container.name === "elohim-node")
      .resources,
    original.spec.template.spec.containers.find(
      (container) => container.name === "elohim-node",
    ).resources,
  );
  assert.equal(worker.volumeMounts.length, 1);
});

test("the local token comes from the Secret by default, or is minted per render and shared by both containers", async (t) => {
  const repoRoot = await writeFittingFixtureRepo();
  t.after(() => rm(repoRoot, { recursive: true, force: true }));
  const base = {
    enabled: true,
    image: `registry/worker@sha256:${"a".repeat(64)}`,
    performer: "adam-key",
    slice: { cpuMillis: 500, memoryBytes: 512 * 1024 * 1024 },
  };
  const tokenOf = async (config, overrides = {}) => {
    const pod = YAML.parseAllDocuments(
      await renderWorker(source, config, { repoRoot, ...overrides }),
    )
      .map((document) => document.toJS())
      .find((doc) => doc?.kind === "StatefulSet").spec.template.spec;
    const read = (container) =>
      pod.containers
        .find((c) => c.name === container)
        .env.find((item) => item.name === "ELOHIM_COMPUTE_LOCAL_TOKEN");
    return [read("elohim-node"), read("compute-worker")];
  };
  const [storageSecret, workerSecret] = await tokenOf(base);
  assert.equal(storageSecret.valueFrom.secretKeyRef.key, "token");
  assert.deepEqual(storageSecret, workerSecret);

  const [storagePlain, workerPlain] = await tokenOf(
    { ...base, localTokenSource: "per-render" },
    { localToken: "fixed-for-test" },
  );
  assert.equal(storagePlain.value, "fixed-for-test");
  assert.equal(workerPlain.value, "fixed-for-test");
  assert.equal(storagePlain.valueFrom, undefined);

  const [first] = await tokenOf({ ...base, localTokenSource: "per-render" });
  const [second] = await tokenOf({ ...base, localTokenSource: "per-render" });
  assert.match(first.value, /^[a-f0-9]{64}$/);
  assert.notEqual(first.value, second.value);

  await assert.rejects(
    renderWorker(source, { ...base, localTokenSource: "file" }, { repoRoot }),
    /unknown localTokenSource/,
  );
});

test("deploy-only provisions the renderer owner before any human rollout", async () => {
  const edge = await readFile(
    new URL("../../elohim/holochain/Jenkinsfile", import.meta.url),
    "utf8",
  );
  const start = edge.indexOf("def deployHumansInParallel(");
  const install = edge.indexOf(
    "sh 'pnpm install --frozen-lockfile --filter elohim-orchestrator'",
    start,
  );
  const branches = edge.indexOf("humans.each", start);
  assert.ok(start >= 0 && install > start && install < branches);
  const prefix = edge.slice(start, install);
  assert.ok(
    !/skipBuildStages|DEPLOY_ONLY|isValidateOnly/.test(prefix),
    "provisioning is not gated by build mode",
  );
  const owner = JSON.parse(
    await readFile(new URL("./package.json", import.meta.url), "utf8"),
  );
  assert.equal(owner.dependencies.yaml, "2.8.3");
});

test("the deploy renderer CLI reads its YAML dependency and preserves disabled fixture bytes", async () => {
  const dir = await mkdtemp(join(tmpdir(), "compute-worker-cli-"));
  try {
    const manifest = join(dir, "adam.yaml");
    const config = join(dir, "disabled.json");
    await writeFile(manifest, source);
    await writeFile(config, JSON.stringify({ enabled: false }));
    execFileSync(process.execPath, [
      fileURLToPath(
        new URL("./scripts/render-compute-worker.mjs", import.meta.url),
      ),
      manifest,
      config,
    ]);
    assert.equal(await readFile(manifest, "utf8"), source);
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});

test("pool sponsorship is alpha-only and shares doorway token inputs in order", async () => {
  for (const [human, doorway] of [
    ["matthew", "alpha"],
    ["adam", "alpha-b"],
  ]) {
    const output = renderPoolCompute(source, human, "alpha");
    const storage = YAML.parseAllDocuments(output)
      .map((d) => d.toJS())
      .find((d) => d?.kind === "StatefulSet")
      .spec.template.spec.containers.find((c) => c.name === "elohim-node");
    const env = storage.env;
    const bootstrap = env.find((v) => v.name === "POOL_COMPUTE_BOOTSTRAP_KEY");
    const token = env.find((v) => v.name === "ELOHIM_COMPUTE_LOCAL_TOKEN");
    const doorwayDocs = YAML.parseAllDocuments(
      await readFile(
        new URL(`./manifests/doorway/${doorway}.yaml`, import.meta.url),
        "utf8",
      ),
    ).map((d) => d.toJS());
    const doorwayEnv = doorwayDocs
      .find((d) => d?.kind === "Deployment")
      .spec.template.spec.containers.find((c) =>
        c.env?.some((v) => v.name === "POOL_COMPUTE_TOKEN"),
      ).env;
    assert.deepEqual(
      bootstrap.valueFrom,
      doorwayEnv.find((v) => v.name === "POOL_COMPUTE_BOOTSTRAP_KEY").valueFrom,
    );
    assert.equal(
      token.value,
      doorwayEnv.find((v) => v.name === "POOL_COMPUTE_TOKEN").value,
    );
    assert.ok(env.indexOf(bootstrap) < env.indexOf(token));
    assert.equal(
      env.find((v) => v.name === "ELOHIM_COMPUTE_LOCAL_API").value,
      "1",
    );
  }
  assert.equal(renderPoolCompute(source, "adam", "prod"), source);
  assert.equal(renderPoolCompute(source, "jessica", "alpha"), source);
});

test("worker retains the storage token dependency and opts in to a pinned private mesh", async (t) => {
  const repoRoot = await writeFittingFixtureRepo();
  t.after(() => rm(repoRoot, { recursive: true, force: true }));
  const config = {
    enabled: true,
    image: `registry/worker@sha256:${"a".repeat(64)}`,
    performer: "adam-key",
    slice: { cpuMillis: 500, memoryBytes: 512 * 1024 * 1024 },
    localTokenSource: "storage",
    privateMesh: true,
    sourceCommit: "b".repeat(40),
    resumeInitializationLogSha256: "c".repeat(64),
    resumeInitializationStateSha256: "d".repeat(64),
  };
  const render = async (input, settings) =>
    YAML.parseAllDocuments(await renderWorker(input, settings, { repoRoot }))
      .map((d) => d.toJS())
      .find((d) => d?.kind === "StatefulSet").spec.template.spec.containers;
  const containers = await render(
    renderPoolCompute(source, "adam", "alpha"),
    config,
  );
  const worker = containers.find((c) => c.name === "compute-worker");
  const storage = containers.find((c) => c.name === "elohim-node");
  assert.deepEqual(worker.command.slice(0, 2), ["bash", "-c"]);
  assert.match(
    worker.command[2],
    /setsid bash \/opt\/compute\/worker-start\.sh &/,
  );
  assert.match(worker.command[2], /while \(\( !stopping \)\); do/);
  assert.equal(worker.readinessProbe, undefined);
  assert.equal(worker.livenessProbe, undefined);
  assert.equal(
    worker.env.find((v) => v.name === "COMPUTE_PRIVATE_MESH").value,
    "1",
  );
  assert.equal(
    worker.env.find((v) => v.name === "COMPUTE_SOURCE_COMMIT").value,
    config.sourceCommit,
  );
  assert.equal(
    worker.env.find(
      (v) => v.name === "COMPUTE_CONTINUE_INITIALIZATION_LOG_SHA256",
    ).value,
    config.resumeInitializationLogSha256,
  );
  await assert.rejects(
    render(source, {
      ...config,
      resumeInitializationLogSha256: "unreviewed",
    }),
    /reviewed log digest/,
  );
  await assert.rejects(
    render(source, { ...config, privateMesh: false }),
    /reviewed log digest/,
  );
  assert.equal(
    worker.env.find(
      (v) => v.name === "COMPUTE_CONTINUE_INITIALIZATION_STATE_SHA256",
    ).value,
    config.resumeInitializationStateSha256,
  );
  for (const patch of [
    { resumeInitializationStateSha256: "unreviewed" },
    { resumeInitializationLogSha256: undefined },
  ]) {
    await assert.rejects(
      render(source, { ...config, ...patch }),
      /reviewed state digest/,
    );
  }
  const input = worker.env.find((v) => v.name === "POOL_COMPUTE_BOOTSTRAP_KEY");
  const token = worker.env.find((v) => v.name === "ELOHIM_COMPUTE_LOCAL_TOKEN");
  assert.deepEqual(
    input,
    storage.env.find((v) => v.name === input.name),
  );
  assert.deepEqual(
    token,
    storage.env.find((v) => v.name === token.name),
  );
  assert.ok(worker.env.indexOf(input) < worker.env.indexOf(token));
  await assert.rejects(render(source, config), /storage compute token missing/);
  await assert.rejects(
    render(source, {
      ...config,
      slice: { cpuMillis: 999, memoryBytes: 512 * 1024 * 1024 },
    }),
    /delegated quota/,
  );
  await assert.rejects(
    render(source, { ...config, sourceCommit: "dev" }),
    /immutable git commit/,
  );
  const ordinary = await render(source, {
    ...config,
    privateMesh: false,
    sourceCommit: undefined,
    resumeInitializationLogSha256: undefined,
    resumeInitializationStateSha256: undefined,
    localTokenSource: "secret",
  });
  assert.equal(
    ordinary
      .find((c) => c.name === "compute-worker")
      .env.some((v) => v.name === "COMPUTE_PRIVATE_MESH"),
    false,
  );
});

for (const signal of ["SIGTERM", "SIGINT"]) {
  test(
    `rendered supervisor retries failures, releases the lock and cleans children on ${signal}`,
    { timeout: 8000 },
    async (t) => {
      const repoRoot = await writeFittingFixtureRepo();
      const worker = YAML.parseAllDocuments(
        await renderWorker(
          source,
          {
            enabled: true,
            image: `registry/worker@sha256:${"a".repeat(64)}`,
            performer: "adam-key",
            slice: { cpuMillis: 500, memoryBytes: 512 * 1024 * 1024 },
          },
          { repoRoot },
        ),
      )
        .map((doc) => doc.toJS())
        .find((doc) => doc?.kind === "StatefulSet")
        .spec.template.spec.containers.find(
          (container) => container.name === "compute-worker",
        );
      const stub = join(repoRoot, "worker-start.sh");
      await writeFile(
        stub,
        `#!/usr/bin/env bash
state="$COMPUTE_STATE_ROOT"
exec 9>"$state/worker.lock"
flock -n 9 || exit 98
echo "$$" >> "$state/attempts"
bash -c 'trap "" TERM INT; echo "$$" >> "$COMPUTE_STATE_ROOT/children"; exec sleep 60' &
child=$!
until [[ -f "$state/children" ]] && grep -qx "$child" "$state/children"; do sleep 0.01; done
if [[ -f "$state/retried" ]]; then
  wait "$child"
else
  touch "$state/retried"
  exit 1
fi
`,
      );
      const supervisor = spawn(
        worker.command[0],
        [
          worker.command[1],
          worker.command[2].replaceAll(
            "/opt/compute/worker-start.sh",
            '"$COMPUTE_TEST_WORKER"',
          ),
        ],
        {
          env: {
            ...process.env,
            COMPUTE_TEST_WORKER: stub,
            COMPUTE_STATE_ROOT: repoRoot,
            COMPUTE_SUPERVISOR_BACKOFF_START: "1",
            COMPUTE_SUPERVISOR_BACKOFF_CAP: "2",
            COMPUTE_SUPERVISOR_STOP_GRACE: "0.1",
          },
          stdio: ["ignore", "ignore", "pipe"],
        },
      );
      const closed = once(supervisor, "close");
      let stderr = "";
      supervisor.stderr.on("data", (chunk) => {
        stderr += chunk;
      });
      const pids = async (name) => {
        try {
          return (await readFile(join(repoRoot, name), "utf8"))
            .trim()
            .split("\n")
            .map(Number);
        } catch (error) {
          if (error.code === "ENOENT") return [];
          throw error;
        }
      };
      const running = async (pid) => {
        try {
          const stat = await readFile(`/proc/${pid}/stat`, "utf8");
          // Outside a PID namespace, the host's PID 1 owns orphan reaping.
          return !["Z", "X"].includes(
            stat.slice(stat.lastIndexOf(")") + 2).split(" ")[0],
          );
        } catch (error) {
          // A process that exits between open and read answers ESRCH.
          if (["ENOENT", "ESRCH"].includes(error.code)) return false;
          throw error;
        }
      };
      t.after(async () => {
        supervisor.kill("SIGKILL");
        for (const pid of await pids("attempts")) {
          try {
            process.kill(-pid, "SIGKILL");
          } catch (error) {
            if (error.code !== "ESRCH") throw error;
          }
        }
        await closed;
        await rm(repoRoot, { recursive: true, force: true });
      });
      const deadline = Date.now() + 5000;
      while ((await pids("children")).length < 2 && Date.now() < deadline)
        await sleep(20);
      const attempts = await pids("attempts");
      const children = await pids("children");
      assert.equal(attempts.length, 2, stderr);
      assert.equal(children.length, 2, stderr);
      assert.equal(
        await running(children[0]),
        false,
        "failed attempt's child was killed before retry",
      );
      const status = await readFile(
        join(repoRoot, "supervisor-status.txt"),
        "utf8",
      );
      assert.match(
        status,
        /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ exit=1 attempt=1\n$/,
      );
      assert.ok(stderr.includes(status));
      const stoppedAt = Date.now();
      supervisor.kill(signal);
      const [code, exitSignal] = await closed;
      assert.equal(code, 0, stderr);
      assert.equal(exitSignal, null);
      assert.ok(
        Date.now() - stoppedAt < 1500,
        "termination must not wait for backoff or child exit",
      );
      for (const pid of [...attempts, ...children])
        assert.equal(await running(pid), false, `surviving child ${pid}`);
      const lock = spawn("flock", [
        "-n",
        join(repoRoot, "worker.lock"),
        "true",
      ]);
      assert.deepEqual(await once(lock, "close"), [0, null]);
    },
  );
}

test("only explicit reviewed runner grants reach the provider bootstrap environment", async (t) => {
  const repoRoot = await writeFittingFixtureRepo();
  t.after(() => rm(repoRoot, { recursive: true, force: true }));
  const provider = "uhCAk" + "a".repeat(48),
    recipient = "uhCAk" + "b".repeat(48);
  const from = new Date(Date.now() - 60000).toISOString();
  const until = new Date(Date.now() + 3600000).toISOString();
  const runnerGrant = {
    enabled: true,
    stewardOfRecord: "human:matthew",
    fixtureProvider: "adam",
    provider,
    recipient,
    grant: {
      recipient,
      scope: "measure-stage",
      issuedAt: from,
      validFrom: from,
      validUntil: until,
      bounds: {
        epr_scope: ["*"],
        reach_ceiling: "commons",
        rate_per_hour: 8,
        rotation_ttl_days: 1,
      },
    },
  };
  const config = {
    enabled: true,
    image: `registry/worker@sha256:${"a".repeat(64)}`,
    performer: provider,
    slice: { cpuMillis: 500, memoryBytes: 512 * 1024 * 1024 },
    runnerGrant,
  };
  const output = await renderWorker(source, config, { repoRoot });
  const worker = YAML.parseAllDocuments(output)
    .map((d) => d.toJS())
    .find((d) => d?.kind === "StatefulSet")
    .spec.template.spec.containers.find((c) => c.name === "compute-worker");
  assert.deepEqual(
    JSON.parse(
      worker.env.find((v) => v.name === "COMPUTE_RUNNER_GRANT_JSON").value,
    ),
    runnerGrant,
  );
  await assert.rejects(
    renderWorker(source, { ...config, performer: recipient }, { repoRoot }),
    /grant provider differs/,
  );
  const disabled = await renderWorker(
    source,
    { ...config, runnerGrant: { enabled: false } },
    { repoRoot },
  );
  assert.ok(!disabled.includes("COMPUTE_RUNNER_GRANT_JSON"));
});

test("CI worker observation and native probe permissions are scoped to Adam's exact pod", async () => {
  const documents = YAML.parseAllDocuments(
    await readFile(
      new URL(
        "./manifests/rbac/jenkins-deployer-elohim-alpha.yaml",
        import.meta.url,
      ),
      "utf8",
    ),
  ).map((doc) => doc.toJS());
  const role = documents.find((doc) => doc.kind === "Role");
  const binding = documents.find((doc) => doc.kind === "RoleBinding");
  assert.equal(role.metadata.namespace, "elohim-alpha");
  assert.deepEqual(binding.subjects, [
    { kind: "ServiceAccount", name: "jenkins-deployer", namespace: "jenkins" },
  ]);
  assert.equal(binding.roleRef.name, role.metadata.name);
  const podRules = role.rules.filter((rule) =>
    rule.resources.some((name) => name.startsWith("pods/")),
  );
  assert.equal(podRules.length, 2);
  for (const rule of podRules) {
    assert.deepEqual(rule.apiGroups, [""]);
    assert.deepEqual(rule.resourceNames, ["elohim-adam-alpha-0"]);
    assert.deepEqual(
      rule.verbs,
      rule.resources[0] === "pods/log" ? ["get"] : ["get", "create"],
    );
    assert(["pods/log", "pods/exec"].includes(rule.resources[0]));
    assert.equal(rule.resources.length, 1);
  }
});
