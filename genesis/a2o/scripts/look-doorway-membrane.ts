/**
 * Alpha probe: verify the doorway membrane policy stage is LIVE on a deployed
 * doorway — the deployment-verification counterpart to the membrane's unit
 * tests (`doorway/doorway-service/src/server/membrane.rs` + `metrics.rs`).
 *
 * The membrane source-keys every non-exempt/non-static request to doorway :8080,
 * runs a guard verdict (Allow / Shape / Challenge / Deny), and exposes
 * `doorway_membrane_verdict_total{verdict}` + `doorway_membrane_bans_active` at
 * `/metrics`. Until this stage deploys, those series are absent from alpha — so
 * this probe is also the "has it landed yet?" check.
 *
 * Honest, non-abusive scope (NEVER hammer alpha to force a Challenge/Deny — the
 * edge thresholds are shape 300 / challenge 600 / ban 1200 per 60s and a forced
 * deny would ban this probe's own source for ban_secs=900s; the 429/403 +
 * x-membrane header contract is already unit-tested). What a HEALTHY alpha shows:
 *   1. PRESENT  — both membrane series render at /metrics.
 *   2. COUNTING — one normal (non-static, non-exempt) request increments
 *      `doorway_membrane_verdict_total{verdict="allow"}` by exactly 1, proving the
 *      stage is wired into the request path and scoring live traffic.
 *   3. TRANSPARENT — a normal allowed request carries NO `x-membrane` header
 *      (the header is set only on Challenge/Deny; Allow/Shape pass through silently).
 *
 * Writes `reports/look/doorway-membrane/capture.json` + prints the dir and NOTE
 * lines (same convention as `look-resilience-panel.ts`). Exit 0 on PASS, 1 on a
 * verifiable FAIL (series absent / no increment / unexpected header), 2 on a
 * probe error (alpha unreachable).
 *
 * Usage: npx tsx scripts/look-doorway-membrane.ts [base-url]
 *   base-url defaults to https://doorway-alpha.elohim.host
 */

import { mkdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const base = (process.argv[2] ?? 'https://doorway-alpha.elohim.host').replace(/\/+$/, '');
const outDir = resolve('reports/look/doorway-membrane');
const TIMEOUT_MS = 15_000;

const VERDICT_SERIES = 'doorway_membrane_verdict_total';
const BANS_SERIES = 'doorway_membrane_bans_active';

interface ProbeResult {
  base: string;
  probedAt: string;
  pass: boolean;
  checks: { name: string; ok: boolean | null; detail: string }[];
  notes: string[];
}

async function fetchText(
  url: string
): Promise<{ status: number; body: string; xMembrane: string | null }> {
  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(), TIMEOUT_MS);
  try {
    const res = await fetch(url, { signal: ctrl.signal, redirect: 'manual' });
    const body = await res.text();
    return { status: res.status, body, xMembrane: res.headers.get('x-membrane') };
  } finally {
    clearTimeout(timer);
  }
}

/**
 * Sum a labelled counter family across all label sets — or, when `label` is
 * given, read the single series whose labels contain `verdict="<label>"`.
 * Prometheus text exposition: `name{labels} value` (HELP/TYPE comments skipped).
 */
function readCounter(metricsBody: string, series: string, verdictLabel?: string): number | null {
  let total: number | null = null;
  for (const line of metricsBody.split('\n')) {
    if (line.startsWith('#') || !line.startsWith(series)) continue;
    // The line is either `series value` (unlabelled gauge) or `series{...} value`.
    const after = line.slice(series.length);
    if (after.length > 0 && after[0] !== ' ' && after[0] !== '{') continue; // a longer metric name sharing the prefix
    if (verdictLabel !== undefined && !after.includes(`verdict="${verdictLabel}"`)) continue;
    const value = Number(line.trim().split(/\s+/).pop());
    if (!Number.isNaN(value)) total = (total ?? 0) + value;
  }
  return total;
}

