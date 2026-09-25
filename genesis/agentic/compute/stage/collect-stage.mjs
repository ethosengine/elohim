#!/usr/bin/env node
/**
 * collect-stage — S3a: the requester's own collection of a peer-executed a2o stage's
 * evidence, once the provider's completion has been read back.
 *
 * Reused directly (never re-derived): `PEER_STAGE_SCOPE`, `checkNameFor`, `cidToString`,
 * `sutArtifactCidBytes`, `resolveMooring`, `putAttestation`, `resolveBinary`, `gitCommonDir`
 * and `RECEIPT_SCHEMA` all come from genesis/a2o/scripts/lib/household-attestation.ts — the
 * one place `admitAttestation` (the pre-push T2 reader) and this collector agree on what a
 * receipt looks like. Import style mirrors genesis/orchestrator/scripts/serving-receipt.mjs
 * (a plain .mjs importing a .ts file directly under Node's native type-stripping).
 *
 * Contract, same as `publishHouseholdEvidence`: every step here is idempotent, records what
 * it did (or why it did nothing) in the returned `entry.evidence`, and NEVER throws — a
 * refused precondition, a missing binary, or an unreachable epr is an honest recorded fact,
 * not a crash that would also take down the review delivery loop running beside it.
 */
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { basename, dirname, join } from "node:path";
import { spawnSync } from "node:child_process";

import { key } from "../common.mjs";
import { loadCucumber } from "../../../a2o/scripts/lib/load-cucumber.ts";
import {
  PEER_STAGE_SCOPE,
  RECEIPT_SCHEMA,
  checkNameFor,
  cidToString,
  gitCommonDir as defaultGitCommonDir,
  putAttestation as defaultPutAttestation,
  resolveBinary as defaultResolveBinary,
  resolveMooring,
  sutArtifactCidBytes,
} from "../../../a2o/scripts/lib/household-attestation.ts";
import { deliverStageResult as defaultDeliverStageResult } from "./deliver-stage-result.mjs";

const BEGIN_SENTINEL = "-----BEGIN ELOHIM STAGE REPORT-----";
const END_SENTINEL = "-----END ELOHIM STAGE REPORT-----";

/** The D7 sentinel framing — the raw decoded bytes AND the parsed cucumber doc, or null. */
export function decodeFramedReport(stdoutText) {
  const start = stdoutText.indexOf(BEGIN_SENTINEL);
  const stop = stdoutText.indexOf(END_SENTINEL);
  if (start < 0 || stop <= start) return null;
  const encoded = stdoutText.slice(start + BEGIN_SENTINEL.length, stop).trim();
  let buffer;
  try {
    buffer = Buffer.from(encoded, "base64");
  } catch {
    return null;
  }
  let doc;
  try {
    doc = JSON.parse(buffer.toString("utf8"));
  } catch {
    return null;
  }
  return { buffer, doc };
}

/**
 * D9 verdict, moved out of the steps file so both the requester's live probe and the
 * collector share one pure function: the decoded cucumber report — never the receipt's own
 * `status` claim — names the verdict. `stage` is the sidecar build-stage-task.mjs emitted
 * (feature, concern, scenarioNames, …).
 *
 * Refuses (skip) rather than attesting when a scenario's own tags disagree with the feature's
 * declared @concern: — never writes two refs for one stage.
 */
export function stageVerdict(reportDoc, stage) {
  const results = loadCucumber(JSON.stringify(reportDoc ?? []));
  const matching = results.filter((r) => r.feature === stage.feature);

  for (const r of matching) {
    for (const tag of r.tags || []) {
      if (!tag.startsWith("@concern:")) continue;
      const named = tag.slice("@concern:".length);
      if (named !== stage.concern) {
        return {
          verdict: "skip",
          scenarios: [],
          reason: `scenario "${r.name}" declares ${tag}, not the feature's @concern:${stage.concern} — refusing rather than writing a mismatched ref`,
        };
      }
    }
  }

  const declared = new Set(stage.scenarioNames || []);
  const seen = new Set(matching.map((r) => r.name));
  const inventoryOk =
    declared.size === seen.size && [...declared].every((name) => seen.has(name));

  const scenarios = matching.map((r) => ({
    name: r.name,
    status: r.status === "passed" ? "passed" : "failed",
    surface: r.feature,
    durationMs: Math.round(r.durationMs ?? 0),
  }));

  if (!inventoryOk) {
    return {
      verdict: "fail",
      scenarios,
      reason: `report scenario inventory for ${stage.feature} does not match the pinned declaration (declared ${[...declared].sort().join(", ")}; saw ${[...seen].sort().join(", ")})`,
    };
  }
  if (scenarios.length === 0) {
    return { verdict: "skip", scenarios, reason: "report named no scenarios for this feature" };
  }
  const failed = scenarios.some((s) => s.status !== "passed");
  return { verdict: failed ? "fail" : "pass", scenarios };
}

