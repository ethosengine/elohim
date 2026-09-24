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
// CLI:  node genesis/orchestrator/delivery-series.mjs [--window 10] [--min-rate 0.8]
//         [--max-p90-min 240] [--json]
//       node genesis/orchestrator/delivery-series.mjs --stages [--pipelines elohim]
//         [--window 10] [--max-publish-verify-p90-min 20] [--max-cost-hours 1] [--json]
// Exit: 0 within the declared bounds · 1 outside them · 2 not enough evidence
//       (missing data is never read as health).
// The --stages defaults mirror the bounds declared in .claude/epr-meta/measures.yaml
// (stage-wallclock-ceiling@1, delivery-cost-ceiling@1); the test pins the two together.

import { pathToFileURL } from 'node:url';

const JENKINS = 'https://jenkins.ethosengine.com';
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
  const delivered = recent.filter(r => r.delivered);
  const minutes = delivered.map(r => r.minutes).sort((a, b) => a - b);
  return {
    window,
    considered: recent.length,
    delivered: delivered.length,
    rate: recent.length ? delivered.length / recent.length : null,
    timedOut: recent.filter(r => r.timedOut).length,
    p50DeliveredMin: percentile(minutes, 50),
    p90DeliveredMin: percentile(minutes, 90),
    lastDelivered: delivered[0]?.number ?? null,
    runs: recent,
  };
}

/** 0 within bounds · 1 outside · 2 insufficient evidence. */
export function verdict(summary, { minRate = 0.8, maxP90Min = 240 } = {}) {
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

async function main(argv) {
  if (argv.includes('--stages')) return stagesMain(argv);
  const window = arg(argv, '--window', 10);
  const minRate = arg(argv, '--min-rate', 0.8);
  const maxP90Min = arg(argv, '--max-p90-min', 240);
  let runs;
  try {
    runs = await fetchRuns(window * 4);
  } catch (e) {
    console.error(`delivery-series: Jenkins unreadable — ${e.message}. NOT MEASURED.`);
    return 2;
  }
  const summary = summarize(runs, { window });
  const code = verdict(summary, { minRate, maxP90Min });
  if (argv.includes('--json')) {
    console.log(JSON.stringify({ ...summary, bounds: { minRate, maxP90Min }, exit: code }, null, 2));
    return code;
  }
  for (const r of summary.runs) {
    const cells = r.planned.map(n => `${n}=${r.outcome[n]}`).join(' ');
    console.log(`#${r.number} ${r.result ?? '?'} ${r.minutes}m ${r.delivered ? 'DELIVERED' : 'not delivered'}${r.timedOut ? ' (timeout)' : ''} — ${cells}`);
  }
  const pct = summary.rate == null ? 'n/a' : `${Math.round(summary.rate * 100)}%`;
  console.log(
    `delivered ${summary.delivered}/${summary.considered} (${pct}, bound ≥${Math.round(minRate * 100)}%) · ` +
      `timeouts ${summary.timedOut} · p50 ${summary.p50DeliveredMin ?? '—'}m p90 ${summary.p90DeliveredMin ?? '—'}m ` +
      `(bound ≤${maxP90Min}m) · last delivered #${summary.lastDelivered ?? '—'} → ` +
      ['WITHIN BOUNDS', 'OUTSIDE BOUNDS', 'NOT ENOUGH EVIDENCE'][code]
  );
  return code;
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  main(process.argv.slice(2)).then(code => process.exit(code));
}
