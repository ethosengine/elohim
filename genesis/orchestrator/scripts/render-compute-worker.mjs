#!/usr/bin/env node
// Packaging projection only: task/runtime code has no Kubernetes dependency.
import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import YAML from "yaml";

export function renderWorker(source, config) {
  if (!config.enabled) return source;
  if (
    !/^.+@sha256:[a-f0-9]{64}$/.test(config.image || "") ||
    !config.performer
  ) {
    throw new Error(
      "Enabled compute worker requires immutable image digest and Adam agent key",
    );
  }
  const docs = YAML.parseAllDocuments(source);
  const document = docs.find((doc) => doc.get("kind") === "StatefulSet");
  if (!document) throw new Error("Adam storage StatefulSet missing");
  const manifest = document.toJS();
  if (
    manifest.spec.template.metadata.labels["elohim-human"] !== "adam-firstman"
  ) {
    throw new Error("Compute worker slice is assigned only to Adam");
  }
  const pod = manifest.spec.template.spec;
  pod.automountServiceAccountToken = false;
  const storage = pod.containers.find(
    (container) => container.name === "elohim-node",
  );
  if (!storage) throw new Error("Adam storage container missing");
  const localToken = {
    name: "ELOHIM_COMPUTE_LOCAL_TOKEN",
    valueFrom: {
      secretKeyRef: {
        name: `${manifest.metadata.name}-compute-local`,
        key: "token",
      },
    },
  };
  storage.env ??= [];
  storage.env = storage.env.filter(
    (variable) =>
      !["ELOHIM_COMPUTE_LOCAL_API", "ELOHIM_COMPUTE_LOCAL_TOKEN"].includes(
        variable.name,
      ),
  );
  storage.env.push(
    { name: "ELOHIM_COMPUTE_LOCAL_API", value: "1" },
    localToken,
  );
  pod.containers = pod.containers.filter(
    (container) => container.name !== "compute-worker",
  );
  pod.containers.push({
    name: "compute-worker",
    image: config.image,
    command: ["node", "/opt/compute/worker.mjs"],
    env: [
      localToken,
      { name: "COMPUTE_API_URL", value: "http://127.0.0.1:8090" },
      { name: "COMPUTE_PERFORMER", value: config.performer },
      { name: "COMPUTE_RUNTIME_IMAGE", value: config.image.split("@")[1] },
      { name: "COMPUTE_WORKER_ROOT", value: "/var/lib/compute" },
      { name: "TMPDIR", value: "/var/lib/compute/tmp" },
    ],
    resources: {
      requests: { cpu: "2000m", memory: "4Gi" },
      limits: { cpu: "8000m", memory: "8Gi" },
    },
    securityContext: {
      allowPrivilegeEscalation: false,
      readOnlyRootFilesystem: true,
      runAsUser: 0,
      capabilities: { drop: ["ALL"], add: ["SETUID", "SETGID", "CHOWN"] },
    },
    volumeMounts: [{ name: "compute-work", mountPath: "/var/lib/compute" }],
  });
  // Explicit standalone PVC avoids mutating immutable volumeClaimTemplates.
  pod.volumes ??= [];
  pod.volumes = pod.volumes.filter((volume) => volume.name !== "compute-work");
  pod.volumes.push({
    name: "compute-work",
    persistentVolumeClaim: { claimName: `${manifest.metadata.name}-compute` },
  });
  document.contents = new YAML.Document(manifest).contents;
  const pvc = new YAML.Document({
    apiVersion: "v1",
    kind: "PersistentVolumeClaim",
    metadata: {
      name: `${manifest.metadata.name}-compute`,
      namespace: manifest.metadata.namespace,
    },
    spec: {
      accessModes: ["ReadWriteOnce"],
      storageClassName: config.storageClass || "openebs-hostpath",
      resources: { requests: { storage: "24Gi" } },
    },
  });
  return [...docs, pvc]
    .map((doc) => doc.toString().replace(/^---\n/, ""))
    .join("---\n");
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  const [manifest, config] = process.argv.slice(2);
  const source = await readFile(manifest, "utf8");
  const options = JSON.parse(await readFile(config, "utf8"));
  const output = renderWorker(source, options);
  if (output !== source) await writeFile(manifest, output);
}
