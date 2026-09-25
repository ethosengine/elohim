#!/usr/bin/env node
/**
 * deliver-stage-result — D3: the local-ledger half of S3a's collect step. Given a
 * collect-stage.mjs run's verdict and durable evidence directory, this module (1) renders a
 * `sprint-report-peer-stage-<runId>.json` via the EXISTING `build-sprint-report.ts` (never a
 * parallel report writer — CLAUDE.md's four-artifact discipline), (2) fulfils or notes the
 * developer's REA gap commitment, and (3) appends one habit-atom DELTA (S3c, habit-delta.ts).
 *
 * Two independent guards keep the PROVIDER's evidence from ever carrying the WORKSPACE's own
 * key: `--lane peer-stage` (attestationsFromReport only fires for `lane === 'household'`) and
 * `A2O_POST_REPORT=0` in the child's env (publishHouseholdEvidence's own explicit early-out).
 * collect-stage.mjs's own `putAttestation` call is the ONE correctly-attributed ref this run
 * produces; build-sprint-report.ts's internal publishHouseholdEvidence must write nothing.
 */
import { spawnSync } from "node:child_process";
import { join, relative, resolve } from "node:path";

import {
  resolveBinary as defaultResolveBinary,
} from "../../../a2o/scripts/lib/household-attestation.ts";
import { appendDelta as defaultAppendDelta, findHabitByConcern as defaultFindHabitByConcern } from "../../../a2o/scripts/lib/habit-delta.ts";

function defaultExec(cmd, args, { cwd, env } = {}) {
  const r = spawnSync(cmd, args, {
    cwd,
    env: env ?? process.env,
    encoding: "utf8",
    timeout: 300000,
  });
  return { status: r.status, stdout: r.stdout ?? "", stderr: r.stderr ?? String(r.error ?? "") };
}

/** `<repoRoot>/genesis/a2o` — where build-sprint-report.ts is invoked from. */
function a2oDirOf(cwd) {
  return join(cwd, "genesis", "a2o");
}

/**
 * (6) Renders the peer-stage sprint report by running the SAME builder every other lane uses.
 * Returns `{ok, path, status, stderr}`; never throws.
 */
export function buildPeerStageReport({
  cwd,
  durableDir,
  stage,
  runId,
  exec = defaultExec,
  env = process.env,
}) {
  const outJsonRel = join("reports", `sprint-report-peer-stage-${runId}.json`);
  const outMdRel = join("reports", `sprint-report-peer-stage-${runId}.md`);
  const args = [
    "--import",
    "tsx",
    "scripts/build-sprint-report.ts",
    "--cucumber",
    join(durableDir, "cucumber.json"),
    "--out-json",
    outJsonRel,
    "--out-md",
    outMdRel,
    "--profile",
    "mesh",
    "--lane",
    "peer-stage",
    "--run-id",
    runId,
    "--scope",
    stage.feature,
    "--console-dir",
    join(durableDir, "console"),
    "--coverage-gap",
    join(durableDir, "none.json"),
  ];
  const result = exec(process.execPath, args, {
    cwd: a2oDirOf(cwd),
    // Two guards (see module doc): this is the second — the child must never publish a
    // household-lane attestation under the workspace's own key for a peer's run.
    env: { ...env, A2O_POST_REPORT: "0" },
  });
  const path = resolve(a2oDirOf(cwd), outJsonRel);
  return result.status === 0
    ? { ok: true, path }
    : { ok: false, path, status: result.status, stderr: result.stderr.trim() };
}

/**
 * (7) Fulfils the developer's gap on a `pass`; otherwise leaves an observation note naming the
 * failed scenarios. No gap named on the inbox entry -> logged skip. No `epr` binary on this
 * host -> fail-open, said so. A fulfil/note refusal is surfaced VERBATIM (never summarized) —
 * the plan's own example is the refusal text itself: "epr flow claim --on …".
 */
