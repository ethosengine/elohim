/**
 * Baseline-hold lint
 *
 * Guards the rule "a failed orchestrator run must not absorb its own diff".
 *
 * The bug (orchestrator/dev #1875): the 'plan' checkpoint advances
 * `__global__` to the build's own commit and writes it back to
 * env.PIPELINE_BASELINES. At 'post', a FAILURE/ABORTED result "preserved
 * prior" — but prior was already the build's own commit, so the diff that
 * dispatched the aborted pipeline was absorbed and the next push dispatched
 * nothing, while the log claimed "baseline held so the next push re-dispatches".
 *
 * The hold only works if the failed-post branch restores the baseline the run
 * LOADED, not the one an earlier checkpoint of the same run wrote.
 *
 * Run:
 *   node --test genesis/orchestrator/baseline-hold.test.mjs
 */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const source = readFileSync(join(here, 'Jenkinsfile'), 'utf8');

function functionBody(name) {
  const start = source.indexOf(`def ${name}(`);
  assert.notEqual(start, -1, `${name} not found in Jenkinsfile`);
  const next = source.indexOf('\ndef ', start + 1);
  return source.slice(start, next === -1 ? undefined : next);
}

test('auto mode records the global baseline it loaded', () => {
  const body = functionBody('autoModeAnalyze');
  assert.match(body, /env\.LOADED_GLOBAL_BASELINE\s*=/);
});

test('a failed post checkpoint restores the loaded global baseline', () => {
  const body = functionBody('archivePipelineBaselines');
  const failedBranch = body.slice(body.indexOf("['FAILURE', 'ABORTED', 'NOT_BUILT']"));
  assert.match(
    failedBranch,
    /baselines\['__global__'\]\s*=\s*env\.LOADED_GLOBAL_BASELINE/,
    'failed post must restore env.LOADED_GLOBAL_BASELINE, not keep the plan checkpoint value',
  );
});
