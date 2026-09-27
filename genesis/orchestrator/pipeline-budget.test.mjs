/**
 * Pipeline budget invariant — the orchestrator never times out a downstream.
 *
 * Each dispatchable pipeline owns its wall-clock budget in its own Jenkinsfile
 * `options { timeout(...) }`. The orchestrator dispatches selected pipelines in
 * dependency levels, one level after another, and waits for each. Its own
 * `options { timeout }` must therefore cover the LONGEST dependency chain of
 * those budgets plus its own non-dispatch stages; otherwise a healthy but slow
 * upstream uses the parent's clock and a later level is never dispatched.
 *
 * That is exactly how app delivery starved from 2026-09-14 to 09-22:
 * 8ebce05a3 ordered app after edge (DNA → edge → app → genesis) inside a flat
 * 240-minute orchestrator budget, a6ba209e2 then raised edge's own budget to
 * 240, and orchestrators #1885-#1891 hit the parent limit while app was still
 * waiting or 2 minutes into `ng build` — zero app deliveries in eight dispatches.
 *
 * The budget sources stay where they already live (the Jenkinsfiles); no
 * manifest field is added, because the authoritative build-manifest schema is
 * the operator-owned rakia source schema.
 *
 * BARRIER_PAIRS (orchestrator Jenkinsfile, one-head-delivered Lane C2): a
 * producer → consumer dependency that lives INSIDE the consumer as barriers
 * (the happ candidate and the DNA verdict for elohim-holochain → elohim-edge).
 * The orchestrator starts the pair in the same level, so the pair costs
 * max(producer, consumer), not their sum: DNA → edge → app → genesis
 * (240+240+180+90) became conductor → max(DNA, edge) → app → genesis
 * (90+240+180+90). The map is read from the Jenkinsfile — one source.
 *
 * Run: node --test genesis/orchestrator/pipeline-budget.test.mjs
 */

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadPipelineRegistry } from './pipeline-registry.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const ORCHESTRATOR = 'elohim-orchestrator';

// The orchestrator's own stages outside Execute Builds (checkout, plan,
// pre-flight, post graph/reconcile). Measured: a no-dispatch run is ~1 min and
// an app-only run spends ~2.5 min beyond app itself; 30 keeps a wide margin.
const ORCHESTRATOR_OWN_STAGES_MINUTES = 30;

const UNIT_MINUTES = { MINUTES: 1, HOURS: 60 };

