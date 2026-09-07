import { readdir, rm } from "node:fs/promises";
import { join } from "node:path";
import { readJson, save } from "./common.mjs";
import { releasePayloads } from "./payloads.mjs";

export function expired(receipt, receipts, now = Date.now() / 1000) {
  const policy = receipt.retention || {
    maxAgeSeconds: 86400,
    maxRuns: 5,
    expireWhen: "either",
  };
  const newer = receipts.filter(
    (other) =>
      (other.completedAt > receipt.completedAt ||
        (other.completedAt === receipt.completedAt &&
          other.requestAction > receipt.requestAction)) &&
      ["requester", "provider", "project", "taskKind"].every(
        (field) => other[field] === receipt[field],
      ),
  ).length;
  const age =
    policy.maxAgeSeconds == null
      ? null
      : now >= receipt.completedAt + policy.maxAgeSeconds;
  const count = policy.maxRuns == null ? null : newer >= policy.maxRuns;
  const predicates = [age, count].filter((value) => value !== null);
  return (
    predicates.length > 0 &&
    (policy.expireWhen === "both"
      ? predicates.every(Boolean)
      : predicates.some(Boolean))
  );
}

export async function cleanWorker(root, payload) {
  const directory = join(root, "jobs");
  let names;
  try {
    names = await readdir(directory);
  } catch (error) {
    if (error.code === "ENOENT") return;
    throw error;
  }
  const rows = [];
  for (const name of names) {
    const path = join(directory, name, "state.json");
    const state = await readJson(path);
    if (state.refused && !state.payloadExpired) {
      await releasePayloads(
        [state.envelope.binary, state.envelope.dna],
        payload,
        state.taskCid,
      );
      state.payloadExpired = true;
      await save(path, state);
    }
    if (state.receipt && state.completed) rows.push({ path, state });
  }
  for (const row of rows) {
    if (
      row.state.payloadExpired ||
      !expired(
        row.state.receipt,
        rows.map((row) => row.state.receipt),
      )
    )
      continue;
    await releasePayloads(
      [
        ...(row.state.receipt.logs || []),
        row.state.receipt.binary,
        row.state.receipt.dna,
      ],
      payload,
      row.state.receipt.taskCid,
    );
    row.state.payloadExpired = true;
    await save(row.path, row.state);
  }
}

export async function cleanInbox(root, payload) {
  const directory = join(root, "inbox");
  const rows = [];
  for (const name of (await readdir(directory)).filter((name) =>
    name.endsWith(".json"),
  )) {
    const path = join(directory, name);
    const state = await readJson(path);
    if (state.status?.refusal && !state.payloadExpired) {
      await releasePayloads(
        [state.status.envelope.binary, state.status.envelope.dna],
        payload,
        state.status.taskCid,
      );
      state.payloadExpired = true;
      await save(path, state);
    }
    const receipt = state.status?.completion?.receipt;
    if (receipt) rows.push({ path, state, receipt });
  }
  for (const row of rows) {
    if (row.state.payloadExpired) {
      for (const review of Object.values(row.state.reviews || {})) {
        if (review.report) await rm(review.report, { force: true });
      }
    }
    if (
      row.state.payloadExpired ||
      !expired(
        row.receipt,
        rows.map((row) => row.receipt),
      )
    )
      continue;
    await releasePayloads(
      [...(row.receipt.logs || []), row.receipt.binary, row.receipt.dna],
      payload,
      row.receipt.taskCid,
    );
    for (const path of row.state.logPaths || [])
      await rm(path, { force: true });
    row.state.payloadExpired = true;
    row.state.logPaths = [];
    row.state.logsFetched = true;
    row.state.payloadError =
      "Payload expired under task retention policy; compact receipt retained";
    await save(row.path, row.state);
  }
}
