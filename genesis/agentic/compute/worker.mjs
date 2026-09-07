#!/usr/bin/env node
import { mkdir, rm, chmod } from "node:fs/promises";
import { randomUUID } from "node:crypto";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
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
  const taskFile = join(dir, "task.json");
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
      await readJson(runtimePath); // Existence only; lock-protected runtime owns the authoritative read.
      const recovered = JSON.parse(
        await execute(executor, runtimeArgs, {
          env: guestEnvironment(env),
          timeoutMs: (task.resources?.timeoutSeconds || 3600) * 1000 + 120000,
        }),
      );
      if (
        recovered.taskCid !== status.taskCid ||
        recovered.grantAction !== status.grantActionHash
      ) {
        throw new Error("Recovered runtime receipt binding mismatch");
      }
      state.receipt = recovered;
      await save(stateFile, state);
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
    await fetchInput(artifactBase, task.binary, binary);
    await fetchInput(artifactBase, task.dna, dna);
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
          await executeTask({
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
          console.error(
            `Task ${key(status.requestActionHash)}: ${error.message}`,
          );
          const reason = /API 403/.test(error.message)
            ? "compute-grant-refused"
            : /mismatch|invalid artifact|digest/i.test(error.message)
              ? "invalid-artifact"
              : /capacity|quota|ceiling/i.test(error.message)
                ? "capacity-unavailable"
                : /runtime image|runtime.*incompatible/i.test(error.message)
                  ? "runtime-incompatible"
                  : null;
          if (reason) {
            const local = await readJson(
              join(root, "jobs", key(status.requestActionHash), "state.json"),
            );
            await request(
              `/api/v1/compute/tasks/${encodeURIComponent(status.requestActionHash)}/decline`,
              { attemptId: local.attemptId, reason },
            );
          }
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