/** The pipeline-level `options { timeout(...) }`, in minutes; null when absent. */
export function ownTimeoutMinutes(jenkinsfileText) {
  const pipelineAt = jenkinsfileText.search(/^pipeline\s*\{/m);
  if (pipelineAt < 0) return null;
  // The first block-form `options {` after `pipeline {` is the pipeline's own
  // (stage-level options come later, inside stages); indentation is per file.
  const options = jenkinsfileText.slice(pipelineAt).match(/^([ \t]+)options\s*\{[ \t]*\n([\s\S]*?)^\1\}/m);
  if (!options) return null;
  const timeout = options[2].match(/timeout\(\s*time:\s*(\d+)\s*,\s*unit:\s*'(MINUTES|HOURS)'/);
  if (!timeout) return null;
  return Number(timeout[1]) * UNIT_MINUTES[timeout[2]];
}

function budgets() {
  const registry = loadPipelineRegistry(ROOT);
  const own = new Map();
  const unreadable = [];
  for (const p of registry.values()) {
    if (!p.jenkinsPath || p.manualOnly || p.pipeline === ORCHESTRATOR) continue;
    const file = resolve(ROOT, p.jenkinsPath);
    if (!existsSync(file)) {
      // Submodule-hosted Jenkinsfiles (sophia, che-devworkspaces) are absent in
      // a worktree without initialized submodules. Their chains are shorter
      // than DNA → edge → app → genesis today; when present they are counted.
      unreadable.push(p.pipeline);
      continue;
    }
    own.set(p.pipeline, ownTimeoutMinutes(readFileSync(file, 'utf8')));
  }
  return { registry, own, unreadable };
}

/** `@Field Map BARRIER_PAIRS = ['producer': 'consumer', …]` → Map(producer → consumer). */
export function barrierPairs(jenkinsfileText) {
  const decl = jenkinsfileText.match(/^@Field Map BARRIER_PAIRS = \[([^\]]*)\]/m);
  if (!decl) return new Map();
  return new Map([...decl[1].matchAll(/'([^']+)'\s*:\s*'([^']+)'/g)].map((m) => [m[1], m[2]]));
}

/**
 * Longest dependency chain by summed own budget, over dispatchable pipelines.
 * A barrier producer (barriers: producer → consumer) starts in its consumer's
 * level, after the consumer's other dependencies: the pair costs the longer
 * of the two, never their sum.
 */
export function longestChain(registry, own, barriers = new Map()) {
  const memo = new Map();
  const visit = (name, stack = new Set()) => {
    if (memo.has(name)) return memo.get(name);
    if (stack.has(name)) throw new Error(`dependsOn cycle through ${name}`);
    stack.add(name);
    let best = { minutes: 0, chain: [] };
    let span = { minutes: own.get(name) ?? 0, label: name };
    for (const dep of registry.get(name)?.dependsOn ?? []) {
      if (!own.has(dep)) continue;
      const sub = visit(dep, stack);
      if (barriers.get(dep) === name) {
        // Its own upstream still precedes the level; its budget runs beside ours.
        const depOwn = own.get(dep) ?? 0;
        const upstream = { minutes: sub.minutes - depOwn, chain: sub.chain.slice(0, -1) };
        if (upstream.minutes > best.minutes) best = upstream;
        if (depOwn > span.minutes) span = { minutes: depOwn, label: span.label };
        span.label = `max(${dep}, ${name})`;
        continue;
      }
      if (sub.minutes > best.minutes) best = sub;
    }
    stack.delete(name);
    const result = { minutes: best.minutes + span.minutes, chain: [...best.chain, span.label] };
    memo.set(name, result);
    return result;
  };
  let longest = { minutes: 0, chain: [] };
  for (const name of own.keys()) {
    const r = visit(name);
    if (r.minutes > longest.minutes) longest = r;
  }
  return longest;
}

test('ownTimeoutMinutes reads only the pipeline-level options block', () => {
  const text = [
    'def helper() {',
    "    timeout(time: 5, unit: 'MINUTES') { sh 'x' }",
    '}',
    'pipeline {',
    '    options {',
    "        timeout(time: 3, unit: 'HOURS')  // comment",
    '        disableConcurrentBuilds()',
    '    }',
    '    stages {',
    "        stage('a') { options { timeout(time: 9, unit: 'MINUTES') } }",
    '    }',
    '}',
  ].join('\n');
  assert.equal(ownTimeoutMinutes(text), 180);
  assert.equal(ownTimeoutMinutes('pipeline {\n    options {\n        skipDefaultCheckout(true)\n    }\n}'), null);
});

test('every dispatchable pipeline owns a wall-clock budget in its Jenkinsfile', () => {
  const { own } = budgets();
  const missing = [...own].filter(([, minutes]) => minutes == null).map(([name]) => name);
  assert.deepEqual(
    missing,
    [],
    `pipelines without an options { timeout(...) } — an unbounded downstream can hold the ` +
      `orchestrator's clock indefinitely: ${missing.join(', ')}`
  );
});

test('barrierPairs reads the orchestrator map; a barrier pair costs max, not sum', () => {
  assert.deepEqual(
    [...barrierPairs("x\n@Field Map BARRIER_PAIRS = ['p': 'c', 'q':'d']\ny")],
    [['p', 'c'], ['q', 'd']]
  );
  assert.equal(barrierPairs('no map here').size, 0);
  const registry = new Map([
    ['up', { dependsOn: [] }],
    ['p', { dependsOn: [] }],
    ['c', { dependsOn: ['p', 'up'] }],
    ['app', { dependsOn: ['c'] }],
  ]);
  const own = new Map([['up', 90], ['p', 240], ['c', 200], ['app', 180]]);
  assert.equal(longestChain(registry, own).minutes, 240 + 200 + 180);
  const barriered = longestChain(registry, own, new Map([['p', 'c']]));
  assert.equal(barriered.minutes, 90 + 240 + 180);
  assert.deepEqual(barriered.chain, ['up', 'max(p, c)', 'app']);
});

test('elohim-holochain → elohim-edge is a barrier pair in the orchestrator', () => {
  const { registry } = budgets();
  const text = readFileSync(resolve(ROOT, registry.get(ORCHESTRATOR).jenkinsPath), 'utf8');
  assert.equal(barrierPairs(text).get('elohim-holochain'), 'elohim-edge');
});

test("the orchestrator's budget covers the longest dependency chain, so it never times out a downstream", () => {
  const { registry, own, unreadable } = budgets();
  const orchestrator = registry.get(ORCHESTRATOR);
  const orchestratorText = readFileSync(resolve(ROOT, orchestrator.jenkinsPath), 'utf8');
  const ceiling = ownTimeoutMinutes(orchestratorText);
  const longest = longestChain(registry, own, barrierPairs(orchestratorText));
  const required = longest.minutes + ORCHESTRATOR_OWN_STAGES_MINUTES;
  assert.ok(
    ceiling >= required,
    `orchestrator timeout ${ceiling}m < longest chain ${longest.chain.join(' → ')} ` +
      `(${longest.minutes}m) + own stages (${ORCHESTRATOR_OWN_STAGES_MINUTES}m) = ${required}m. ` +
      `Shrink a downstream budget or raise the orchestrator ceiling to the derived sum` +
      (unreadable.length ? ` (not counted, Jenkinsfile absent: ${unreadable.join(', ')})` : '')
  );
});
