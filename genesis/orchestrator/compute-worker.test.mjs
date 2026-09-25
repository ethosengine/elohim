import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import YAML from "yaml";
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

test("enabled worker at today's hardcoded numbers refuses the Σ envelope bound", async () => {
  const image = `registry/worker@sha256:${"a".repeat(64)}`;
  await assert.rejects(
    renderWorker(source, {
      enabled: true,
      image,
      performer: "adam-key",
      // The worker's own resources.requests/limits before S4a1 (2000m/4Gi, 8000m/8Gi) —
      // reproduced here as the slice to prove the refusal names the pod's real, already
      // over-budget totals (R6/rung-A finding: 16448Mi limits > 8192Mi bound).
      slice: { cpuMillis: 8000, memoryBytes: 8 * 1024 * 1024 * 1024 },
    }),
    /Σ limits 16448Mi > envelope\.bound 8192Mi/,
  );
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
  // conductor manifest, isolating the "a slice that fits" success path from the real
  // adam pod's already-over-budget totals (named in the refusal test above).
  const root = await mkdtemp(join(tmpdir(), "compute-worker-fixture-"));
  const orchestratorDir = join(root, "genesis", "orchestrator");
  await mkdir(join(orchestratorDir, "data"), { recursive: true });
  await mkdir(join(orchestratorDir, "manifests", "runtime"), { recursive: true });
  await mkdir(join(orchestratorDir, "manifests", "humans"), { recursive: true });
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
      envelope: { bound: { memory_bytes: 100 * 1024 * 1024 * 1024, cpu_millis: 100000 } },
    }),
  );
  await writeFile(
    join(orchestratorDir, "manifests", "humans", "adam-firstman-conductor.yaml"),
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