function defaultSyncRunner(cmd, args, cwd) {
  const r = spawnSync(cmd, args, { cwd, encoding: "utf8", timeout: 120000 });
  return { status: r.status, stdout: r.stdout ?? "", stderr: r.stderr ?? String(r.error ?? "") };
}

/**
 * The requester's collection of one peer-executed stage's evidence, called from
 * `workspace.mjs`'s `pollInbox` once `status.completion` and `entry.stage` (the sidecar
 * build-stage-task.mjs copied into the inbox entry on submit) are both present. Mutates and
 * returns `entry.evidence`; the caller persists `entry` (mirrors the logs/reviews blocks
 * beside this call).
 */
export async function collectStageEvidence({
  entry,
  status,
  root,
  cwd = process.cwd(),
  runners = {},
  env = process.env,
  now = () => new Date(),
}) {
  const runSync = runners.run ?? defaultSyncRunner;
  const resolveBin = runners.resolveBinary ?? defaultResolveBinary;
  const getGitCommonDir = runners.gitCommonDir ?? defaultGitCommonDir;
  const putRef = runners.putAttestation ?? defaultPutAttestation;
  const deliver = runners.deliverStageResult ?? defaultDeliverStageResult;

  entry.evidence ??= {};
  if (entry.evidence.done) return entry.evidence; // idempotent across restarts / re-polls

  const stage = entry.stage;
  if (!stage) {
    entry.evidence = { done: true, verdict: null, refused: "no stage.json on this inbox entry" };
    return entry.evidence;
  }

  // (1) precondition — the requester's own storage must have verified the provider's grant
  // under the measure-stage scope before this counts as anything.
  const observed = status.observed;
  if (!observed?.verified || observed.scope !== PEER_STAGE_SCOPE) {
    entry.evidence = {
      done: true,
      verdict: null,
      refused: observed?.refused
        ? String(observed.refused)
        : `grant not observed under scope ${PEER_STAGE_SCOPE}`,
    };
    return entry.evidence;
  }

  // (2) decode the sentinel-framed report from the materialized stdout.log, re-checking its
  // digest against the receipt's own declared descriptor (materialize() already checked it on
  // the way down — this is a second, independent check at the point of use).
  const receipt = status.completion?.receipt;
  const logsByName = Object.fromEntries((receipt?.logs || []).map((l) => [l.name, l]));
  const logPaths = entry.logPaths || [];
  const stdoutPath = logPaths.find((p) => p.endsWith("stdout.log"));
  const stderrPath = logPaths.find((p) => p.endsWith("stderr.log"));

  // The durable target — computed once, ahead of every terminal (skip/fail/pass) return below,
  // not just the pass path: a refusal is evidence too. `genesis/a2o/reports/peer-stage/<date>/
  // <key(ref)>/`.
  const ref = status.requestActionHash || entry.requestActionHash || "";
  const refKey = key(ref);
  const dateStr = now().toISOString().slice(0, 10);
  const reportRelDir = join("genesis", "a2o", "reports", "peer-stage", dateStr, refKey);
  const durableDir = join(cwd, reportRelDir);

  let stderrText = "";
  if (stderrPath) {
    try {
      stderrText = await readFile(stderrPath, "utf8");
    } catch {
      /* an absent/unreadable stderr.log is not itself a failure */
    }
  }

  // Writes whatever pieces of evidence are available at the call site — cucumber.json only
  // once the framed report has decoded, stdout.log only once fetched — so an early refusal (a
  // missing artifact, a digest mismatch, STAGE-PRECONDITION-UNMET) still leaves a durable
  // trail under genesis/a2o/reports/peer-stage/ instead of only the ephemeral build dir.
  const writeDurable = async ({ framed = null, stdoutBuf = null } = {}) => {
    await mkdir(durableDir, { recursive: true, mode: 0o700 });
    if (framed) await writeFile(join(durableDir, "cucumber.json"), framed.buffer);
    await writeFile(
      join(durableDir, "receipt.json"),
      JSON.stringify(receipt ?? null, null, 2) + "\n",
    );
    await writeFile(join(durableDir, "status.json"), JSON.stringify(status, null, 2) + "\n");
    await writeFile(join(durableDir, "stage.json"), JSON.stringify(stage, null, 2) + "\n");
    if (stdoutBuf) await writeFile(join(durableDir, "stdout.log"), stdoutBuf);
    if (stderrText) await writeFile(join(durableDir, "stderr.log"), stderrText);
    return durableDir;
  };

  if (!stdoutPath) {
    await writeDurable();
    entry.evidence = {
      done: true,
      verdict: "skip",
      reason: "stdout.log was not materialized",
      report: reportRelDir,
      durable: durableDir,
    };
    return entry.evidence;
  }
  let stdoutBuf;
  try {
    stdoutBuf = await readFile(stdoutPath);
  } catch (error) {
    await writeDurable();
    entry.evidence = {
      done: true,
      verdict: "skip",
      reason: `cannot read stdout.log: ${error.message}`,
      report: reportRelDir,
      durable: durableDir,
    };
    return entry.evidence;
  }
  const declaredSha = logsByName["stdout.log"]?.sha256;
  const actualSha = createHash("sha256").update(stdoutBuf).digest("hex");
  if (!declaredSha || actualSha !== declaredSha) {
    await writeDurable({ stdoutBuf });
    entry.evidence = {
      done: true,
      verdict: "fail",
      reason: `stdout.log digest ${actualSha} does not match the receipt's declared ${declaredSha ?? "(absent)"}`,
      report: reportRelDir,
      durable: durableDir,
    };
    return entry.evidence;
  }

  const preconditionLine = stderrText
    .split("\n")
    .find((line) => line.includes("STAGE-PRECONDITION-UNMET"));
  if (preconditionLine) {
    await writeDurable({ stdoutBuf });
    entry.evidence = {
      done: true,
      verdict: "skip",
      reason: preconditionLine.trim(),
      report: reportRelDir,
      durable: durableDir,
    };
    return entry.evidence;
  }

  const framed = decodeFramedReport(stdoutBuf.toString("utf8"));
  if (!framed) {
    await writeDurable({ stdoutBuf });
    entry.evidence = {
      done: true,
      verdict: "fail",
      reason: "stdout.log did not carry a framed ELOHIM STAGE REPORT",
      report: reportRelDir,
      durable: durableDir,
    };
    return entry.evidence;
  }
  const reportSha256 = createHash("sha256").update(framed.buffer).digest("hex");

  // (3) the verdict — reads the decoded report, never receipt.status.
  const { verdict, scenarios, reason: verdictReason } = stageVerdict(framed.doc, stage);

  // (4) durable copy: genesis/a2o/reports/peer-stage/<date>/<key(ref)>/ — every terminal
  // verdict (skip/fail/pass alike) leaves one here; only the brit put, the gap fulfil and the
  // DELTA below stay gated on how `deliverStageResult` itself reads `verdict`.
  await writeDurable({ framed, stdoutBuf });

  // (5) ONE brit put per stage.
  const rung = entry.rung === "A" ? "A" : "H";
  const runId = `peer-${refKey.slice(0, 8)}`;
  const summary = {
    schema: RECEIPT_SCHEMA,
    concern: stage.concern,
    sut: stage.sut,
    sutParts: stage.sutParts,
    // R2 final: rung H (household stand-in) is honestly `lane: 'household'` — jessica IS a
    // peer on this household's own mesh; rung A (a real, separate peer) is honestly
    // `lane: 'peer-stage'`, and admits only as habit evidence, never a T2 push receipt.
    lane: rung === "H" ? "household" : "peer-stage",
    processControl: true,
    runId,
    report: join(reportRelDir, "stage.json"),
    reach: "trusted",
    moored: resolveMooring(env),
    scenarios,
    peer: {
      rung,
      provider: status.provider,
      requester: status.requester,
      grantActionHash: status.grantActionHash,
      grantCid: observed.grantCid,
      scope: observed.scope,
      requestActionHash: status.requestActionHash,
      completionActionHash: status.completion?.actionHash,
      receiptCid: status.completion?.receiptCid,
      taskCid: status.taskCid,
      featureSha256: status.envelope?.dna?.sha256 ?? stage.featureSha256,
      reportSha256,
    },
  };
  const attestation = {
    check: checkNameFor(stage.concern, stage.sut),
    artifact: cidToString(sutArtifactCidBytes(stage.sutParts)),
    result: verdict,
    summary,
  };

  let attestationOutcome;
  const common = getGitCommonDir(cwd);
  const workspaceDir = common && basename(common) === ".git" ? dirname(common) : null;
  const brit = resolveBin("brit-build-ref", env.BRIT_BUILD_REF, env.PATH);
  if (!brit) {
    attestationOutcome = {
      skipped: true,
      reason:
        "validation attestation skipped: brit-build-ref is not built on this host (BRIT_BUILD_REF or PATH)",
    };
  } else if (!workspaceDir) {
    attestationOutcome = {
      skipped: true,
      reason: "validation attestation skipped: no .git common dir to hold the workspace key",
    };
  } else {
    const r = putRef(brit, workspaceDir, attestation, runSync, cwd);
    attestationOutcome =
      r.status === 0
        ? { ok: true, check: attestation.check, artifact: attestation.artifact }
        : { ok: false, status: r.status, stderr: (r.stderr || "").trim() };
  }

  // (6)-(9): the local-ledger half — D3's own module (sprint report, epr fulfil/note, habit
  // delta), kept separate so its own concerns (subprocess invocation, CLI flags) stay out of
  // this collector's precondition/decode/attest logic.
  const delivery = await deliver({
    entry,
    status,
    stage,
    verdict,
    scenarios,
    durableDir,
    reportRelDir,
    cwd,
    runners: { run: runSync, resolveBinary: resolveBin },
    env,
    now,
    refKey,
    runId,
    rung,
  });

  entry.evidence = {
    done: true,
    verdict,
    verdictReason,
    report: reportRelDir,
    durable: durableDir,
    attestation: attestationOutcome,
    ...delivery,
  };
  return entry.evidence;
}
