import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import YAML from "yaml";
import { renderWorker } from "./scripts/render-compute-worker.mjs";

const source = await readFile(
  new URL("./manifests/humans/adam-firstman.yaml", import.meta.url),
  "utf8",
);
test("disabled worker leaves Adam manifest byte-identical", () => {
  assert.equal(renderWorker(source, { enabled: false }), source);
});
test("enabled worker shares loopback only and has separate bounded resources", () => {
  const image = `registry/worker@sha256:${"a".repeat(64)}`;
  const output = YAML.parseAllDocuments(
    renderWorker(source, { enabled: true, image, performer: "adam-key" }),
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
  assert.deepEqual(worker.resources, {
    requests: { cpu: "2000m", memory: "4Gi" },
    limits: { cpu: "8000m", memory: "8Gi" },
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
