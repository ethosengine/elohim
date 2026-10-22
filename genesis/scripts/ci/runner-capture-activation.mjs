// CI-only read wrapper: bind a bootstrap receipt to the current reviewed grant window.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import {
  readFileSync,
  writeFileSync,
  rmSync,
  mkdirSync,
  chmodSync,
} from "node:fs";
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
    // This runs after rollout: prove read transport before spending a bootstrap budget.
    const observedPod = execute(
      "kubectl",
      [
        "--request-timeout=15s",
        "get",
        "pod",
        `${workload}-0`,
        "-n",
        namespace,
        "-o",
        "name",
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
    assert.equal(
      observedPod.trim(),
      `pod/${workload}-0`,
      "provider pod read transport unproven",
    );
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
    // Selected public proof must be readable by Jenkins' separate agent UID.
    chmodSync(resolve(root, output), 0o755);
    chmodSync(join(resolve(root, output), `${namespace}--${workload}`), 0o755);
    writeFileSync(artifact, JSON.stringify(result, null, 2) + "\n", {
      mode: 0o644,
    });
    chmodSync(artifact, 0o644);
    writeFileSync(
      statusFile,
      "READY: current declaration window verified; native status checked separately\n",
      { mode: 0o644 },
    );
    chmodSync(statusFile, 0o644);
    return result;
  } catch (error) {
    let reason = "declaration-window-or-incarnation-unproven";
    try {
      const original = readFileSync(statusFile, "utf8");
      if (
        original.includes("startup budget") &&
        original.includes("worker bootstrap or grant receipt unavailable")
      )
        reason = "bootstrap-or-grant-receipt-timeout";
      else if (
        original.includes("startup budget") &&
        original.includes(
          "worker missing, not Ready, terminating, or image differs",
        )
      )
        reason = "worker-state-or-image-timeout";
      else if (
        original.includes(
          "public receipt, running worker, or enforced capacity differs",
        )
      )
        reason = "receipt-worker-or-capacity-mismatch";
    } catch {
      /* No original capture status exists for earlier validation refusals. */
    }
    error.activationReason = reason;
    rmSync(artifact, { force: true });
    rmSync(statusFile, { force: true });
    throw error;
  }
}
// Failure observations never replace activation or admit a task. Only CI calls this.
export function captureDiagnostics({
  root,
  workload,
  namespace,
  env = process.env,
  execute = execFileSync,
  reason = "activation-unproven",
}) {
  assert(
    /^[a-z0-9][a-z0-9-]*$/.test(workload) &&
      /^[a-z0-9][a-z0-9-]*$/.test(namespace),
  );
  const directory = join(
    root,
    "rollout-evidence",
    `${namespace}--compute-worker--${workload}`,
  );
  mkdirSync(directory, { recursive: true, mode: 0o755 });
  chmodSync(join(root, "rollout-evidence"), 0o755);
  chmodSync(directory, 0o755);
  const knownReasons = new Set([
    "activation-unproven",
    "declaration-window-or-incarnation-unproven",
    "bootstrap-or-grant-receipt-timeout",
    "worker-state-or-image-timeout",
    "receipt-worker-or-capacity-mismatch",
  ]);
  writeFileSync(
    join(directory, "status.txt"),
    `REFUSED: ${knownReasons.has(reason) ? reason : "activation-unproven"}\n`,
    { mode: 0o644 },
  );
  chmodSync(join(directory, "status.txt"), 0o644);
  const redact = (input) => {
    let text = String(input || "").slice(-65536);
    for (const [name, value] of Object.entries(env)) {
      if (
        /(TOKEN|PASSWORD|SECRET|KEY)$/.test(name) &&
        typeof value === "string" &&
        value.length >= 8
      )
        text = text.split(value).join("[REDACTED]");
    }
    return text.replace(
      /eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+/g,
      "[REDACTED JWT]",
    );
  };
  const pod = `${workload}-0`;
  const state =
    '{.metadata.uid}{"\\n"}{.status.phase}{"\\n"}{.status.containerStatuses[?(@.name=="compute-worker")].ready}{"\\n"}{.status.containerStatuses[?(@.name=="compute-worker")].restartCount}{"\\n"}{.status.containerStatuses[?(@.name=="compute-worker")].state}{"\\n"}{.status.containerStatuses[?(@.name=="compute-worker")].lastState}';
  const reads = [
    [
      "state.txt",
      ["get", "pod", pod, "-n", namespace, "-o", `jsonpath=${state}`],
    ],
    [
      "current.log",
      ["logs", pod, "-n", namespace, "-c", "compute-worker", "--tail=100"],
    ],
    [
      "previous.log",
      [
        "logs",
        pod,
        "-n",
        namespace,
        "-c",
        "compute-worker",
        "--previous",
        "--tail=100",
      ],
    ],
  ];
  // Detached services write files, so container stdout alone can hide the failed station.
  const meshTail = `const fs = require('node:fs');
const paths = ['logs/start.log','logs/mongod.log','logs/doorway.log','logs/doorway-b.log','logs/doorway-c.log','logs/matthew.log','logs/jessica.log','logs/james.log','conductors/.sandbox_run_log','conductors/.sandbox_run_log.matthew','conductors/.sandbox_run_log.jessica','conductors/.sandbox_run_log.james'];
for (const path of paths) {
 console.log('--- ' + path);
 let fd;
 try {
  fd = fs.openSync('/var/lib/compute/mesh/' + path, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW);
  const stat = fs.fstatSync(fd); if (!stat.isFile()) continue;
  const length = Math.min(stat.size, 4096), buffer = Buffer.alloc(length);
  fs.readSync(fd, buffer, 0, length, stat.size - length); console.log(buffer.toString());
 } catch { console.log('unavailable'); } finally { if (fd !== undefined) fs.closeSync(fd); }
}`;
  reads.push([
    "mesh-startup.log",
    [
      "exec",
      pod,
      "-n",
      namespace,
      "-c",
      "compute-worker",
      "--",
      "/usr/local/bin/node",
      "-e",
      meshTail,
    ],
  ]);
  for (const [file, args] of reads) {
    let output;
    try {
      output = execute("kubectl", ["--request-timeout=15s", ...args], {
        env,
        cwd: root,
        encoding: "utf8",
        stdio: ["ignore", "pipe", "pipe"],
        timeout: 20000,
        maxBuffer: 262144,
      });
    } catch (error) {
      output = `diagnostic read unavailable (exit ${Number.isInteger(error.status) ? error.status : "unknown"})\n${error.stdout || ""}${error.stderr || ""}`;
    }
    writeFileSync(join(directory, file), redact(output), { mode: 0o644 });
    chmodSync(join(directory, file), 0o644);
  }
  return directory;
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
  } catch (error) {
    try {
      const [workload, namespace] = process.argv.slice(2);
      const diagnosticDirectory = captureDiagnostics({
        root: resolve(process.env.WORKSPACE || "."),
        workload,
        namespace,
        reason: error.activationReason,
      });
      for (const name of [
        "status.txt",
        "state.txt",
        "current.log",
        "previous.log",
        "mesh-startup.log",
      ]) {
        console.error(
          `compute worker diagnostic ${name}:\n${readFileSync(join(diagnosticDirectory, name), "utf8")}`,
        );
      }
    } catch {
      /* Diagnostic failure never replaces the original refusal. */
    }
    console.error(
      "REFUSED compute worker activation: declaration window or incarnation unproven",
    );
    process.exitCode = 1;
  }
}
