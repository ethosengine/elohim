import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createHash } from "node:crypto";
import { artifactDescriptor, execute, jsonCid, REFUSAL } from "./cid-executor.mjs";

test("artifact names bytes as a raw CIDv1 over their sha2-256, with digest and length", async () => {
  const bytes = Buffer.from("a pinned stage");
  const named = await artifactDescriptor(bytes);
  assert.match(named.cid, /^bafkrei[a-z2-7]+$/);
  assert.equal(named.sha256, createHash("sha256").update(bytes).digest("hex"));
  assert.equal(named.bytes, bytes.length);
});

test("cid names a task as dag-cbor, independent of JSON key order", async () => {
  const a = await jsonCid({ schemaVersion: 1, requester: "r", provider: "p" });
  const b = await jsonCid({ provider: "p", requester: "r", schemaVersion: 1 });
  assert.match(a, /^bafyrei[a-z2-7]+$/);
  assert.equal(a, b);
});

test("the stand-in names stages and refuses to run one", async (t) => {
  const dir = await mkdtemp(join(tmpdir(), "cid-executor-"));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const task = join(dir, "task.json");
  await writeFile(task, JSON.stringify({ schemaVersion: 1 }));
  assert.match(await execute("cid-executor", ["cid", task]), /^bafyrei/);
  assert.match(JSON.parse(await execute("cid-executor", ["artifact", task])).cid, /^bafkrei/);
  for (const verb of ["run", "receipt-path", "cleanup"])
    await assert.rejects(execute("cid-executor", [verb, task]), { message: REFUSAL });
});
