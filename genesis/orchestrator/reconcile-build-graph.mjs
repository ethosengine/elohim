#!/usr/bin/env node
// Reconcile predicted-build-graph.json (what the orchestrator decided in
// Determine Build Plan) against actual-build-graph.json (what Execute
// Builds actually dispatched and how each downstream pipeline finished).
//
// Library usage:   import { reconcile } from './reconcile-build-graph.mjs'
// CLI usage:       node reconcile-build-graph.mjs <predicted.json> <actual.json> <out.json>
//
// Drift between predicted and actual is the highest-value pipeline-quality
// signal — every disconnect should be a candidate for investigation toward
// predictive, compiler-validated builds.

import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { ORCHESTRATOR } from './build-artifacts.mjs';

// ── Reconciliation primitives ──────────────────────────────────────

const SUCCESSFUL_RESULTS = new Set(['SUCCESS', 'UNSTABLE']);
const UNSUCCESSFUL_TERMINAL = new Set(['FAILURE', 'ABORTED', 'NOT_BUILT', 'ERROR']);

function classifyResult(name, info) {
  const result = info?.result;
  if (result === 'SUCCESS') return 'success';
  if (result === 'UNSTABLE') return 'unstable';
  if (UNSUCCESSFUL_TERMINAL.has(result)) return 'aborted';
  // Defensive: unknown result codes treated as drift, not match.
  return 'unknown';
}

function pipelineUrl(name, info) {
  return info?.url || null;
}

/**
 * Reconcile predicted vs actual build graphs.
 *
 * Inputs are the parsed JSON objects emitted by the orchestrator. The shape
 * is documented in genesis/orchestrator/Jenkinsfile (Determine Build Plan
 * and Execute Builds emit blocks).
 *
 * @param {{ predicted: object, actual: object }} args
 * @returns {{
 *   schemaVersion: string,
 *   reconciledAt: string,
 *   commitSha: string,
 *   verdict: 'match' | 'drift',
 *   predictedPipelines: string[],
 *   actualPipelines: string[],
 *   disconnects: {
 *     predictedNotExecuted: string[],
 *     executedNotPredicted: string[],
 *     abortedAfterStart: string[],
 *     unstableResults: string[],
 *     commitShaDrift: boolean,
 *     unknownResults: string[],
 *   },
 *   summary: string,
 *   investigationPointers: string[],
 * }}
 */
