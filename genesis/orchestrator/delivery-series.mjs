#!/usr/bin/env node
// delivery-series — did recent pushes actually deliver, and how long did it take?
//
// The check behind genesis/orchestrator/.epr-meta/push-delivers-within-budget.habit.md.
// It reads the actual-build-graph.json every orchestrator run already archives
// (no new ledger). For each run that planned work, it asks one question: did
// every planned pipeline finish SUCCESS or UNSTABLE? It also reports how long
// the delivered runs took and how many runs a timeout killed.
//
// Why: from 2026-09-19 to 09-22 app delivered zero of eight dispatches.
// Nothing counted that. ci-harvest dropped the ABORTED builds, and no series
// kept push → delivered time. A green downstream verdict here and there hid
// a pipeline that no longer shipped end to end.
//
// --stages (Lane D1 of the native-delivery sprint; evidence-ladder spec §8) answers the
// question the orchestrator series cannot: WHERE inside a pipeline the wall clock went, and
// what one delivered bundle cost. It reads each build's Pipeline Stage View
// (`wfapi/describe`, the same source and vocabulary as
// scripts/pipeline-trajectory.mjs getBuildStages) plus the concern-scoped phase cases the
// App pipeline records in its junit report (classname `elohim-app.deploy.<env>`, case names
// `readiness`, `publish.<x>`, `verify.<x>`, `converge.<x>`). A build that predates the phases
// falls back to its "Publish and Verify" stage, so the readiness wait is reported unsplit,
// never guessed. #1719–#1725 spent ≈12 pipeline-hours and delivered nothing; that is the
// number this mode puts in front of a reader.
//
// --from attestations is a partial commit-level view. Notes do not preserve the complete
// wave plan or Jenkins wall-clock timing; builtAt is the device's harvest timestamp.
// Jenkins remains the default and the only source for delivery bounds and --stages.
// The notes are read THROUGH brit (`brit-build-ref build get`, one verified read per ci:<pipeline>
// note) when the binary is on BRIT_BUILD_REF_BIN / BRIT_BUILD_REF / PATH. The env must name a REAL
// binary: /opt/rust/cargo/bin/brit-build-ref is a symlink to the bin-store launcher, which resolves
// only through PATH and refuses its own alias (`REFUSED launcher-alias`, exit 127) when named
// directly. A note brit reports `unverifiable:` is absent to this series — dropped from the run
// and named once on stderr. The output says which reader produced it (`verification.reader`, and
// the source label): `brit-verified` only when the binary verifies what it prints (it knows
// `signer --json`); the pinned 76ba95d8a0 binary echoes stored bytes, so its reads are
// `brit-unverified` — and a body it cannot decode is its `Error: JSON error: …` exit 1, which this
// reader maps to the same absent-and-named outcome rather than letting one stray body collapse
// the series; with no brit the notes are read from git directly and labelled `raw-git`.
// UNVERIFIED is always said, never implied. A git failure inside brit is NOT MEASURED, never an
// empty series, and an empty or all-refused series never falls back to Jenkins.
//
// CLI:  node genesis/orchestrator/delivery-series.mjs [--window 10] [--min-rate 0.8]
//         [--max-p90-min 240] [--json] [--from jenkins|attestations] [--repo <path>]
//       node genesis/orchestrator/delivery-series.mjs --stages [--pipelines elohim]
//         [--window 10] [--max-publish-verify-p90-min 20] [--max-cost-hours 1] [--json]
// Exit: 0 within the declared bounds · 1 outside them · 2 not enough evidence
//       (missing data is never read as health).
// The --stages defaults mirror the bounds declared in .claude/epr-meta/measures.yaml
// (stage-wallclock-ceiling@1, delivery-cost-ceiling@1); the test pins the two together.

