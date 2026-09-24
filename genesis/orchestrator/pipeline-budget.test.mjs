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

/** Longest dependency chain by summed own budget, over dispatchable pipelines. */
export function longestChain(registry, own) {
  const memo = new Map();
  const visit = (name, stack = new Set()) => {
    if (memo.has(name)) return memo.get(name);
    if (stack.has(name)) throw new Error(`dependsOn cycle through ${name}`);
    stack.add(name);
    let best = { minutes: 0, chain: [] };
    for (const dep of registry.get(name)?.dependsOn ?? []) {
      if (!own.has(dep)) continue;
      const sub = visit(dep, stack);
      if (sub.minutes > best.minutes) best = sub;
    }
    stack.delete(name);
    const result = { minutes: best.minutes + (own.get(name) ?? 0), chain: [...best.chain, name] };
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

test("the orchestrator's budget covers the longest dependency chain, so it never times out a downstream", () => {
  const { registry, own, unreadable } = budgets();
  const orchestrator = registry.get(ORCHESTRATOR);
  const ceiling = ownTimeoutMinutes(readFileSync(resolve(ROOT, orchestrator.jenkinsPath), 'utf8'));
  const longest = longestChain(registry, own);
  const required = longest.minutes + ORCHESTRATOR_OWN_STAGES_MINUTES;
  assert.ok(
    ceiling >= required,
    `orchestrator timeout ${ceiling}m < longest chain ${longest.chain.join(' → ')} ` +
      `(${longest.minutes}m) + own stages (${ORCHESTRATOR_OWN_STAGES_MINUTES}m) = ${required}m. ` +
      `Shrink a downstream budget or raise the orchestrator ceiling to the derived sum` +
      (unreadable.length ? ` (not counted, Jenkinsfile absent: ${unreadable.join(', ')})` : '')
  );
});