export function reconcile({ predicted, actual }) {
  if (!predicted || !Array.isArray(predicted.pipelines)) {
    throw new Error('predicted.pipelines must be an array');
  }
  if (!actual || typeof actual.results !== 'object' || actual.results === null) {
    throw new Error('actual.results must be an object');
  }

  const predictedSet = new Set(predicted.pipelines);
  const actualResults = actual.results;
  const actualSet = new Set(Object.keys(actualResults));
  const abortedBeforeStart = new Set(actual.abortedBeforeStart || []);

  // predictedNotExecuted: predicted to run, but never produced a result
  // AND wasn't recorded as aborted-before-start. (abortedBeforeStart is
  // counted separately because it's the expected behaviour of the level-
  // failed cascade — predicted but skipped due to upstream failure.)
  const predictedNotExecuted = [...predictedSet].filter(
    name => !actualSet.has(name) && !abortedBeforeStart.has(name),
  );

  // For the disconnect output, surface abortedBeforeStart entries too:
  // they ARE a drift from "predicted pipeline ran" even if expected.
  for (const name of abortedBeforeStart) {
    if (predictedSet.has(name) && !actualSet.has(name)) {
      predictedNotExecuted.push(name);
    }
  }

  // executedNotPredicted: ran but the orchestrator never said it would.
  // This catches cascade bugs, commit-tag overrides not recorded in the
  // predicted artifact, and direct webhook re-triggers that bypassed
  // the planning stage.
  const executedNotPredicted = [...actualSet].filter(name => !predictedSet.has(name));

  // Result classification for pipelines that DID run.
  const abortedAfterStart = [];
  const unstableResults = [];
  const unknownResults = [];
  for (const name of actualSet) {
    const klass = classifyResult(name, actualResults[name]);
    if (klass === 'aborted') abortedAfterStart.push(name);
    else if (klass === 'unstable') unstableResults.push(name);
    else if (klass === 'unknown') unknownResults.push(name);
  }

  const commitShaDrift =
    typeof predicted.commitSha === 'string' &&
    typeof actual.commitSha === 'string' &&
    predicted.commitSha !== actual.commitSha;

  const hasDrift =
    predictedNotExecuted.length > 0 ||
    executedNotPredicted.length > 0 ||
    abortedAfterStart.length > 0 ||
    unstableResults.length > 0 ||
    unknownResults.length > 0 ||
    commitShaDrift;

  const verdict = hasDrift ? 'drift' : 'match';

  // Investigation pointers: one per aborted/failed pipeline, with its URL
  // if available. These are the things that "every disconnect elevates to
  // an investigation."
  const investigationPointers = [];
  for (const name of abortedAfterStart) {
    const info = actualResults[name];
    const url = pipelineUrl(name, info);
    const piece = url ? `${name} ${info.result} — ${url}` : `${name} ${info.result}`;
    investigationPointers.push(piece);
  }
  for (const name of predictedNotExecuted) {
    investigationPointers.push(`${name} predicted but did NOT run`);
  }
  for (const name of executedNotPredicted) {
    investigationPointers.push(`${name} ran but was NOT predicted`);
  }
  if (commitShaDrift) {
    investigationPointers.push(
      `commit sha drift: predicted ${predicted.commitSha?.slice(0, 8)} actual ${actual.commitSha?.slice(0, 8)}`,
    );
  }

  // Human-readable summary.
  const parts = [];
  if (commitShaDrift) parts.push('commit sha drift');
  if (predictedNotExecuted.length) parts.push(`${predictedNotExecuted.length} predicted-not-executed`);
  if (executedNotPredicted.length) parts.push(`${executedNotPredicted.length} executed-not-predicted`);
  if (abortedAfterStart.length) parts.push(`${abortedAfterStart.length} aborted`);
  if (unstableResults.length) parts.push(`${unstableResults.length} unstable`);
  if (unknownResults.length) parts.push(`${unknownResults.length} unknown-result`);
  const summary = parts.length === 0
    ? `match: ${actualSet.size} pipelines ran clean`
    : `drift: ${parts.join(', ')}`;

  return {
    schemaVersion: '1',
    reconciledAt: new Date().toISOString(),
    commitSha: actual.commitSha || predicted.commitSha || null,
    verdict,
    predictedPipelines: [...predictedSet],
    actualPipelines: [...actualSet],
    disconnects: {
      predictedNotExecuted: [...new Set(predictedNotExecuted)].sort(),
      executedNotPredicted: [...executedNotPredicted].sort(),
      abortedAfterStart: [...abortedAfterStart].sort(),
      unstableResults: [...unstableResults].sort(),
      commitShaDrift,
      unknownResults: [...unknownResults].sort(),
    },
    summary,
    investigationPointers,
  };
}

// ── CLI mode ───────────────────────────────────────────────────────

const isMain = import.meta.url === `file://${process.argv[1]}` ||
               import.meta.url === `file://${resolve(process.argv[1])}`;

if (isMain) {
  // Default to canonical filenames from build-artifacts.json so the CLI
  // works as `node reconcile-build-graph.mjs` from a workspace where
  // those artifacts already exist. Args still override for explicit use.
  const [predictedArg, actualArg, outArg] = process.argv.slice(2);
  const predictedPath = predictedArg || ORCHESTRATOR.predictedBuildGraph;
  const actualPath = actualArg || ORCHESTRATOR.actualBuildGraph;
  const outPath = outArg || ORCHESTRATOR.buildGraphReconciliation;
  if (!predictedPath || !actualPath || !outPath) {
    console.error('usage: node reconcile-build-graph.mjs [<predicted.json> <actual.json> <out.json>]');
    process.exit(2);
  }

  const predicted = JSON.parse(readFileSync(predictedPath, 'utf8'));
  const actual = JSON.parse(readFileSync(actualPath, 'utf8'));
  const result = reconcile({ predicted, actual });

  writeFileSync(outPath, JSON.stringify(result, null, 2) + '\n');

  // Print a one-screen summary for the Jenkins console.
  console.log('');
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
  console.log(`📊 BUILD GRAPH RECONCILIATION — verdict: ${result.verdict.toUpperCase()}`);
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
  console.log(`  ${result.summary}`);
  if (result.investigationPointers.length > 0) {
    console.log('');
    console.log('  Investigation pointers:');
    for (const p of result.investigationPointers) {
      console.log(`    • ${p}`);
    }
  }
  console.log('');

  // Exit code: 0 on match, 1 on drift. The orchestrator can use this to
  // mark the build UNSTABLE without aborting.
  process.exit(result.verdict === 'match' ? 0 : 1);
}
