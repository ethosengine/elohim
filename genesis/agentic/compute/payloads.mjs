import { open, mkdtemp, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createHash } from "node:crypto";
import { run } from "./common.mjs";

// Count-only and conjunctive policies are released by the worker/inbox
// retention evaluator. A hidden age lease would violate their envelope.
export function leaseSeconds(policy) {
  return policy?.maxAgeSeconds == null || policy?.expireWhen === "both"
    ? 0
    : policy.maxAgeSeconds;
}

export const CHUNK_BYTES = 1024 * 1024;
export function payloadClient(
  base,
  token = process.env.ELOHIM_COMPUTE_LOCAL_TOKEN || "",
  fetcher = fetch,
) {
  const origin = new URL(base);
  if (
    origin.protocol !== "http:" ||
    !["127.0.0.1", "localhost", "[::1]"].includes(origin.hostname)
  ) {
    throw new Error("Payload adapter requires own loopback storage");
  }
  return async (cid, method = "GET", body, retention = 86400, owner) => {
    if (!owner) throw new Error("Payload lease owner task CID required");
    const response = await fetcher(
      new URL(`/api/v1/compute/payloads/${encodeURIComponent(cid)}`, origin),
      {
        method,
        body,
        redirect: "error",
        signal: AbortSignal.timeout(300_000),
        headers: {
          "x-elohim-compute-token": token,
          "content-type": "application/octet-stream",
          "x-compute-retain-seconds": String(retention),
          "x-compute-owner": owner,
        },
      },
    );
    if (!response.ok) throw new Error(`Compute payload ${response.status}`);
    if (method !== "GET") return;
    const chunks = [];
    let size = 0;
    for await (const part of response.body) {
      size += part.length;
      if (size > CHUNK_BYTES) throw new Error("Payload chunk exceeds 1 MiB");
      chunks.push(part);
    }
    return Buffer.concat(chunks);
  };
}

export async function publishFile(
  path,
  descriptor,
  { payload, executor, retention = 86400, execute = run, owner },
) {
  const actual = JSON.parse(await execute(executor, ["artifact", path]));
  if (
    ["cid", "sha256", "bytes"].some((name) => actual[name] !== descriptor[name])
  )
    throw new Error("Input artifact mismatch");
  const dir = await mkdtemp(join(tmpdir(), "compute-chunks-"));
  const source = await open(path, "r");
  const chunks = [];
  try {
    let offset = 0;
    while (offset < descriptor.bytes) {
      const buffer = Buffer.alloc(
        Math.min(CHUNK_BYTES, descriptor.bytes - offset),
      );
      let count = 0;
      while (count < buffer.length) {
        const result = await source.read(
          buffer,
          count,
          buffer.length - count,
          offset + count,
        );
        if (result.bytesRead === 0)
          throw new Error("Artifact truncated during publication");
        count += result.bytesRead;
      }
      const temporary = join(dir, "chunk");
      await writeFile(temporary, buffer, { mode: 0o600 });
      const chunk = JSON.parse(
        await execute(executor, ["artifact", temporary]),
      );
      await payload(chunk.cid, "PUT", buffer, retention, owner);
      chunks.push(chunk);
      offset += count;
    }
  } finally {
    await source.close();
    await rm(dir, { recursive: true, force: true });
  }
  return { ...descriptor, chunks };
}

export async function materialize(
  descriptor,
  path,
  payload,
  owner,
  retention = 86400,
) {
  if (descriptor.bytes === 0 && descriptor.chunks?.length === 0) {
    if (descriptor.sha256 !== createHash("sha256").update("").digest("hex"))
      throw new Error("Empty artifact digest mismatch");
    await writeFile(path, Buffer.alloc(0), { mode: 0o600 });
    return;
  }
  if (!descriptor.chunks?.length)
    throw new Error("Task requires native payload chunk descriptors");
  const output = await open(path, "w", 0o700);
  const hash = createHash("sha256");
  let bytes = 0;
  try {
    for (const chunk of descriptor.chunks) {
      const body = await payload(chunk.cid, "GET", undefined, retention, owner);
      if (
        body.length !== chunk.bytes ||
        createHash("sha256").update(body).digest("hex") !== chunk.sha256
      ) {
        throw new Error("Payload chunk digest mismatch");
      }
      bytes += body.length;
      if (bytes > descriptor.bytes)
        throw new Error("Artifact exceeds declared length");
      hash.update(body);
      await output.writeFile(body);
    }
    if (bytes !== descriptor.bytes || hash.digest("hex") !== descriptor.sha256)
      throw new Error("Artifact digest or length mismatch");
  } finally {
    await output.close();
  }
}

export async function releasePayloads(descriptors, payload, owner) {
  for (const cid of new Set(
    descriptors.flatMap(
      (item) => item?.chunks?.map((chunk) => chunk.cid) || [],
    ),
  )) {
    try {
      await payload(cid, "DELETE", undefined, 86400, owner);
    } catch (error) {
      if (!/payload (404|410)$/.test(error.message)) throw error;
    }
  }
}