export function recordGapOutcome({
  entry,
  verdict,
  scenarios,
  durableDir,
  cwd,
  exec = defaultExec,
  env = process.env,
  resolveBinary = defaultResolveBinary,
}) {
  const epr = resolveBinary("epr", env.EPR_BIN, env.PATH);
  if (!epr) {
    return { skipped: true, reason: "no epr binary on this host (EPR_BIN or PATH)" };
  }
  if (!entry.gap) {
    return { skipped: true, reason: "no gap named on this inbox entry (--on was omitted at submit)" };
  }
  if (verdict === "pass") {
    const result = exec(
      epr,
      ["flow", "fulfill", "--on", entry.gap, "--report", join(durableDir, "stage.json"), "--status", "DONE", "--root", cwd],
      { cwd, env },
    );
    return result.status === 0
      ? { ok: true, kind: "fulfill", stdout: result.stdout.trim() }
      : { ok: false, kind: "fulfill", status: result.status, stderr: result.stderr.trim() };
  }
  const failedSlugs =
    scenarios.filter((s) => s.status !== "passed").map((s) => s.name).join(", ") || "(none named)";
  const result = exec(
    epr,
    [
      "flow",
      "note",
      "--on",
      entry.gap,
      "--kind",
      "observation",
      "--reason",
      `peer-stage ${verdict}: ${failedSlugs}`,
      "--root",
      cwd,
    ],
    { cwd, env },
  );
  return result.status === 0
    ? { ok: true, kind: "note", stdout: result.stdout.trim() }
    : { ok: false, kind: "note", status: result.status, stderr: result.stderr.trim() };
}

/**
 * (8) One habit DELTA (S3c). `COMPUTE_HABIT_DELTA=0` disables it (default on for peer
 * collection, per the plan). Rung H's line names the property it does NOT prove, in the exact
 * words R5 requires ("rung H, household stand-in, no offload proven") — never inferred as
 * offload evidence from the label alone.
 */
export function recordHabitDelta({
  stage,
  status,
  scenarios,
  verdict,
  reportPath,
  durableDir,
  reportRelDir,
  cwd,
  runId,
  rung,
  now = () => new Date(),
  env = process.env,
  findHabitByConcern = defaultFindHabitByConcern,
  appendDelta = defaultAppendDelta,
}) {
  if (env.COMPUTE_HABIT_DELTA === "0") {
    return { skipped: true, reason: "COMPUTE_HABIT_DELTA=0" };
  }
  const found = findHabitByConcern(cwd, stage.concern);
  if (!found.ok) {
    return { ok: false, reason: found.reason };
  }
  const providerShort = String(status.provider || "").slice(0, 8);
  const completionShort = String(status.completion?.actionHash || "").slice(0, 8);
  const rungLabel = rung === "H" ? "rung H, household stand-in, no offload proven" : "rung A";
  const label = `peer-stage ${rungLabel}, provider ${providerShort}, run ${runId}, completion ${completionShort}`;
  const passed = scenarios.filter((s) => s.status === "passed").length;
  const failed = scenarios.length - passed;
  const receiptRel = join(reportRelDir ?? relative(cwd, durableDir), "receipt.json");
  const reportRel = reportPath ? relative(cwd, reportPath) : "(report not written)";
  const text = `${stage.concern} passed=${passed} failed=${failed} — report ${reportRel}; receipt ${receiptRel}`;
  const date = now().toISOString().slice(0, 10);
  const result = appendDelta(found.path, { date, label, text, onceKey: completionShort });
  return { ok: result.ok, appended: result.appended, path: found.path, reason: result.reason };
}

/**
 * The full local-ledger half: build the report, then the gap outcome, then the habit delta.
 * Never throws; every step records its own outcome.
 */
export async function deliverStageResult({
  entry,
  status,
  stage,
  verdict,
  scenarios,
  durableDir,
  reportRelDir,
  cwd = process.cwd(),
  runners = {},
  env = process.env,
  now = () => new Date(),
  runId,
}) {
  const exec = runners.run ?? defaultExec;
  const resolveBinary = runners.resolveBinary ?? defaultResolveBinary;
  const findHabitByConcern = runners.findHabitByConcern ?? defaultFindHabitByConcern;
  const appendDelta = runners.appendDelta ?? defaultAppendDelta;

  const report = buildPeerStageReport({ cwd, durableDir, stage, runId, exec, env });
  const fulfill = recordGapOutcome({
    entry,
    verdict,
    scenarios,
    durableDir,
    cwd,
    exec,
    env,
    resolveBinary,
  });
  const delta = recordHabitDelta({
    stage,
    status,
    scenarios,
    verdict,
    reportPath: report.ok ? report.path : null,
    durableDir,
    reportRelDir,
    cwd,
    runId,
    rung: entry.rung === "A" ? "A" : "H",
    now,
    env,
    findHabitByConcern,
    appendDelta,
  });

  return {
    reportPath: report.ok ? report.path : null,
    reportError: report.ok ? undefined : { status: report.status, stderr: report.stderr },
    fulfill,
    delta,
  };
}
