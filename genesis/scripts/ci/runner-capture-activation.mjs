// CI-only read wrapper: bind a bootstrap receipt to the current reviewed grant window.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync, rmSync } from "node:fs";
import { resolve, join } from "node:path";
import { pathToFileURL } from "node:url";
import { validateGrantDeclaration } from "../../agentic/compute/grant-bootstrap.mjs";

export function verifyDeclaration(
  activation,
  declaration,
  pod,
  expected,
  now = Date.now(),
) {
  validateGrantDeclaration(expected, now);
  assert.deepEqual(
    declaration,
    expected,
    "running declaration differs from reviewed grant",
  );
  const w = activation.worker;
  assert.equal(pod.uid, w.podUid);
  assert.equal(pod.containerId, w.containerId);
  assert.equal(pod.restartCount, w.restartCount);
  assert.equal(pod.image, w.image);
  assert.equal(activation.provider, declaration.provider);
  assert.equal(activation.recipient, declaration.recipient);
  const { issuedAt, validFrom, validUntil } = declaration.grant;
  return {
    ...activation,
    issuedAt,
    validFrom,
    validUntil,
    declarationCheckedAt: new Date(now).toISOString(),
    stateEvidence:
      "bootstrap-receipt-observation; native status requires signed grant-status",
    nativeStatusChecked: false,
  };
}
export function captureActivation({
  root,
  workload,
  namespace,
  output = "compute-worker-evidence",
  env = process.env,
  execute = execFileSync,
  now = Date.now,
}) {
  assert(
    /^[a-z0-9][a-z0-9-]*$/.test(workload) &&
      /^[a-z0-9][a-z0-9-]*$/.test(namespace),
  );
  const artifact = resolve(
    root,
    output,
    `${namespace}--${workload}`,
    "activation.json",
  );
  const statusFile = join(
    resolve(root, output, `${namespace}--${workload}`),
    "status.txt",
  );
  rmSync(artifact, { force: true });
  rmSync(statusFile, { force: true });
  try {
    const worker = JSON.parse(
      readFileSync(
        join(root, "genesis/orchestrator/data/adam-compute-worker.json"),
      ),
    );
    assert.equal(worker.enabled, true);
    validateGrantDeclaration(worker.runnerGrant, now());
    const captureEnv = {
      ...env,
      COMPUTE_WORKER_ENABLED: "true",
      COMPUTE_WORKER_GRANT_ENABLED: "true",
      COMPUTE_WORKER_PROVIDER: worker.runnerGrant.provider,
      COMPUTE_WORKER_RECIPIENT: worker.runnerGrant.recipient,
      COMPUTE_WORKER_IMAGE: worker.image,
      COMPUTE_WORKER_CPU_MILLIS: String(worker.slice?.cpuMillis || ""),
      COMPUTE_WORKER_MEMORY_BYTES: String(worker.slice?.memoryBytes || ""),
      COMPUTE_WORKER_STARTUP_SECS:
        env.COMPUTE_WORKER_STARTUP_SECS ||
        String(worker.startupBudgetSeconds || 900),
    };
    execute(
      "bash",
      [
        join(root, "scripts/ci/capture-compute-worker-grant.sh"),
        workload,
        namespace,
        resolve(root, output),
      ],
      {
        env: captureEnv,
        cwd: root,
        stdio: ["ignore", "pipe", "pipe"],
        timeout: 920000,
      },
    );
    const activation = JSON.parse(readFileSync(artifact));
    rmSync(artifact, { force: true }); // Never leave the weaker intermediate artifact on refusal.
    const selected =
      '{.metadata.uid}{"\\n"}{.status.containerStatuses[?(@.name=="compute-worker")].containerID}{"\\n"}{.status.containerStatuses[?(@.name=="compute-worker")].restartCount}{"\\n"}{.spec.containers[?(@.name=="compute-worker")].image}{"\\n"}{.spec.containers[?(@.name=="compute-worker")].env[?(@.name=="COMPUTE_RUNNER_GRANT_JSON")].value}';
    const raw = execute(
      "kubectl",
      [
        "--request-timeout=15s",
        "get",
        "pod",
        `${workload}-0`,
        "-n",
        namespace,
        "-o",
        `jsonpath=${selected}`,
      ],
      {
        env,
        cwd: root,
        encoding: "utf8",
        stdio: ["ignore", "pipe", "pipe"],
        timeout: 20000,
        maxBuffer: 65536,
      },
    );
    const [uid, containerId, restartCount, image, ...json] = raw.split("\n");
    const declaration = JSON.parse(json.join("\n"));
    const result = verifyDeclaration(
      activation,
      declaration,
      { uid, containerId, restartCount: Number(restartCount), image },
      worker.runnerGrant,
      now(),
    );
    assert.equal(image, worker.image);
    assert.equal(worker.performer, declaration.provider);
    writeFileSync(artifact, JSON.stringify(result, null, 2) + "\n", {
      mode: 0o600,
    });
    writeFileSync(
      statusFile,
      "READY: current declaration window verified; native status checked separately\n",
      { mode: 0o600 },
    );
    return result;
  } catch (error) {
    rmSync(artifact, { force: true });
    rmSync(statusFile, { force: true });
    throw error;
  }
}
if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  try {
    const [workload, namespace, output, extra] = process.argv.slice(2);
    assert(!extra);
    captureActivation({
      root: resolve(process.env.WORKSPACE || "."),
      workload,
      namespace,
      output,
    });
    console.log(
      "compute worker receipt: READY; current declaration window verified, native status checked separately",
    );
  } catch {
    console.error(
      "REFUSED compute worker activation: declaration window or incarnation unproven",
    );
    process.exitCode = 1;
  }
}
