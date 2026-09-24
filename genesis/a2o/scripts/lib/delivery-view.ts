/**
 * delivery-view — the native "build #" page `pnpm reports:serve` renders at /delivery.
 *
 * Native-delivery sprint, Lane G2. The portal's data contract IS three existing readings,
 * joined here and never re-derived:
 *   - `epr flow walk <habit atom> --json`   → the habit's tier (evidence ladder) and cost bounds
 *   - `delivery-series.mjs --window 10 --json` → the last orchestrator runs, per planned pipeline
 *   - `delivery-series.mjs --stages --json`    → the App pipeline's stage graph, with the
 *                                               readiness case as its own node
 * The same combined object is served as /delivery.json, so the aesthetic pass (graphos,
 * Library B — captured backlog) is built on this data, not on Jenkins.
 *
 * Functional, deliberately unstyled. Pure rendering lives here; loading is `loadDeliveryData`.
 */

import { execFile } from 'node:child_process';
import { resolve } from 'node:path';
import { promisify } from 'node:util';

const run = promisify(execFile);

export const JENKINS = 'https://jenkins.ethosengine.com';
export const HABIT_ATOM = 'genesis/orchestrator/.epr-meta/push-delivers-within-budget.habit.md';
const ORCHESTRATOR_JOB = '/job/elohim-orchestrator/job/dev';
const PHASES = ['readiness', 'publish', 'verify', 'converge'] as const;
const PUBLISH_VERIFY_STAGE = /\bpublish\b.*\bverify\b/i;

/** The Jenkins-backed stages are CI evidence: rung T4 of the evidence ladder. */
const CI_TIER = 'T4';

export interface Stage {
  name: string;
  result: string | null;
  durationMs: number;
}

export interface AppRun {
  number: number;
  result: string | null;
  minutes: number;
  split: 'phases' | 'stage';
  phases: Partial<Record<string, number | null>>;
  publishVerifyMin: number | null;
  delivered: number;
  stages: Stage[];
}

export interface OrchestratorRun {
  number: number;
  result: string | null;
  minutes: number;
  planned: string[];
  outcome: Record<string, string>;
  delivered: boolean;
  timedOut: boolean;
}

export interface Reading<T> {
  /** The parsed JSON, or null when the reading could not be taken. */
  data: T | null;
  /** The command that produced it, so a reader can run it by hand. */
  command: string;
  /** Why the reading is absent — never read as health. */
  error?: string;
}

export interface DeliveryData {
  generatedAt: string;
  habit: Reading<Record<string, unknown>>;
  orchestrator: Reading<{ runs: OrchestratorRun[]; [k: string]: unknown }>;
  app: Reading<{ pipelines: Record<string, { runs: AppRun[]; [k: string]: unknown }> }>;
}

// ── loading ────────────────────────────────────────────────────────────────────────────────

async function reading<T>(cmd: string, args: string[], cwd: string): Promise<Reading<T>> {
  const command = [cmd, ...args].join(' ');
  try {
    const { stdout } = await run(cmd, args, { cwd, maxBuffer: 64 * 1024 * 1024, timeout: 300_000 });
    return { data: JSON.parse(stdout) as T, command };
  } catch (e) {
    // delivery-series exits 1 (outside bounds) and 2 (not enough evidence) with JSON on stdout:
    // an out-of-bounds reading is still a reading.
    const err = e as { stdout?: string; message?: string };
    if (err.stdout) {
      try {
        return { data: JSON.parse(err.stdout) as T, command };
      } catch {
        /* fall through to the error */
      }
    }
    return { data: null, command, error: (err.message ?? String(e)).split('\n')[0] };
  }
}

/** Take the three readings. `EPR_BIN` overrides the `epr` on PATH (a lane-built binary). */
export async function loadDeliveryData(repoRoot: string): Promise<DeliveryData> {
  const epr = process.env['EPR_BIN'] ?? 'epr';
  const series = resolve(repoRoot, 'genesis/orchestrator/delivery-series.mjs');
  const [habit, orchestrator, app] = await Promise.all([
    reading<Record<string, unknown>>(
      epr,
      ['flow', 'walk', HABIT_ATOM, '--json', '--root', repoRoot],
      repoRoot
    ),
    reading<{ runs: OrchestratorRun[]; [k: string]: unknown }>(
      'node',
      [series, '--window', '10', '--json'],
      repoRoot
    ),
    reading<{ pipelines: Record<string, { runs: AppRun[]; [k: string]: unknown }> }>(
      'node',
      [series, '--stages', '--window', '10', '--json'],
      repoRoot
    ),
  ]);
  return { generatedAt: new Date().toISOString(), habit, orchestrator, app };
}

// ── rendering ──────────────────────────────────────────────────────────────────────────────

