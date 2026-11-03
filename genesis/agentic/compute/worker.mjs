#!/usr/bin/env node
import { copyFile, mkdir, readFile, rm, chmod } from "node:fs/promises";
import { randomUUID } from "node:crypto";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { isDeepStrictEqual } from "node:util";
import { api, delay, key, readJson, run, save } from "./common.mjs";
import {
  leaseSeconds,
  materialize,
  payloadClient,
  publishFile,
} from "./payloads.mjs";
import { cleanWorker } from "./retention.mjs";

// The guest sees a deliberately small environment; signing belongs to local storage.
export function guestEnvironment(env) {
  return Object.fromEntries(
    ["PATH", "LD_LIBRARY_PATH", "COMPUTE_RUNTIME_IMAGE", "TMPDIR"]
      .filter((name) => env[name] !== undefined)
      .map((name) => [name, env[name]]),
  );
}

const artifactIdentity = ({ cid, sha256, bytes } = {}) => ({
  cid,
  sha256,
  bytes,
});

// Submission adds an invocation nonce and transport chunk lists. Neither changes the stage
// Adam is willing to run; every other field is provider-reconstructed and must match exactly.
export function comparableStageEnvelope(envelope) {
  const { invocation: _invocation, ...comparable } = envelope;
  return {
    ...comparable,
    binary: artifactIdentity(envelope.binary),
    dna: artifactIdentity(envelope.dna),
  };
}

function declined(message, reason = "invalid-artifact") {
  return Object.assign(new Error(message), { computeDeclineReason: reason });
}

function stageFeatureForProject(features, project) {
  return features.find((feature) => {
    const file = feature.split("/").at(-1);
    return project === `a2o-stage:${file.replace(/\.feature$/, "")}`;
  });
}

async function rebuildProviderStage({
  task,
  root,
  dir,
  env,
  executor,
  execute,
  buildStage,
}) {
  const sourceCommit = env.COMPUTE_SOURCE_COMMIT;
  if (!/^[0-9a-f]{40,64}$/.test(sourceCommit || ""))
    throw declined(
      "provider stage refused: pinned COMPUTE_SOURCE_COMMIT is absent",
    );
  const stateRoot = env.COMPUTE_STATE_ROOT || dirname(root);
  const repoRoot = join(stateRoot, "repos", sourceCommit);
  const allowlistPath = join(
    repoRoot,
    "genesis",
    "agentic",
    "compute",
    "provider-stages.json",
  );
  let allowlist;
  try {
    allowlist = JSON.parse(await readFile(allowlistPath, "utf8"));
  } catch (error) {
    throw declined(
      `provider stage refused: provider-stages.json unreadable: ${error.message}`,
    );
  }
  if (
    !Array.isArray(allowlist.features) ||
    allowlist.features.some(
      (feature) =>
        typeof feature !== "string" ||
        !/^features\/[a-zA-Z0-9_./-]+\.feature$/.test(feature) ||
        feature.split("/").includes(".."),
    )
  )
    throw declined(
      "provider stage refused: provider-stages.json has an invalid feature list",
    );
  // A task names its feature by basename (a2o-stage:<basename>), so two allowlisted features
  // sharing a basename would resolve to whichever is listed first. Refuse that list outright.
  const basenames = allowlist.features.map((feature) =>
    feature.split("/").at(-1),
  );
  if (new Set(basenames).size !== basenames.length)
    throw declined(
      "provider stage refused: provider-stages.json lists two features with the same file name",
    );
  const feature = stageFeatureForProject(allowlist.features, task.project);
  if (!feature)
    throw declined(
      `provider stage refused: ${task.project || "unnamed project"} is not listed in provider-stages.json`,
    );

  let builder = buildStage;
  let built;
  try {
    if (!builder) {
      const modulePath = join(
        repoRoot,
        "genesis",
        "agentic",
        "compute",
        "stage",
        "build-stage-task.mjs",
      );
      builder = (await import(pathToFileURL(modulePath).href)).buildStageTask;
    }
    built = await builder({
      feature,
      requester: task.requester,
      provider: env.COMPUTE_PERFORMER,
      out: join(dir, "provider-stage"),
      runtimeImage: env.COMPUTE_RUNTIME_IMAGE,
      executor,
      repoRoot,
      sourceRepoRoot: repoRoot,
      a2oDir: join(repoRoot, "genesis", "a2o"),
      nodeBin: env.COMPUTE_NODE_BIN || process.execPath,
      justBin:
        env.COMPUTE_JUST_BIN ||
        join(env.COMPUTE_IMAGE_ROOT || "/opt/compute", "mesh-just.sh"),
      writablePaths: [join(repoRoot, "genesis", "a2o", "reports")],
      cpuMillis: Number(env.COMPUTE_SLICE_CPU_MILLIS),
      memoryBytes: Number(env.COMPUTE_SLICE_MEMORY_BYTES),
      timeoutSeconds: Number(env.COMPUTE_STAGE_TIMEOUT_SECONDS || 1800),
      // SUT identity is derived only from the pinned checkout. Runner environment cannot
      // inject E2E_DNA_HASH, A2O_SUT_*, or E2E_HOUSEHOLD_FIXTURE_PATH into this build.
      env: {},
      execute,
      probeRun: false,
    });
  } catch (error) {
    throw declined(
      `provider stage refused while rebuilding ${feature}: ${error.message}`,
    );
  }
  if (
    !isDeepStrictEqual(
      comparableStageEnvelope(task),
      comparableStageEnvelope(built.envelope),
    )
  )
    throw declined(
      `provider stage differs from submitted envelope for ${feature}`,
    );
  return built;
}