import { execFileSync, spawnSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';
import { resolveBin, signerIdentity } from './gate-evidence.mjs';
import { unverifiableNotes } from './scripts/brit-attest.mjs';

const JENKINS = process.env.JENKINS || 'https://jenkins.ethosengine.com';
const JOB = '/job/elohim-orchestrator/job/dev';
const DELIVERED = new Set(['SUCCESS', 'UNSTABLE']);
// A fire-and-forget (longRunning, standalone) pipeline reports DISPATCHED: the
// orchestrator never waited for it, so this series cannot see its verdict and
// does not count it against delivery (its own job view carries that).
const NOT_AWAITED = 'DISPATCHED';
const TIMEOUT_TEXT = 'Timeout has been exceeded';

/**
 * One orchestrator run → the facts this habit reads, or null when it planned
 * no work (no-op timers, [skip ci]) and so says nothing about delivery.
 *
 * @param {{number:number, result:string|null, durationMs:number}} build
 * @param {{pipelines?:string[]}|null} predicted
 * @param {{results?:object, abortedBeforeStart?:string[]}|null} actual
 */
export function classifyRun(build, predicted, actual) {
  const planned = actual?.executionOrder ?? predicted?.pipelines ?? [];
  if (planned.length === 0) return null;
  const results = actual?.results ?? {};
  const outcome = Object.fromEntries(planned.map(name => [name, results[name]?.result ?? 'NOT_DISPATCHED']));
  const delivered =
    build.result !== 'ABORTED' &&
    planned.every(name => DELIVERED.has(outcome[name]) || outcome[name] === NOT_AWAITED);
  const timedOut =
    Object.values(results).some(r => (r?.error ?? '').includes(TIMEOUT_TEXT)) ||
    (build.result === 'ABORTED' && !actual);
  return {
    number: build.number,
    result: build.result,
    minutes: Math.round(build.durationMs / 6000) / 10,
    planned,
    outcome,
    delivered,
    timedOut,
  };
}

function percentile(sorted, p) {
  if (sorted.length === 0) return null;
  const idx = Math.min(sorted.length - 1, Math.ceil((p / 100) * sorted.length) - 1);
  return sorted[Math.max(0, idx)];
}

/** Summarize the newest `window` work-bearing runs (runs newest-first). */
export function summarize(runs, { window = 10 } = {}) {
  const recent = runs.filter(Boolean).slice(0, window);
  const delivered = recent.filter(r => r.delivered === true);
  const minutes = delivered.map(r => r.minutes).filter(Number.isFinite).sort((a, b) => a - b);
  return {
    window,
    considered: recent.length,
    delivered: recent.some(r => r.delivered === 'unknown') ? null : delivered.length,
    rate: recent.length && recent.every(r => typeof r.delivered === 'boolean') ? delivered.length / recent.length : null,
    // A source that cannot see timeouts (build notes) makes the count unknown, not zero.
    timedOut: recent.some(r => r.timedOut == null) ? null : recent.filter(r => r.timedOut).length,
    p50DeliveredMin: percentile(minutes, 50),
    p90DeliveredMin: percentile(minutes, 90),
    lastDelivered: delivered[0]?.number ?? null,
    runs: recent,
  };
}

// ── --from attestations: brit build notes instead of the Jenkins API ──────────────────────

const BUILD_NOTES = 'refs/notes/brit/build/';
const BUILD_REF_ENV = ['BRIT_BUILD_REF_BIN', 'BRIT_BUILD_REF'];

/** The inverse of brit's `encode_segment` (brit-epr refs.rs): the step name a notes ref carries. */
export function decodeSegment(seg) {
  return seg
    .replace(/%3A/g, ':')
    .replace(/%40/g, '@')
    .replace(/%3F/g, '?')
    .replace(/%2A/g, '*')
    .replace(/%5B/g, '[')
    .replace(/%5C/g, '\\')
    .replace(/%5E/g, '^')
    .replace(/%7E/g, '~')
    .replace(/%20/g, ' ')
    .replace(/%2E%2E/g, '..');
}

const firstLine = text => (text ?? '').trim().split('\n')[0];
/** The pinned 76ba95d8a0 `build get` on a body serde_json cannot parse (anyhow over `RefError::Json`). */
const PINNED_JSON_ERROR = /^Error: JSON error: (.*)$/;

/** A `ci:<pipeline>` step of the series: the prefix, and no further `:` (step notes stay out). */
const seriesStep = (step, prefix) => step.startsWith(prefix) && !step.slice(prefix.length).includes(':');

/**
 * The repository's signer identity for a series read: gate-evidence.mjs `signerIdentity` owns the
 * rule (`signer --json` first; the seed file only under the pinned 76ba95d8a0, which answers the
 * verb with clap's usage error, exit 2) and this asks it once, of the binary the series resolved.
 * No brit-build-ref at all is `via: 'none'` — a raw-git series names no signer.
 *
 * @returns {{agentId: string|null, via: 'signer'|'agent-key'|'none'}}
 */
export function britSigner(buildRefBin, repo, { spawn = spawnSync, env = process.env } = {}) {
  if (!buildRefBin) return { agentId: null, via: 'none' };
  return signerIdentity(repo, spawn, { env, buildRefBin });
}

/** Every series (ref, step, blob, commit) under refs/notes/brit/build/, via git — enumeration only, never a body. */
function listBuildNotes(repo, prefix) {
  const git = args => execFileSync('git', args, { cwd: repo, stdio: ['pipe', 'pipe', 'ignore'], maxBuffer: 256 * 2 ** 20 }).toString();
  let refs;
  try {
    refs = git(['for-each-ref', '--format=%(refname)', BUILD_NOTES]).split('\n').filter(Boolean);
  } catch {
    return [];
  }
  const notes = [];
  for (const ref of refs) {
    const step = decodeSegment(ref.slice(BUILD_NOTES.length));
    if (!seriesStep(step, prefix)) continue;
    for (const line of git(['notes', '--ref', ref, 'list']).split('\n').filter(Boolean)) {
      const [blob, commit] = line.split(' ');
      notes.push({ ref, step, blob, commit });
    }
  }
  return notes;
}

/** A parsed BuildAttestationContentNode → this series' row, or null when it is not one. */
function attestationRow(node, commit, prefix, { verified, reader }) {
  // brit serializes BuildAttestationContentNode camelCase (stepName, buildDurationMs, builtAt).
  if (!node || typeof node.stepName !== 'string' || typeof node.success !== 'boolean') return null;
  if (!seriesStep(node.stepName, prefix)) return null;
  return {
    step: node.stepName.slice(prefix.length),
    commit,
    success: node.success,
    durationMs: Number(node.buildDurationMs) || 0,
    harvestedAt: node.builtAt,
    inputsHashKind: node.hardwareProfile?.inputsHashKind ?? 'tree',
    orchestratorBuild: node.hardwareProfile?.orchestratorBuild,
    verified,
    reader,
  };
}

/**
 * The build attestation series plus how it was read.
 *
 *   reader = 'brit-verified'   every row passed brit's verified read (`build get`, stdout ⇔ verified)
 *          = 'brit-unverified' the pinned brit decoded the bytes but verifies nothing
 *          = 'raw-git'         no brit: git notes read directly, nothing verified
 *
 * A note brit reports `unverifiable:` is ABSENT: not a row, counted in `unverifiable`, and named
 * once through `warn` with its ref, commit and brit's reason. The pinned brit has no
 * `unverifiable:` — a body it cannot decode is `Error: JSON error: …` exit 1 (refs.rs
 * `RefError::Json` under anyhow), and only under the brit-unverified reader is that exit 1 read
 * the same way (absent, counted, named); the verified binary reports a decode failure as
 * `unverifiable: … does not decode`, exit 0, so its exit 1 is never a decode. Any other failure
 * inside brit (exit ≠ 0) throws — it is not an empty series. An unreadable repository (no git) is
 * an empty series.
 */
export function readAttestationSeries(
  repo,
  { prefix = 'ci:', buildRefBin = resolveBin('brit-build-ref', BUILD_REF_ENV, process.env), spawn = spawnSync, env = process.env, warn = console.error, signer } = {}
) {
  const notes = listBuildNotes(repo, prefix);
  if (!buildRefBin) {
    return { attestations: readRawNotes(repo, notes, prefix), reader: 'raw-git', buildRef: null, signer: null, unverifiable: 0 };
  }
  const who = signer ?? britSigner(buildRefBin, repo, { spawn, env });
  const reader = who.via === 'signer' ? 'brit-verified' : 'brit-unverified';
  const verified = reader === 'brit-verified';
  const attestations = [];
  let unverifiable = 0;
  for (const n of notes) {
    const r = spawn(buildRefBin, ['--repo', repo, 'build', 'get', '--step', n.step, '--commit', n.commit], {
      cwd: repo,
      encoding: 'utf8',
      env,
      timeout: 60_000,
      maxBuffer: 16 * 2 ** 20,
    });
    if (r.status !== 0) {
      const head = firstLine(r.stderr);
      const pinnedDecode = reader === 'brit-unverified' && r.status === 1 ? PINNED_JSON_ERROR.exec(head) : null;
      if (pinnedDecode) {
        unverifiable += 1;
        warn(`delivery-series: unverifiable ${n.ref} @ ${n.commit}: does not decode: ${pinnedDecode[1]} (pinned brit-build-ref exit 1) — read as absent`);
        continue;
      }
      throw new Error(`brit-build-ref build get --step ${n.step} --commit ${n.commit}: exit ${r.status ?? r.signal}: ${head || 'no stderr'}`);
    }
    if ((r.stdout ?? '').trim() === '') {
      const [refused] = unverifiableNotes(r.stderr);
      if (refused) {
        unverifiable += 1;
        warn(`delivery-series: unverifiable ${n.ref} @ ${refused.commit}: ${refused.reason} — read as absent`);
      } else {
        warn(`delivery-series: ${n.ref} @ ${n.commit} is listed but brit-build-ref reports no build ref — read as absent`);
      }
      continue;
    }
    let node;
    try {
      node = JSON.parse(r.stdout);
    } catch {
      throw new Error(`brit-build-ref build get --step ${n.step} --commit ${n.commit}: stdout is not JSON`);
    }
    const row = attestationRow(node, n.commit, prefix, { verified, reader });
    if (row) attestations.push(row);
  }
  return { attestations, reader, buildRef: buildRefBin, signer: who.agentId, unverifiable };
}

/** The raw-git reader: one `cat-file --batch` over every note body, nothing verified. */
function readRawNotes(repo, notes, prefix) {
  if (notes.length === 0) return [];
  // "<sha> blob <size>\n<body>\n" per object.
  const out = execFileSync('git', ['cat-file', '--batch'], {
    cwd: repo,
    input: notes.map(n => n.blob).join('\n') + '\n',
    stdio: ['pipe', 'pipe', 'ignore'],
    maxBuffer: 256 * 2 ** 20,
  });
  const attestations = [];
  let at = 0;
  for (const n of notes) {
    const eol = out.indexOf(0x0a, at);
    const [, type, size] = out.subarray(at, eol).toString().split(' ');
    const body = out.subarray(eol + 1, eol + 1 + Number(size)).toString();
    at = eol + 1 + Number(size) + 1;
    if (type !== 'blob') continue;
    try {
      const row = attestationRow(JSON.parse(body), n.commit, prefix, { verified: false, reader: 'raw-git' });
      if (row) attestations.push(row);
    } catch {
      // Not an attestation body; skip it.
    }
  }
  return attestations;
}

/**
 * Every ci:<pipeline> build attestation note in `repo` →
 * [{step, commit, success, durationMs, harvestedAt, inputsHashKind, orchestratorBuild, verified, reader}].
 * `readAttestationSeries` with only the rows; see it for the reader semantics.
 */
export function readBuildAttestations(repo, options = {}) {
  return readAttestationSeries(repo, options).attestations;
}

/** Notes lack a recoverable wave plan: a successful subset cannot prove delivery. */
export function attestationRuns(attestations) {
  const byCommit = new Map();
  for (const a of attestations) {
    if (!byCommit.has(a.commit)) byCommit.set(a.commit, []);
    byCommit.get(a.commit).push(a);
  }
  return [...byCommit].map(([commit, builds]) => ({
    number: builds.find(b => typeof b.orchestratorBuild === 'number')?.orchestratorBuild ?? commit.slice(0, 12),
    commit,
    result: null,
    minutes: null,
    planned: null,
    outcome: Object.fromEntries(builds.map(b => [b.step, b.success ? 'SUCCESS' : 'FAILURE'])),
    delivered: 'unknown',
    // A run is verified only when every note in it came through brit's verified read.
    verified: builds.every(b => b.verified === true),
    timedOut: null,
    harvestedAt: builds.map(b => b.harvestedAt).filter(Boolean).sort().at(-1) ?? null,
  })).sort((x, y) => (y.harvestedAt ?? '').localeCompare(x.harvestedAt ?? ''));
}

/** 0 within bounds · 1 outside · 2 insufficient evidence. */
export function verdict(summary, { minRate = 0.8, maxP90Min = 240 } = {}) {
  if (summary.runs.some(r => r.delivered === 'unknown')) return 2;
  if (summary.considered < Math.min(summary.window, 3)) return 2;
  if (summary.rate < minRate) return 1;
  if (summary.p90DeliveredMin == null || summary.p90DeliveredMin > maxP90Min) return 1;
  return 0;
}

// ── --stages: per-stage wall clock and cost per delivered bundle ─────────────────────────

const PUBLISH_VERIFY_STAGE = /\bpublish\b.*\bverify\b/i;
const DEPLOY_CLASS = 'elohim-app.deploy.';
const PHASE_TOKEN = /^(readiness|(?:publish|verify|converge)\.[a-z0-9-]+)$/;
const PASSED = new Set(['PASSED', 'FIXED']);
const PHASES = ['readiness', 'publish', 'verify', 'converge'];
// A stage Jenkins marks FAILED/ABORTED only because an EARLIER stage failed still reports a
// sub-second duration (#1725: "Build Image FAILED 0.0"). It never ran, so it has no wall clock.
const SKIPPED_AFTER_FAILURE_MS = 1000;

function executed(s) {
  if (!s.result || s.result === 'NOT_BUILT') return false;
  return !((s.result === 'FAILURE' || s.result === 'ABORTED') && s.durationMs < SKIPPED_AFTER_FAILURE_MS);
}

/**
 * wfapi/describe → [{name, result, durationMs}], normalized exactly as
 * scripts/pipeline-trajectory.mjs getBuildStages does (FAILED→FAILURE,
 * NOT_EXECUTED→NOT_BUILT, IN_PROGRESS→null). Pure; the fetch is the caller's.
 */
export function normalizeStages(describe) {
  return (describe?.stages ?? []).map(s => ({
    name: s.name,
    result:
      s.status === 'FAILED'
        ? 'FAILURE'
        : s.status === 'NOT_EXECUTED'
          ? 'NOT_BUILT'
          : s.status === 'IN_PROGRESS'
            ? null
            : s.status,
    durationMs: s.durationMillis ?? 0,
  }));
}

/** The App pipeline's concern-scoped phase cases from a junit testReport body. */
export function phaseCases(testReport) {
  const out = [];
  for (const suite of testReport?.suites ?? []) {
    for (const c of suite.cases ?? []) {
      if (!String(c.className ?? '').startsWith(DEPLOY_CLASS)) continue;
      const token = String(c.name ?? '').split(/\s+/)[0];
      if (!PHASE_TOKEN.test(token)) continue;
      out.push({ token, phase: token.split('.')[0], passed: PASSED.has(c.status), seconds: Number(c.duration) || 0 });
    }
  }
  return out;
}

/**
 * One build → its phase split and whether it delivered a bundle.
 * Phased: minutes per phase = the sum of that phase's leg times (the Jenkinsfile's
 * emitAppDeployJunit times each leg; legs run in sequence, so the sum is the phase's clock);
 * delivered iff every `verify.shell` case passed (a refused readiness emits no later leg). Unphased: the "Publish and Verify" stage is the whole publish+verify figure
 * (readiness included and NOT split out) and delivered iff that stage succeeded.
 */
export function stageRun(build, stages, phases) {
  const byPhase = Object.fromEntries(PHASES.map(p => [p, null]));
  let split;
  let publishVerifyMin = null;
  let delivered = 0;
  if (phases.length > 0) {
    split = 'phases';
    const seconds = {};
    for (const c of phases) seconds[c.phase] = (seconds[c.phase] ?? 0) + c.seconds;
    for (const p of PHASES) if (seconds[p] != null) byPhase[p] = seconds[p] / 60;
    if (seconds.publish != null || seconds.verify != null) {
      publishVerifyMin = ((seconds.publish ?? 0) + (seconds.verify ?? 0)) / 60;
    }
    const shells = phases.filter(c => c.token === 'verify.shell');
    delivered = shells.length > 0 && shells.every(c => c.passed) ? 1 : 0;
  } else {
    split = 'stage';
    const pv = stages.find(s => PUBLISH_VERIFY_STAGE.test(s.name) && executed(s));
    if (pv) {
      publishVerifyMin = pv.durationMs / 60_000;
      delivered = pv.result === 'SUCCESS' ? 1 : 0;
    }
  }
  return {
    number: build.number,
    result: build.result,
    minutes: Math.round(build.durationMs / 6000) / 10,
    durationMs: build.durationMs,
    split,
    phases: byPhase,
    publishVerifyMin,
    delivered,
    stages: stages.filter(executed),
  };
}

const tenth = v => (v == null ? null : Math.round(v * 10) / 10);
const hundredth = v => Math.round(v * 100) / 100;

/**
 * p50/p90 over the PRESENT samples only, with their count. A build that did not run a
 * stage or phase contributes no sample (null/undefined/NaN), so an absent stage reads
 * n=0 with null percentiles — never a number, and never a NaN that poisons the sort.
 */
function spread(values) {
  const sorted = values.filter(Number.isFinite).sort((a, b) => a - b);
  return { n: sorted.length, p50Min: tenth(percentile(sorted, 50)), p90Min: tenth(percentile(sorted, 90)) };
}

/** Per-stage and per-phase p50/p90 plus cost per delivered bundle over the newest `window` builds. */
export function summarizeStages(runs, { window = 10 } = {}) {
  const recent = runs.filter(Boolean).slice(0, window);
  const names = [...new Set(recent.flatMap(r => r.stages.map(s => s.name)))];
  const stages = Object.fromEntries(
    names.map(name => [
      name,
      spread(recent.map(r => {
        const stage = r.stages.find(s => s.name === name);
        return stage ? stage.durationMs / 60_000 : null;
      })),
    ])
  );
  const phases = Object.fromEntries(PHASES.map(p => [p, spread(recent.map(r => r.phases[p]))]));
  const delivered = recent.reduce((n, r) => n + r.delivered, 0);
  const pipelineHours = hundredth(recent.reduce((h, r) => h + r.durationMs, 0) / 3_600_000);
  return {
    window,
    considered: recent.length,
    delivered,
    pipelineHours,
    // Zero delivered is an UNBOUNDED cost, never 0 — a reader must not see "free".
    costPerDeliveredHours: delivered > 0 ? hundredth(pipelineHours / delivered) : null,
    publishVerify: spread(recent.map(r => r.publishVerifyMin)),
    phases,
    split: { phases: recent.filter(r => r.split === 'phases').length, stage: recent.filter(r => r.split === 'stage').length },
    stages,
    runs: recent,
  };
}

/** Mirrors of the declared bounds (measures.yaml stage-wallclock-ceiling@1, delivery-cost-ceiling@1). */
export const STAGE_BOUNDS = Object.freeze({ maxPublishVerifyP90Min: 20, maxCostHours: 1 });

/** 0 within bounds · 1 outside · 2 insufficient evidence. */
export function stagesVerdict(
  summary,
  { maxPublishVerifyP90Min = STAGE_BOUNDS.maxPublishVerifyP90Min, maxCostHours = STAGE_BOUNDS.maxCostHours } = {}
) {
  if (summary.considered < Math.min(summary.window, 3)) return 2;
  if (summary.costPerDeliveredHours == null || summary.costPerDeliveredHours > maxCostHours) return 1;
  if (summary.publishVerify.p90Min != null && summary.publishVerify.p90Min > maxPublishVerifyP90Min) return 1;
  return 0;
}

async function getJson(path) {
  const res = await fetch(JENKINS + path, { signal: AbortSignal.timeout(60_000) });
  if (!res.ok) throw new Error(`${res.status} ${path}`);
  return res.json();
}

async function artifact(number, names, name) {
  if (!names.has(name)) return null;
  try {
    return await getJson(`${JOB}/${number}/artifact/${name}`);
  } catch {
    return null;
  }
}

async function fetchRuns(limit) {
  const tree = encodeURIComponent(`builds[number,result,duration,artifacts[relativePath]]{0,${limit}}`);
  const { builds } = await getJson(`${JOB}/api/json?tree=${tree}`);
  const completed = builds.filter(b => b.result);
  return Promise.all(
    completed.map(async b => {
      const names = new Set((b.artifacts ?? []).map(a => a.relativePath));
      const [predicted, actual] = await Promise.all([
        artifact(b.number, names, 'predicted-build-graph.json'),
        artifact(b.number, names, 'actual-build-graph.json'),
      ]);
      return classifyRun({ number: b.number, result: b.result, durationMs: b.duration }, predicted, actual);
    })
  );
}

async function fetchStageRuns(job, limit) {
  const tree = encodeURIComponent(`builds[number,result,duration]{0,${limit}}`);
  const base = `/job/${job}/job/dev`;
  const { builds } = await getJson(`${base}/api/json?tree=${tree}`);
  const completed = builds.filter(b => b.result).slice(0, limit);
  return Promise.all(
    completed.map(async b => {
      const [describe, report] = await Promise.all([
        getJson(`${base}/${b.number}/wfapi/describe`).catch(() => null),
        getJson(`${base}/${b.number}/testReport/api/json?tree=suites[cases[className,name,status,duration]]`).catch(() => null),
      ]);
      return stageRun({ number: b.number, result: b.result, durationMs: b.duration }, normalizeStages(describe), phaseCases(report));
    })
  );
}

function fmt(v, unit = 'm') {
  return v == null ? '—' : `${v}${unit}`;
}

async function stagesMain(argv) {
  const window = arg(argv, '--window', 10);
  const maxPublishVerifyP90Min = arg(argv, '--max-publish-verify-p90-min', STAGE_BOUNDS.maxPublishVerifyP90Min);
  const maxCostHours = arg(argv, '--max-cost-hours', STAGE_BOUNDS.maxCostHours);
  const i = argv.indexOf('--pipelines');
  const jobs = (i >= 0 && argv[i + 1] ? argv[i + 1] : 'elohim').split(',').filter(Boolean);
  const bounds = { maxPublishVerifyP90Min, maxCostHours };
  const pipelines = {};
  let code = 0;
  for (const job of jobs) {
    let runs;
    try {
      runs = await fetchStageRuns(job, window);
    } catch (e) {
      console.error(`delivery-series --stages: ${job} unreadable — ${e.message}. NOT MEASURED.`);
      return 2;
    }
    const summary = summarizeStages(runs, { window });
    const exit = stagesVerdict(summary, bounds);
    code = Math.max(code, exit);
    pipelines[job] = { ...summary, exit };
  }
  if (argv.includes('--json')) {
    console.log(JSON.stringify({ pipelines, bounds, exit: code }, null, 2));
    return code;
  }
  for (const [job, s] of Object.entries(pipelines)) {
    console.log(`## ${job} — last ${s.considered} builds`);
    for (const r of s.runs) {
      const split =
        r.split === 'phases'
          ? `readiness ${fmt(tenth(r.phases.readiness))} · publish ${fmt(tenth(r.phases.publish))} · verify ${fmt(tenth(r.phases.verify))}`
          : `publish+verify ${fmt(tenth(r.publishVerifyMin))} (unsplit: readiness wait inside the stage)`;
      console.log(`#${r.number} ${r.result} ${r.minutes}m — ${split} — ${r.delivered ? 'DELIVERED' : 'not delivered'}`);
    }
    for (const [name, st] of Object.entries(s.stages).sort((a, b) => (b[1].p90Min ?? 0) - (a[1].p90Min ?? 0))) {
      if ((st.p90Min ?? 0) < 1) continue;
      console.log(`  stage ${name}: p50 ${fmt(st.p50Min)} p90 ${fmt(st.p90Min)} (n=${st.n})`);
    }
    const pv = s.publishVerify;
    console.log(
      `publish+verify p50 ${fmt(pv.p50Min)} p90 ${fmt(pv.p90Min)} (n=${pv.n}, bound ≤${maxPublishVerifyP90Min}m) · ` +
        `split: ${s.split.phases} phased / ${s.split.stage} unsplit · ` +
        `${s.pipelineHours} pipeline-h for ${s.delivered} delivered → ` +
        `cost ${s.costPerDeliveredHours == null ? '∞ (nothing delivered)' : `${s.costPerDeliveredHours}h`} per bundle ` +
        `(bound ≤${maxCostHours}h) → ${['WITHIN BOUNDS', 'OUTSIDE BOUNDS', 'NOT ENOUGH EVIDENCE'][s.exit]}`
    );
  }
  return code;
}

function arg(argv, flag, fallback) {
  const i = argv.indexOf(flag);
  return i >= 0 && argv[i + 1] !== undefined ? Number(argv[i + 1]) : fallback;
}

function strArg(argv, flag, fallback) {
  const i = argv.indexOf(flag);
  return i >= 0 && argv[i + 1] !== undefined ? argv[i + 1] : fallback;
}

async function main(argv) {
  const from = strArg(argv, '--from', 'jenkins');
  if (from !== null && from !== 'jenkins' && from !== 'attestations') {
    console.error(`delivery-series: --from ${from} — expected jenkins or attestations. NOT MEASURED.`);
    return 2;
  }
  if (argv.includes('--stages')) {
    if (from === 'attestations') {
      console.error('delivery-series --stages: stage and phase times are not in a build attestation. NOT MEASURED.');
      return 2;
    }
    return stagesMain(argv);
  }
  const window = arg(argv, '--window', 10);
  const minRate = arg(argv, '--min-rate', 0.8);
  const maxP90Min = arg(argv, '--max-p90-min', 240);
  let runs;
  const source = from;
  const partial = source === 'attestations' ? 'partial: commit-level view; wave wall-clock unknown' : null;
  let verification = null;
  if (source === 'attestations') {
    const repo = strArg(argv, '--repo', process.cwd());
    let series;
    try {
      series = readAttestationSeries(repo);
    } catch (e) {
      console.error(`delivery-series: brit build attestations in ${repo} unreadable — ${e.message}. NOT MEASURED.`);
      return 2;
    }
    runs = attestationRuns(series.attestations);
    verification = {
      reader: series.reader,
      buildRef: series.buildRef,
      signer: series.signer,
      attestations: series.attestations.length,
      unverifiable: series.unverifiable,
    };
    if (runs.length === 0) {
      // An all-refused repository is not an empty one: the count is named here, not only per note.
      const refused = series.unverifiable > 0 ? ` (${series.unverifiable} listed, every one unverifiable — read as absent)` : '';
      console.error(`delivery-series: no brit build attestations under ${BUILD_NOTES} in ${repo}${refused}.`);
    }
  }
  if (source === 'jenkins') {
    try {
      runs = await fetchRuns(window * 4);
    } catch (e) {
      console.error(`delivery-series: Jenkins unreadable — ${e.message}. NOT MEASURED.`);
      return 2;
    }
  }
  const summary = summarize(runs, { window });
  const code = partial ? 2 : verdict(summary, { minRate, maxP90Min });
  if (argv.includes('--json')) {
    console.log(JSON.stringify({ source, ...(partial ? { partial } : {}), ...(verification ? { verification } : {}), ...summary, bounds: { minRate, maxP90Min }, exit: code }, null, 2));
    return code;
  }
  if (partial) {
    console.log(partial);
    console.log(`source: ${source}${verificationLabel(verification)}`);
    for (const r of summary.runs) console.log(`#${r.number} delivered: unknown; harvestedAt: ${r.harvestedAt ?? 'unknown'} — ${JSON.stringify(r.outcome)}`);
    return code;
  }
  for (const r of summary.runs) {
    const cells = r.planned.map(n => `${n}=${r.outcome[n]}`).join(' ');
    console.log(`#${r.number} ${r.result ?? '?'} ${r.minutes}m ${r.delivered ? 'DELIVERED' : 'not delivered'}${r.timedOut ? ' (timeout)' : ''} — ${cells}`);
  }
  const pct = summary.rate == null ? 'n/a' : `${Math.round(summary.rate * 100)}%`;
  console.log(
    `delivered ${summary.delivered}/${summary.considered} (${pct}, bound ≥${Math.round(minRate * 100)}%) · ` +
      `timeouts ${summary.timedOut ?? '—'} · p50 ${summary.p50DeliveredMin ?? '—'}m p90 ${summary.p90DeliveredMin ?? '—'}m ` +
      `(bound ≤${maxP90Min}m) · last delivered #${summary.lastDelivered ?? '—'} → ` +
      ['WITHIN BOUNDS', 'OUTSIDE BOUNDS', 'NOT ENOUGH EVIDENCE'][code] +
      ` (source: ${source})`
  );
  return code;
}

/** The reader behind an attestation series, said beside the source; unverified is never implied. */
function verificationLabel(v) {
  if (!v) return '';
  if (v.reader === 'brit-verified') return ', brit-verified';
  if (v.reader === 'brit-unverified') return `, UNVERIFIED — brit-build-ref ${v.buildRef} reads notes without verifying them`;
  return ', UNVERIFIED — raw git notes, no brit-build-ref on BRIT_BUILD_REF_BIN or PATH';
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  main(process.argv.slice(2)).then(code => process.exit(code));
}