export function esc(value: unknown): string {
  return String(value ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

const min = (ms: number) => Math.round(ms / 6000) / 10;
const tenth = (v: number | null | undefined) =>
  v === null || v === undefined ? null : Math.round(v * 10) / 10;

interface NodeSpec {
  label: string;
  result: string;
  tier: string;
  cost: string;
  href?: string;
  links?: { text: string; href: string }[];
  id?: string;
}

/** One graph node: a bordered box naming the step, its verdict, its tier and its cost. */
export function node(n: NodeSpec): string {
  const title = n.href ? `<a href="${esc(n.href)}">${esc(n.label)}</a>` : esc(n.label);
  const links = (n.links ?? []).map(l => `<a href="${esc(l.href)}">${esc(l.text)}</a>`).join(' · ');
  return (
    `<span class="node" data-node="${esc(n.id ?? n.label)}" data-result="${esc(n.result)}">` +
    `<b>${title}</b><br>${esc(n.result)}<br>tier ${esc(n.tier)} · cost ${esc(n.cost)}` +
    (links ? `<br>${links}` : '') +
    `</span>`
  );
}

const arrow = ' → ';

/** The App build's stages as a chain, the publish+verify stage expanded into its phase nodes. */
export function appRunGraph(run: AppRun, job = 'elohim'): string {
  const build = `${JENKINS}/job/${job}/job/dev/${run.number}`;
  const nodes: string[] = [];
  for (const stage of run.stages) {
    const stageNode = node({
      label: stage.name,
      result: stage.result ?? 'IN_PROGRESS',
      tier: CI_TIER,
      cost: `${min(stage.durationMs)}m`,
      href: `${build}/console`,
    });
    if (!PUBLISH_VERIFY_STAGE.test(stage.name)) {
      nodes.push(stageNode);
      continue;
    }
    // The readiness case is always its own node — split when the build recorded phase cases,
    // named as unsplit (inside this stage) when it predates them. Never silently folded away.
    if (run.split === 'phases') {
      for (const phase of PHASES) {
        const minutes = run.phases[phase];
        if (minutes === null || minutes === undefined) continue;
        nodes.push(
          node({
            id: phase,
            label: phase,
            result: phase === 'verify' && run.delivered ? 'PASSED' : 'recorded',
            tier: CI_TIER,
            cost: `${tenth(minutes)}m`,
            href: `${build}/testReport/`,
          })
        );
      }
    } else {
      nodes.push(
        node({
          id: 'readiness',
          label: 'readiness',
          result: 'UNSPLIT — inside the publish+verify stage (no phase cases recorded)',
          tier: CI_TIER,
          cost: 'unmeasured',
          href: `${build}/console`,
        })
      );
      nodes.push(stageNode);
    }
  }
  return nodes.join(arrow);
}

function costText(hours: unknown): string {
  return hours === null || hours === undefined ? '∞ (nothing delivered)' : esc(hours) + ' h';
}

function appSection(app: DeliveryData['app']): string {
  if (!app.data)
    return `<p>App stage series NOT MEASURED: ${esc(app.error)} (<code>${esc(app.command)}</code>)</p>`;
  const out: string[] = [];
  for (const [job, summary] of Object.entries(app.data.pipelines)) {
    const s = summary as Record<string, unknown> & { runs: AppRun[] };
    const pv = (s['publishVerify'] ?? {}) as { p50Min?: number; p90Min?: number; n?: number };
    out.push(
      `<h2 id="app-${esc(job)}">${esc(job)}/dev — last ${esc(s['considered'])} builds</h2>` +
        `<p>${esc(s['pipelineHours'])} pipeline-hours for ${esc(s['delivered'])} delivered → cost per delivered bundle ` +
        `${costText(s['costPerDeliveredHours'])} · ` +
        `publish+verify p50 ${esc(pv.p50Min ?? '—')}m p90 ${esc(pv.p90Min ?? '—')}m (n=${esc(pv.n ?? 0)})</p>`
    );
    out.push('<ol>');
    for (const r of s.runs) {
      const build = `${JENKINS}/job/${job}/job/dev/${r.number}`;
      out.push(
        `<li id="app-${esc(job)}-${esc(r.number)}"><a href="${esc(build)}/">#${esc(r.number)}</a> ${esc(r.result)} ` +
          `${esc(r.minutes)}m — ${r.delivered ? 'DELIVERED' : 'not delivered'} — ` +
          `<a href="${esc(build)}/console">log</a> · <a href="${esc(build)}/artifact/">artifacts</a> · ` +
          `<a href="${esc(build)}/testReport/">tests</a><div class="graph">${appRunGraph(r, job)}</div></li>`
      );
    }
    out.push('</ol>');
  }
  return out.join('\n');
}

/** One orchestrator run: its own node, then one node per pipeline it planned. */
export function orchestratorRunGraph(r: OrchestratorRun): string {
  const build = `${JENKINS}${ORCHESTRATOR_JOB}/${r.number}`;
  const head = node({
    id: `orchestrator-${r.number}`,
    label: `#${r.number}`,
    result: `${r.result ?? 'IN_PROGRESS'}${r.timedOut ? ' (timeout)' : ''} — ${r.delivered ? 'DELIVERED' : 'not delivered'}`,
    tier: CI_TIER,
    cost: `${r.minutes}m`,
    href: `${build}/`,
    links: [
      { text: 'log', href: `${build}/console` },
      { text: 'build graph', href: `${build}/artifact/actual-build-graph.json` },
    ],
  });
  const pipelines = r.planned.map(name =>
    node({
      label: name,
      result: r.outcome[name] ?? 'NOT_DISPATCHED',
      tier: CI_TIER,
      cost: '—',
      href: name === 'elohim' ? '#app-elohim' : `${JENKINS}/job/${name}/job/dev/`,
    })
  );
  return [head, ...pipelines].join(arrow);
}

function orchestratorSection(o: DeliveryData['orchestrator']): string {
  if (!o.data)
    return `<p>Orchestrator series NOT MEASURED: ${esc(o.error)} (<code>${esc(o.command)}</code>)</p>`;
  const d = o.data as Record<string, unknown> & { runs: OrchestratorRun[] };
  const rate = typeof d['rate'] === 'number' ? `${Math.round(d['rate'] * 100)}%` : 'n/a';
  return (
    `<h2>Orchestrator — last ${esc(d['considered'])} work-bearing runs</h2>` +
    `<p>delivered ${esc(d['delivered'])}/${esc(d['considered'])} (${esc(rate)}) · timeouts ${esc(d['timedOut'])} · ` +
    `p90 delivered ${esc(d['p90DeliveredMin'] ?? '—')}m</p><ol>` +
    d.runs.map(r => `<li><div class="graph">${orchestratorRunGraph(r)}</div></li>`).join('\n') +
    '</ol>'
  );
}

interface HabitJson {
  id?: string;
  status?: string;
  tier?: {
    highest_green?: { rung: string; label: string; when: string; receipt: string } | null;
    runnable_from?: string | null;
    unread?: string[];
  };
  cost?: { bounds?: { measure: string; outcome: string; summary: string }[] };
}

function habitSection(h: DeliveryData['habit']): string {
  if (!h.data)
    return `<p>Habit walk NOT MEASURED: ${esc(h.error)} (<code>${esc(h.command)}</code>)</p>`;
  const habit = h.data['habit'] as HabitJson | undefined;
  const diagnostics = (h.data['diagnostics'] ?? []) as {
    severity: string;
    code: string;
    subject: string;
    message: string;
  }[];
  if (!habit) {
    return `<p>The epr binary answered without a <code>habit</code> section — it predates Lane G1. Set EPR_BIN to a newer build. (<code>${esc(h.command)}</code>)</p>`;
  }
  const green = habit.tier?.highest_green;
  const tier = green
    ? `${green.rung} ${green.label} green at ${green.when} (${green.receipt})`
    : `no rung reads green; runnable from ${habit.tier?.runnable_from ?? '—'}`;
  const bounds = (habit.cost?.bounds ?? [])
    .map(b => `<li>${esc(b.measure)} ${esc(b.outcome)} — ${esc(b.summary)}</li>`)
    .join('');
  const diag = diagnostics
    .map(
      d =>
        '<li><code>' +
        esc(d.severity + ' ' + d.code + ' ' + d.subject + ': ' + d.message) +
        '</code></li>'
    )
    .join('');
  return (
    `<h2>Habit ${esc(habit.id)} [${esc(habit.status)}]</h2>` +
    `<p>tier: ${esc(tier)} · unread rungs: ${esc((habit.tier?.unread ?? []).join(', '))}</p>` +
    `<ul>${bounds || '<li>no cost bound names this habit</li>'}</ul>` +
    (diag ? `<ul>${diag}</ul>` : '')
  );
}

export function renderDeliveryPage(data: DeliveryData): string {
  return `<!doctype html><meta charset="utf-8"><title>Native build view</title>
<style>body{font:14px/1.5 system-ui;margin:1rem}.graph{margin:.25rem 0 .75rem}.node{display:inline-block;border:1px solid;padding:.2rem .4rem;margin:.1rem;vertical-align:top}</style>
<h1>Native build view — push-delivers-within-budget</h1>
<p>Taken ${esc(data.generatedAt)} · data contract: <a href="/delivery.json">/delivery.json</a> ·
readings: <code>${esc(data.habit.command)}</code>, <code>${esc(data.orchestrator.command)}</code>, <code>${esc(data.app.command)}</code></p>
${habitSection(data.habit)}
${orchestratorSection(data.orchestrator)}
${appSection(data.app)}`;
}