export async function executeTask({
  status,
  root,
  request,
  executor = "compute-executor",
  ark = "ark",
  artifactBase,
  execute = run,
  fetchInput = (_base, descriptor, destination) =>
    materialize(
      descriptor,
      destination,
      payloadClient(artifactBase),
      status.taskCid,
      0,
    ),
  env = process.env,
  buildStage,
}) {
  const ref = status.requestActionHash;
  const dir = join(root, "jobs", key(ref));
  await mkdir(dir, { recursive: true, mode: 0o700 });
  const stateFile = join(dir, "state.json");
  let state;
  try {
    state = await readJson(stateFile);
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
    state = { attemptId: randomUUID() };
    await save(stateFile, state);
  }
  if (status.refusal) {
    state.refused = true;
    state.taskCid = status.taskCid;
    state.envelope = status.envelope;
    await save(stateFile, state);
    await Promise.all(
      ["test-binary", "lamad.dna"].map((name) =>
        rm(join(dir, name), { force: true }),
      ),
    );
    return;
  }
  if (status.completion) {
    // Reconcile a lost publication response before count-based cleanup.
    state.receipt = status.completion.receipt;
    state.completed = true;
    await save(stateFile, state);
    return;
  }
  if (status.acceptance && status.acceptance.attemptId !== state.attemptId) {
    throw new Error("Task already accepted by another durable attempt");
  }
  const task = status.envelope;
  // S4a3 — the k8s-rendered sibling container's slice is a hard ceiling on what this
  // worker may accept, independent of the grant's own bounds. Unset env = no ceiling
  // (household/dev worker runs, or a not-yet-declared slice — S4a2).
  const sliceCpuMillis = env.COMPUTE_SLICE_CPU_MILLIS;
  const sliceMemoryBytes = env.COMPUTE_SLICE_MEMORY_BYTES;
  if (
    sliceCpuMillis !== undefined &&
    task.resources?.cpuMillis > Number(sliceCpuMillis)
  ) {
    throw new Error(
      `capacity ceiling: task asks ${task.resources.cpuMillis}m cpu over slice ${sliceCpuMillis}m`,
    );
  }
  if (
    sliceMemoryBytes !== undefined &&
    task.resources?.memoryBytes > Number(sliceMemoryBytes)
  ) {
    throw new Error(
      `capacity ceiling: task asks ${task.resources.memoryBytes} bytes memory over slice ${sliceMemoryBytes} bytes`,
    );
  }
  const taskFile = join(dir, "task.json");
  const providerStage =
    env.COMPUTE_PRIVATE_MESH === "1" || env.COMPUTE_SOURCE_COMMIT
      ? await rebuildProviderStage({
          task,
          root,
          dir,
          env,
          executor,
          execute,
          buildStage,
        })
      : null;
  const runtimeArgs = [
    "run",
    "--task",
    taskFile,
    "--binary",
    join(dir, "test-binary"),
    "--dna",
    join(dir, "lamad.dna"),
    "--root",
    join(root, "runs"),
    "--ark",
    ark,
    "--request-action",
    ref,
    "--grant-action",
    status.grantActionHash,
  ];
  await save(taskFile, task);
  const calculated = (
    await execute(executor, ["cid", taskFile], { env: guestEnvironment(env) })
  ).trim();
  if (calculated !== status.taskCid) throw new Error("Task CID mismatch");
  if (!state.receipt) {
    const runtimePath = (
      await execute(
        executor,
        ["receipt-path", "--root", join(root, "runs"), "--request-action", ref],
        { env: guestEnvironment(env) },
      )
    ).trim();
    try {
      // Existence only; the lock-protected executor validates the authoritative
      // bindings. Interrupted evidence can be sealed even after a grant revocation.
      try {
        await readJson(runtimePath);
      } catch (error) {
        if (error.code !== "ENOENT") throw error;
        await readJson(join(dirname(runtimePath), "started.json"));
      }
      const recovered = JSON.parse(
        await execute(executor, [...runtimeArgs, "--recover-only"], {
          env: guestEnvironment(env),
          timeoutMs: (task.resources?.timeoutSeconds || 3600) * 1000 + 120000,
        }),
      );
      if (
        recovered &&
        (recovered.taskCid !== status.taskCid ||
          recovered.grantAction !== status.grantActionHash)
      ) {
        throw new Error("Recovered runtime receipt binding mismatch");
      }
      if (recovered) {
        state.receipt = recovered;
        await save(stateFile, state);
      }
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
    }
  }
  if (!state.receipt) {
    await request(`/api/v1/compute/tasks/${encodeURIComponent(ref)}/accept`, {
      attemptId: state.attemptId,
    });
    const binary = join(dir, "test-binary");
    const dna = join(dir, "lamad.dna");
    if (providerStage) {
      // The submitted payload is never executed. Descriptor equality above proves these
      // provider-built bytes have the identity the immutable task and receipt will bind.
      await copyFile(providerStage.runnerPath, binary);
      await chmod(binary, 0o755);
      await copyFile(providerStage.dnaPath, dna);
    } else {
      await fetchInput(artifactBase, task.binary, binary);
      await fetchInput(artifactBase, task.dna, dna);
    }
    await request(
      `/api/v1/compute/tasks/${encodeURIComponent(ref)}/authorize-launch`,
      { attemptId: state.attemptId },
    );
    const output = await execute(
      executor,
      [
        "run",
        "--task",
        taskFile,
        "--binary",
        binary,
        "--dna",
        dna,
        "--root",
        join(root, "runs"),
        "--ark",
        ark,
        "--request-action",
        ref,
        "--grant-action",
        status.grantActionHash,
      ],
      {
        env: guestEnvironment(env),
        timeoutMs: (task.resources?.timeoutSeconds || 3600) * 1000 + 120000,
      },
    );
    state.receipt = JSON.parse(output);
    await save(stateFile, state);
    await Promise.all([rm(binary, { force: true }), rm(dna, { force: true })]);
  }
  if (!state.logsPublished && state.receipt.logs?.length) {
    const runtimeReceipt = (
      await execute(
        executor,
        ["receipt-path", "--root", join(root, "runs"), "--request-action", ref],
        { env: guestEnvironment(env) },
      )
    ).trim();
    const payload = payloadClient(artifactBase);
    for (let index = 0; index < state.receipt.logs.length; index++) {
      const log = state.receipt.logs[index];
      if (!["stdout.log", "stderr.log", "witness.json"].includes(log.name))
        throw new Error("Invalid runtime log name");
      state.receipt.logs[index] = await publishFile(
        join(dirname(runtimeReceipt), "payload", log.name),
        log,
        {
          payload,
          executor,
          retention: leaseSeconds(task.retention),
          execute,
          owner: status.taskCid,
        },
      );
    }
    state.logsPublished = true;
    await save(stateFile, state);
  }
  const receiptFile = join(dir, "receipt.json");
  await save(receiptFile, state.receipt);
  const receiptCid = (
    await execute(executor, ["cid", receiptFile], {
      env: guestEnvironment(env),
    })
  ).trim();
  await request(`/api/v1/compute/tasks/${encodeURIComponent(ref)}/complete`, {
    attemptId: state.attemptId,
    receiptCid,
    receipt: state.receipt,
  });
  state.completed = true;
  await save(stateFile, state);
  await execute(executor, ["cleanup", "--root", join(root, "runs")], {
    env: guestEnvironment(env),
  });
}

