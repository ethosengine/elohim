#!/usr/bin/env node
// cid-executor — the two NAMING subcommands of the rakia compute-executor, and nothing else.
//
//   cid-executor artifact <path>   -> {"cid","sha256","bytes"}  (CIDv1 raw over sha2-256)
//   cid-executor cid <task.json>   -> the task's CIDv1 dag-cbor over its JSON
//
// Every other subcommand (run, receipt-path, cleanup) refuses: this tool can NAME a stage, it
// can never RUN one. The executor source is present at elohim/rakia/rakia-executor
// (rakia 39cd2eb, 2026-10-21); the executor is unbuilt in the dev slot, not lost
// (genesis/data/timeline/backlog/rakia-executor-untracked-in-submodule-pin.md), while a
// requester still has to name what it submits. Nothing here is trusted on its word: the
// sponsor's custody re-hashes every chunk it stocks, and a provider's storage recomputes the
// task CID from the envelope (elohim-storage api/compute_tasks.rs content_cid), so a naming
// drift is refused by the substrate, never accepted silently.
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

import * as dagCbor from "@ipld/dag-cbor";
import { CID } from "multiformats/cid";
import * as raw from "multiformats/codecs/raw";
import { sha256 } from "multiformats/hashes/sha2";

export async function artifactDescriptor(bytes) {
  const digest = await sha256.digest(bytes);
  return {
    cid: CID.createV1(raw.code, digest).toString(),
    sha256: createHash("sha256").update(bytes).digest("hex"),
    bytes: bytes.length,
  };
}

export async function jsonCid(value) {
  const digest = await sha256.digest(dagCbor.encode(value));
  return CID.createV1(dagCbor.code, digest).toString();
}

export const REFUSAL =
  "cid-executor names stages and never runs them: the rakia compute-executor is unavailable " +
  "(genesis/data/timeline/backlog/rakia-executor-untracked-in-submodule-pin.md)";

/** In-process stand-in for `execute(executor, args)` (common.mjs run): same stdout contract. */
export async function execute(_program, args) {
  const [verb, path] = args;
  if (verb === "artifact" && path) return JSON.stringify(await artifactDescriptor(await readFile(path)));
  if (verb === "cid" && path) return `${await jsonCid(JSON.parse(await readFile(path, "utf8")))}\n`;
  throw new Error(REFUSAL);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  execute("cid-executor", process.argv.slice(2)).then(
    (out) => process.stdout.write(out.endsWith("\n") ? out : `${out}\n`),
    (error) => {
      process.stderr.write(`cid-executor: ${error.message}\n`);
      process.exit(2);
    },
  );
}
