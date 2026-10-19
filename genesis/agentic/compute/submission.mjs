// The ONE shape a requester submits a compute task in — shared by the household
// requester (workspace.mjs, over its own loopback storage) and the hosted CI runner
// (genesis/a2o/scripts/compute-runner.ts, over its own hosted cell + the sponsor's
// payload custody). Both call these two functions, so the provider's worker sees
// byte-identical envelopes and submit inputs whichever requester sent them.
import { join, resolve } from "node:path";
import { randomUUID } from "node:crypto";
import { key, run, save } from "./common.mjs";
import { publishFile } from "./payloads.mjs";

/** The native compute envelope limit (content_store::compute_task). */
export const ENVELOPE_LIMIT = 65536;

/**
 * Fix a task's identity and publish its two inputs.
 *
 * 1. a fresh `invocation` nonce unless one is pinned (or `newRun`);
 * 2. chunk descriptors for `binary`/`dna` (no payload writes yet);
 * 3. the envelope saved under `root/submissions/` and back to `file`, so a lost
 *    submit response can be retried without producing another execution;
 * 4. the task CID from the executor, then the chunks written through `payload`
 *    leased to that CID.
 */
export async function publishSubmission({
  envelope,
  inputPaths,
  file,
  root,
  executor,
  payload,
  execute = run,
  newRun = false,
}) {
  if (!envelope.invocation || newRun) envelope.invocation = randomUUID();
  if (!inputPaths || inputPaths.length !== 2 || inputPaths.some((path) => !path))
    throw new Error("submit requires TASK.json TEST_BINARY LAMAD.dna");
  for (const [index, field] of ["binary", "dna"].entries()) {
    envelope[field] = await publishFile(resolve(inputPaths[index]), envelope[field], {
      payload: async () => {},
      executor,
      retention: 0,
      execute,
    });
  }
  const publishedTask = join(root, "submissions", `${key(JSON.stringify(envelope))}.json`);
  await save(publishedTask, envelope);
  if (file) await save(resolve(file), envelope);
  if (Buffer.byteLength(JSON.stringify(envelope)) > ENVELOPE_LIMIT)
    throw new Error(
      "Compute envelope exceeds native 64 KiB limit; use a smaller prebuilt artifact",
    );
  const taskCid = (await execute(executor, ["cid", publishedTask])).trim();
  for (const [index, field] of ["binary", "dna"].entries()) {
    await publishFile(resolve(inputPaths[index]), envelope[field], {
      payload,
      executor,
      owner: taskCid,
      retention: 0,
      execute,
    });
  }
  return { envelope, taskCid, publishedTask };
}

/** The submit body: what storage's adapter forwards to `compute_task` (it adds
 * `operation: "submit"`), and what a hosted requester sends its own cell. */
export function submitRequest({ envelope, taskCid, grantActionHash }) {
  return { taskCid, provider: envelope.provider, grantActionHash, envelope };
}