export function taskDeclineReason(error) {
  return (
    error.computeDeclineReason ||
    (/API 403/.test(error.message)
      ? "compute-grant-refused"
      : /mismatch|invalid artifact|digest/i.test(error.message)
        ? "invalid-artifact"
        : /capacity|quota|ceiling/i.test(error.message)
          ? "capacity-unavailable"
          : /runtime image|runtime.*incompatible/i.test(error.message)
            ? "runtime-incompatible"
            : null)
  );
}

export async function executeOrDecline(options) {
  try {
    await executeTask(options);
    return { declined: false };
  } catch (error) {
    console.error(
      `Task ${key(options.status.requestActionHash)}: ${error.message}`,
    );
    const reason = taskDeclineReason(error);
    if (!reason) throw error;
    const local = await readJson(
      join(
        options.root,
        "jobs",
        key(options.status.requestActionHash),
        "state.json",
      ),
    );
    await options.request(
      `/api/v1/compute/tasks/${encodeURIComponent(options.status.requestActionHash)}/decline`,
      { attemptId: local.attemptId, reason },
    );
    return { declined: true, reason, message: error.message };
  }
}

export async function main() {
  const root = resolve(process.env.COMPUTE_WORKER_ROOT || "/var/lib/compute");
  await mkdir(root, { recursive: true, mode: 0o755 });
  await chmod(root, 0o755);
  await mkdir(join(root, "runs"), { recursive: true, mode: 0o755 });
  await mkdir(join(root, "tmp"), { recursive: true, mode: 0o700 });
  const performer = process.env.COMPUTE_PERFORMER;
  if (!performer) throw new Error("Worker identity required");
  const base = process.env.COMPUTE_API_URL || "http://127.0.0.1:8090";
  const request = api(base, performer);
  do {
    await cleanWorker(root, payloadClient(base));
    await run(
      process.env.COMPUTE_EXECUTOR || "compute-executor",
      ["cleanup", "--root", join(root, "runs")],
      { env: guestEnvironment(process.env) },
    );
    let offset = 0;
    do {
      const page = await request(
        `/api/v1/compute/tasks?provider=${encodeURIComponent(performer)}&offset=${offset}&limit=50`,
      );
      for (const status of page.tasks) {
        if (status.provider !== performer) continue;
        try {
          await executeOrDecline({
            status,
            root,
            request,
            executor: process.env.COMPUTE_EXECUTOR || "compute-executor",
            ark: process.env.COMPUTE_ARK || "ark",
            artifactBase: base,
            fetchInput: async (_base, descriptor, destination) =>
              materialize(
                descriptor,
                destination,
                payloadClient(base),
                status.taskCid,
                0,
              ),
          });
          // Apply count retention between runs, not after an entire queue page.
          await cleanWorker(root, payloadClient(base));
        } catch (error) {
          console.error(`Worker queue error: ${error.message}`);
        }
      }
      offset = page.nextOffset;
    } while (offset !== null && offset !== undefined);
    if (process.argv.includes("--once")) return;
    await delay(Number(process.env.COMPUTE_POLL_MS || 15000));
  } while (true);
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  main().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
}