async function main(): Promise<void> {
  await mkdir(outDir, { recursive: true });
  const result: ProbeResult = {
    base,
    probedAt: new Date().toISOString(),
    pass: true,
    checks: [],
    notes: [],
  };

  const record = (name: string, ok: boolean | null, detail: string): void => {
    result.checks.push({ name, ok, detail });
    result.notes.push(`${ok === null ? 'SKIP' : ok ? 'PASS' : 'FAIL'} ${name}: ${detail}`);
    if (ok === false) result.pass = false;
  };

  try {
    // ── Check 1: /metrics reachable + both membrane series PRESENT ───────────
    const before = await fetchText(`${base}/metrics`);
    if (before.status !== 200) {
      record('metrics-reachable', false, `GET /metrics → HTTP ${before.status} (expected 200)`);
      throw new Error('metrics endpoint not 200 — cannot verify membrane');
    }
    record('metrics-reachable', true, `GET /metrics → 200, ${before.body.length} bytes`);

    const verdictPresent = before.body.includes(VERDICT_SERIES);
    const bansPresent = before.body.includes(BANS_SERIES);
    record(
      'verdict-series-present',
      verdictPresent,
      verdictPresent
        ? `${VERDICT_SERIES} renders`
        : `${VERDICT_SERIES} ABSENT — membrane stage not deployed on this doorway yet`
    );
    record(
      'bans-series-present',
      bansPresent,
      bansPresent
        ? `${BANS_SERIES} renders`
        : `${BANS_SERIES} ABSENT — membrane stage not deployed on this doorway yet`
    );

    if (!verdictPresent) {
      // Stage not live — the counting/transparency checks are meaningless. Stop
      // here with a clear "not deployed" verdict rather than a misleading FAIL.
      result.notes.push(
        'NOTE: membrane series absent → this doorway has not deployed the membrane stage. ' +
          'This probe goes green once the stage lands on alpha (deploy + restart).'
      );
      throw new Error('membrane verdict series absent — stage not deployed');
    }

    const allowBefore = readCounter(before.body, VERDICT_SERIES, 'allow') ?? 0;
    record('allow-baseline', true, `${VERDICT_SERIES}{verdict="allow"} = ${allowBefore}`);

    // ── Check 2: one normal request is COUNTED (allow increments) ────────────
    // `/` is non-static (no asset dir, no Angular bundle basename) and
    // non-exempt (not /health|/metrics|WS) → it is scored by the membrane and,
    // on a healthy alpha well under threshold, returns Allow.
    const normal = await fetchText(`${base}/`);
    record('normal-request', true, `GET / → HTTP ${normal.status}`);

    // ── Check 3: an allowed request is TRANSPARENT (no x-membrane header) ─────
    // x-membrane is set ONLY on Challenge (429) / Deny (403). A normal allowed
    // request must carry no such header.
    if (normal.status === 429 || normal.status === 403) {
      record(
        'allow-transparent',
        false,
        `GET / was shaped to ${normal.status} (x-membrane=${normal.xMembrane}) — ` +
          `alpha is at/over threshold OR this probe's source is already counted; re-run later`
      );
    } else {
      record(
        'allow-transparent',
        normal.xMembrane === null,
        normal.xMembrane === null
          ? 'allowed GET / carries no x-membrane header (correct — Allow is silent)'
          : `unexpected x-membrane="${normal.xMembrane}" on a ${normal.status} response`
      );
    }

    // Re-scrape and assert the allow counter advanced by the request(s) above.
    const after = await fetchText(`${base}/metrics`);
    const allowAfter = readCounter(after.body, VERDICT_SERIES, 'allow') ?? 0;
    const delta = allowAfter - allowBefore;
    record(
      'allow-counted',
      delta >= 1,
      `${VERDICT_SERIES}{verdict="allow"} ${allowBefore} → ${allowAfter} (Δ=${delta}; ` +
        `≥1 proves the stage scores live traffic)`
    );

    const bansNow = readCounter(after.body, BANS_SERIES) ?? 0;
    record(
      'bans-gauge-readable',
      true,
      `${BANS_SERIES} = ${bansNow} (informational; healthy alpha = 0)`
    );
  } catch (e) {
    result.notes.push(`PROBE: ${e instanceof Error ? e.message : String(e)}`);
    // pass already reflects any FAIL check; a thrown error with no FAIL recorded
    // (e.g. network) is a probe error, surfaced via exit code 2 below.
  } finally {
    await writeFile(join(outDir, 'capture.json'), JSON.stringify(result, null, 2));
  }

  console.log(outDir);
  for (const n of result.notes) console.log('NOTE:', n);

  // Exit: 0 = all checks pass; 1 = a verifiable FAIL; 2 = probe could not complete.
  const anyFail = result.checks.some(c => c.ok === false);
  const completed = result.checks.some(c => c.name === 'allow-counted');
  if (anyFail) process.exit(1);
  if (!completed) process.exit(2);
  process.exit(0);
}

main().catch(e => {
  console.error(e instanceof Error ? e.message : String(e));
  process.exit(2);
});
