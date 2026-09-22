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
// CLI:  node genesis/orchestrator/delivery-series.mjs [--window 10] [--min-rate 0.8]
//         [--max-p90-min 240] [--json]
// Exit: 0 within the declared bounds · 1 outside them · 2 not enough evidence
//       (missing data is never read as health).

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

function arg(argv, flag, fallback) {
  const i = argv.indexOf(flag);
  return i >= 0 && argv[i + 1] !== undefined ? Number(argv[i + 1]) : fallback;
}

async function main(argv) {
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
