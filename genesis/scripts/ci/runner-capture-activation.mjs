// CI-only read wrapper: bind a bootstrap receipt to the current reviewed grant window.
import assert from "node:assert/strict";
import { withClusterTransport } from "./runner-cluster-transport.mjs";
import { inspectProviderBootstrap } from "./runner-provider-diagnostics.mjs";
import { readRecoveryProof } from "./runner-recovery-diagnostics.mjs";
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
// Observe only an existing incarnation; restart races never authorize mutation.
export async function readPersistedLogs({
  execute,
  args,
  options,
  pod,
  namespace,
  expectedImage,
  expectedUid,
  now = () => performance.now(),
  pause = (ms) => new Promise((r) => setTimeout(r, ms)),
  budgetMs = 360000,
}) {
  assert(budgetMs > 0 && budgetMs <= 360000);
  assert(expectedImage && expectedImage.includes("@sha256:"));
  const deadline = now() + budgetMs;
  assert(expectedUid, "original pod identity unavailable");
  const uid = expectedUid;
  const projection =
    '{.metadata.uid}{"\\n"}{.spec.containers[?(@.name=="compute-worker")].image}{"\\n"}{.status.containerStatuses[?(@.name=="compute-worker")].state.running.startedAt}{"\\n"}{.status.containerStatuses[?(@.name=="compute-worker")].restartCount}{"\\n"}{.status.containerStatuses[?(@.name=="compute-worker")].containerID}';
  const call = (argv) => {
    const remaining = deadline - now();
    assert(remaining > 0, "persisted-log observation deadline");
    // The fixed exec now includes two sequential five-second read-only HTTP calls.
    // Pod observations stay short; neither arm extends the shared deadline.
    const isExec = argv[0] === "exec";
    return execute(
      "kubectl",
      [
        `--request-timeout=${Math.max(1, Math.floor(Math.min(isExec ? 12000 : 3000, remaining)))}ms`,
        ...argv,
      ],
      {
        ...options,
        timeout: Math.max(
          1,
          Math.floor(Math.min(isExec ? 13000 : 4000, remaining)),
        ),
        killSignal: "SIGKILL",
      },
    );
  };
  while (now() < deadline) {
    try {
      const fields = String(
        call([
          "get",
          "pod",
          pod,
          "-n",
          namespace,
          "-o",
          `jsonpath=${projection}`,
        ]),
      ).split("\n");
      const [observedUid, image, running, restart, containerId] = fields;
      assert(
        observedUid && image === expectedImage,
        "pod identity/image changed or unavailable",
      );
      assert.equal(observedUid, uid, "pod identity changed");
      if (running) {
        const output = call(args);
        const after = String(
          call([
            "get",
            "pod",
            pod,
            "-n",
            namespace,
            "-o",
            `jsonpath=${projection}`,
          ]),
        ).split("\n");
        assert(
          after[0] === uid && after[1] === expectedImage,
          "pod identity/image changed during read",
        );
        return `diagnostic observation only; capturedAt=${new Date().toISOString()} podUid=${uid} restartCount=${restart} containerId=${containerId}\n${output}`;
      }
    } catch (error) {
      const detail = String(error.stderr || error.message || "");
      if (
        !/container not found|container .* terminated|container .* not running/i.test(
          detail,
        )
      )
        throw error;
    }
    const remaining = deadline - now();
    if (remaining > 0) await pause(Math.min(1000, remaining));
  }
  throw new Error("persisted-log transport unavailable: observation deadline");
}
// Failure observations never replace activation or admit a task. Only CI calls this.
export async function captureDiagnostics({
  root,
  workload,
  namespace,
  env = process.env,
  execute = execFileSync,
  reason = "activation-unproven",
  retryLogs = false,
  expectedImage,
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
      "provider-errors.json",
      ["logs", pod, "-n", namespace, "-c", "elohim-node", "--tail=200"],
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
const paths = ['logs/start.log','logs/mongod.log','logs/doorway.log','logs/doorway-b.log','logs/doorway-c.log','logs/matthew.log','logs/jessica.log','logs/james.log','conductors/.sandbox_log','conductors/.sandbox_run_log','conductors/.sandbox_run_log.matthew','conductors/.sandbox_run_log.jessica','conductors/.sandbox_run_log.james'];
for (const path of paths) {
 console.log('--- ' + path);
 let fd;
 try {
  fd = fs.openSync('/var/lib/compute/mesh/' + path, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW);
  const stat = fs.fstatSync(fd); if (!stat.isFile()) { console.log('unavailable: not regular'); continue; }
  console.log('size=' + stat.size + ' mtime=' + stat.mtime.toISOString());
  const length = Math.min(stat.size, 4096), buffer = Buffer.alloc(length);
  fs.readSync(fd, buffer, 0, length, stat.size - length); console.log(buffer.toString());
 } catch { console.log('unavailable'); } finally { if (fd !== undefined) fs.closeSync(fd); }
}
// Hash-only identity observations: never print key, credential, config or DB bytes.
const crypto = require('node:crypto'), base = '/var/lib/compute/mesh/';
function physical(path) {
 let parent = base.slice(0, -1);
 for (const part of path.split('/')) { parent += '/' + part; if (fs.lstatSync(parent).isSymbolicLink()) throw new Error('symlink'); }
 return parent;
}
function observe(path, hash = false) {
 let fd;
 try {
  const target = physical(path), st = fs.lstatSync(target);
  const row = { path, kind: st.isDirectory() ? 'directory' : st.isFile() ? 'file' : 'other', size: st.size, mtime: st.mtime.toISOString() };
  if (hash) {
   if (!st.isFile() || st.size > 65536) throw new Error('not bounded regular file');
   fd = fs.openSync(target, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW);
   const current = fs.fstatSync(fd);
   if (!current.isFile() || current.size > 65536 || current.ino !== st.ino) throw new Error('changed');
   const buffer = Buffer.alloc(current.size);
   if (fs.readSync(fd, buffer, 0, buffer.length, 0) !== buffer.length) throw new Error('short read');
   row.sha256 = crypto.createHash('sha256').update(buffer).digest('hex');
  }
  console.log('identity-observation ' + JSON.stringify(row));
 } catch (error) { console.log('identity-observation ' + JSON.stringify({ path, ...(error.code === 'ENOENT' ? { missing: true } : { unavailable: true }) })); }
 finally { if (fd !== undefined) fs.closeSync(fd); }
}
for (const path of ['logs/start.log','initialization-continuation/start.log','initialization-continuation/apps-before.jsonl','initialization-continuation/apps-after.jsonl','initialization-continuation/complete']) observe(path, true);
observe('initialization-continuation/consumed');
for (const peer of ['matthew','jessica','james']) {
 observe(peer + '/content.db');
 for (const file of ['identity.key','iroh.key','closed-chain-fence/closed-cells.json']) observe(peer + '/' + file, true);
 const directory = peer + '/closed-chain-fence/credentials';
 try {
  const names = fs.readdirSync(physical(directory));
  if (names.length > 16 || names.some(name => !/^[0-9a-f]{78}-[0-9a-f]{78}[.]json$/.test(name))) throw new Error('unexpected credential inventory');
  console.log('identity-observation ' + JSON.stringify({ path: directory, credentialCount: names.length }));
  for (const name of names.sort()) observe(directory + '/' + name, true);
 } catch (error) { console.log('identity-observation ' + JSON.stringify({ path: directory, ...(error.code === 'ENOENT' ? { missing: true } : { unavailable: true }) })); }
}
console.log('recovery-proof ' + JSON.stringify((${readRecoveryProof.toString()})()));
(${inspectProviderBootstrap.toString()})().then(result => console.log('provider-observation ' + JSON.stringify(result)), () => console.log('provider-observation unavailable'));`;
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
  let originalUid;
  for (const [file, args] of reads) {
    let output;
    try {
      const options = {
        env,
        cwd: root,
        encoding: "utf8",
        stdio: ["ignore", "pipe", "pipe"],
        timeout: 20000,
        maxBuffer: 262144,
      };
      output =
        file === "mesh-startup.log" && retryLogs
          ? await readPersistedLogs({
              execute,
              args,
              options,
              pod,
              namespace,
              expectedImage,
              expectedUid: originalUid,
            })
          : execute("kubectl", ["--request-timeout=15s", ...args], options);
    } catch (error) {
      output = `diagnostic read unavailable (exit ${Number.isInteger(error.status) ? error.status : "unknown"})\n${error.stdout || ""}${error.stderr || error.message || ""}`;
    }
    if (
      file === "state.txt" &&
      !String(output).startsWith("diagnostic read unavailable")
    )
      originalUid = String(output).split("\n")[0];
    if (file === "provider-errors.json") {
      // Server logs can contain arbitrary request data: publish categories only.
      const errors = String(output)
        .split("\n")
        .filter((line) => /Request error/i.test(line));
      const classifiers = [
        [
          /capability|cap.?grant|cap.?secret|unauthoriz/i,
          "native-authorization",
        ],
        [/timed? ?out|timeout|deadline/i, "native-timeout"],
        [
          /deserial|serializ|decode|missing field|invalid type/i,
          "native-wire-shape",
        ],
        [/sqlite|database|locked/i, "database"],
        [/not found|missing/i, "native-record-missing"],
        [/connect|socket|transport/i, "native-transport"],
      ];
      output =
        JSON.stringify({
          kind: "provider-log-observation",
          requestErrorCount: errors.length,
          readUnavailable: String(output).startsWith(
            "diagnostic read unavailable",
          ),
          categories: errors
            .slice(-16)
            .map(
              (line) =>
                classifiers.find(([pattern]) => pattern.test(line))?.[1] ||
                "request-error-unclassified",
            ),
          grantRequestCorrelationProven: false,
        }) + "\n";
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
  const [workload, namespace, output = "compute-worker-evidence", extra] =
    process.argv.slice(2);
  assert(
    !extra &&
      /^[a-z0-9][a-z0-9-]*$/.test(workload) &&
      /^[a-z0-9][a-z0-9-]*$/.test(namespace),
  );
  const evidenceDirectory = resolve(
    process.env.WORKSPACE || ".",
    output,
    `${namespace}--${workload}`,
  );
  for (const name of ["activation.json", "status.txt"])
    rmSync(join(evidenceDirectory, name), { force: true });
  await withClusterTransport(async (env) => {
    try {
      captureActivation({
        root: resolve(env.WORKSPACE || "."),
        env,
        workload,
        namespace,
        output,
      });
      console.log(
        "compute worker receipt: READY; current declaration window verified, native status checked separately",
      );
    } catch (error) {
      try {
        const diagnosticDirectory = await captureDiagnostics({
          root: resolve(env.WORKSPACE || "."),
          env,
          workload,
          namespace,
          reason: error.activationReason,
          retryLogs: true,
          expectedImage: JSON.parse(
            readFileSync(
              join(
                resolve(env.WORKSPACE || "."),
                "genesis/orchestrator/data/adam-compute-worker.json",
              ),
            ),
          ).image,
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
  }).catch(() => {
    mkdirSync(evidenceDirectory, { recursive: true, mode: 0o755 });
    writeFileSync(
      join(evidenceDirectory, "status.txt"),
      "REFUSED: projected-cluster-transport-unavailable\n",
      { mode: 0o644 },
    );
    console.error(
      "REFUSED compute worker activation: projected cluster transport unavailable",
    );
    process.exitCode = 1;
  });
}
