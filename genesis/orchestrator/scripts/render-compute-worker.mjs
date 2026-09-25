#!/usr/bin/env node
// Packaging projection only: task/runtime code has no Kubernetes dependency.
import { readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import YAML from "yaml";

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
// genesis/orchestrator/scripts -> repo root, three levels up.
const DEFAULT_REPO_ROOT = resolve(SCRIPT_DIR, "..", "..", "..");
const MI = 1024 * 1024;

// Mirrors bridges/k8s/src/lib.rs `quantity(value: &str, memory: bool)` exactly — same
// vocabulary (Ti/Gi/Mi/Ki for memory, `m`-suffixed or plain whole-units-times-1000 for
// cpu millis), same exact-integer-math refusal of anything that would round or overflow.
// `kind` stands in for the Rust function's `memory: bool` parameter.
const MEMORY_UNITS = [
  ["Ti", 1024n ** 4n],
  ["Gi", 1024n ** 3n],
  ["Mi", 1024n ** 2n],
  ["Ki", 1024n],
];

export function quantity(value, kind) {
  if (kind !== "memory" && kind !== "cpu") {
    throw new Error(`unknown quantity kind: ${kind}`);
  }
  let number;
  let multiplier;
  if (kind === "memory") {
    const unit = MEMORY_UNITS.find(([suffix]) => value.endsWith(suffix));
    if (!unit) throw new Error(`unsupported memory quantity: ${value}`);
    number = value.slice(0, value.length - unit[0].length);
    multiplier = unit[1];
  } else if (value.endsWith("m")) {
    number = value.slice(0, -1);
    multiplier = 1n;
  } else {
    number = value;
    multiplier = 1000n;
  }
  const dot = number.indexOf(".");
  const whole = dot === -1 ? number : number.slice(0, dot);
  const fraction = dot === -1 ? "" : number.slice(dot + 1);
  const digits = whole + fraction;
  if (
    whole.length === 0 ||
    digits.length === 0 ||
    ![...digits].every((char) => char >= "0" && char <= "9")
  ) {
    throw new Error(`invalid quantity: ${value}`);
  }
  const denominator = 10n ** BigInt(fraction.length);
  const scaled = BigInt(digits) * multiplier;
  if (scaled % denominator !== 0n) {
    throw new Error(`fractional base unit: ${value}`);
  }
  const result = scaled / denominator;
  if (result > BigInt(Number.MAX_SAFE_INTEGER)) {
    throw new Error(`quantity overflow: ${value}`);
  }
  return Number(result);
}

function miString(bytes) {
  if (bytes % MI !== 0) {
    throw new Error(`quantity ${bytes} is not a whole number of Mi`);
  }
  return `${bytes / MI}Mi`;
}

function displayMi(bytes) {
  return `${Math.round(bytes / MI)}Mi`;
}

// Sums every container's requests/limits (memory bytes, cpu millis) in a pod spec.
// initContainers are deliberately excluded — they never run concurrently with the
// steady-state containers this Σ refusal is bounding.
function accumulate(pod, totals) {
  for (const container of pod.containers ?? []) {
    const requests = container.resources?.requests ?? {};
    const limits = container.resources?.limits ?? {};
    if (requests.memory) totals.requestsMemory += quantity(requests.memory, "memory");
    if (requests.cpu) totals.requestsCpu += quantity(requests.cpu, "cpu");
    if (limits.memory) totals.limitsMemory += quantity(limits.memory, "memory");
    if (limits.cpu) totals.limitsCpu += quantity(limits.cpu, "cpu");
  }
}

function findStatefulSet(source, label) {
  const docs = YAML.parseAllDocuments(source);
  const document = docs.find((doc) => doc.get("kind") === "StatefulSet");
  if (!document) throw new Error(`${label} StatefulSet missing`);
  return { docs, document };
}

export function renderWorker(source, config, overrides = {}) {
  if (!config.enabled) return source;
  if (
    !/^.+@sha256:[a-f0-9]{64}$/.test(config.image || "") ||
    !config.performer
  ) {
    throw new Error(
      "Enabled compute worker requires immutable image digest and Adam agent key",
    );
  }
  return renderEnabled(source, config, overrides);
}

async function renderEnabled(source, config, overrides) {
  if (
    !config.slice ||
    typeof config.slice.cpuMillis !== "number" ||
    typeof config.slice.memoryBytes !== "number"
  ) {
    throw new Error(
      "Enabled compute worker requires a declared slice {cpuMillis, memoryBytes} on config.slice — S4a2 declare the worker slice",
    );
  }
  const { docs, document } = findStatefulSet(source, "Adam storage");
  const manifest = document.toJS();
  if (
    manifest.spec.template.metadata.labels["elohim-human"] !== "adam-firstman"
  ) {
    throw new Error("Compute worker slice is assigned only to Adam");
  }
  const pod = manifest.spec.template.spec;

  // Σ refusal (S4a1, R6): the worker's request/limit is rendered INSIDE adam's declared
  // envelope.bound — sourced from the deployments.json pin (adam.runtimeManifest.path)
  // and every sibling container's rendered/declared resources (storage StatefulSet =
  // `source` itself; conductor StatefulSet = adam.conductorManifest), never hardcoded.
  const repoRoot = overrides.repoRoot ?? DEFAULT_REPO_ROOT;
  const deploymentsPath = resolve(repoRoot, "genesis/orchestrator/data/deployments.json");
  const deployments = JSON.parse(await readFile(deploymentsPath, "utf8"));
  const adam = (deployments.humans ?? []).find((human) => human.name === "adam");
  if (!adam) throw new Error(`adam entry missing from ${deploymentsPath}`);
  if (!adam.runtimeManifest?.path) {
    throw new Error(`${deploymentsPath}: adam.runtimeManifest.path missing`);
  }
  if (!adam.conductorManifest) {
    throw new Error(`${deploymentsPath}: adam.conductorManifest missing`);
  }
  const runtimeManifestPath = resolve(repoRoot, adam.runtimeManifest.path);
  const runtimeManifest = JSON.parse(await readFile(runtimeManifestPath, "utf8"));
  const bound = runtimeManifest.envelope?.bound;
  if (
    !bound ||
    typeof bound.memory_bytes !== "number" ||
    typeof bound.cpu_millis !== "number"
  ) {
    throw new Error(`${runtimeManifestPath}: envelope.bound missing memory_bytes/cpu_millis`);
  }
  const conductorManifestPath = resolve(repoRoot, adam.conductorManifest);
  const conductorSource = await readFile(conductorManifestPath, "utf8");
  const conductorPod = findStatefulSet(conductorSource, "Adam conductor").document.toJS()
    .spec.template.spec;

  const totals = { requestsMemory: 0, requestsCpu: 0, limitsMemory: 0, limitsCpu: 0 };
  accumulate(pod, totals); // storage: elohim-node (compute-worker not pushed yet)
  accumulate(conductorPod, totals); // conductor: elohim-conductor + ws-proxy
  totals.requestsMemory += config.slice.memoryBytes;
  totals.limitsMemory += config.slice.memoryBytes;
  totals.requestsCpu += config.slice.cpuMillis;
  totals.limitsCpu += config.slice.cpuMillis;

  const refuse = (label, total, boundValue, format) => {
    if (total > boundValue) {
      throw new Error(
        `Σ ${label} ${format(total)} > envelope.bound ${format(boundValue)}`,
      );
    }
  };
  refuse("limits", totals.limitsMemory, bound.memory_bytes, displayMi);
  refuse("limits", totals.limitsCpu, bound.cpu_millis, (v) => `${v}m`);
  refuse("requests", totals.requestsMemory, bound.memory_bytes, displayMi);
  refuse("requests", totals.requestsCpu, bound.cpu_millis, (v) => `${v}m`);

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
  const sliceCpu = `${config.slice.cpuMillis}m`;
  const sliceMemory = miString(config.slice.memoryBytes);
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
      { name: "COMPUTE_SLICE_CPU_MILLIS", value: String(config.slice.cpuMillis) },
      { name: "COMPUTE_SLICE_MEMORY_BYTES", value: String(config.slice.memoryBytes) },
    ],
    resources: {
      requests: { cpu: sliceCpu, memory: sliceMemory },
      limits: { cpu: sliceCpu, memory: sliceMemory },
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
  const output = await renderWorker(source, options);
  if (output !== source) await writeFile(manifest, output);
}
