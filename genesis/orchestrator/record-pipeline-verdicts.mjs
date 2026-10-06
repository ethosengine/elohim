#!/usr/bin/env node
/**
 * record-pipeline-verdicts.mjs — the evidence half of the already-built
 * filter's `verdict-recorded` provenance (timer-dispatch.mjs recordedVerdict).
 *
 * A `longRunning` (dispatch-only) pipeline's baseline only records that the
 * orchestrator DISPATCHED a build of that sha; the run never waited for its
 * verdict. This reads, for each such pipeline, its Jenkins build of the
 * baseline sha and writes the finished result:
 *
 *   pipeline-verdicts.json = { "<pipeline>": { "sha", "result", "build" } }
 *
 * Absence is the safe answer: a build still running, no build whose checked-out
 * revision is the baseline sha, or any fetch/parse error writes NO row, and the
 * dispatcher keeps today's behaviour (KEPT). The script always exits 0.
 *
 * Usage: node record-pipeline-verdicts.mjs <state.json> [out.json]
 *   state.json carries `baselines` ({pipeline: sha, __global__: sha}) — the
 *   same state file the already-built filter writes for timer-dispatch.mjs.
 *
 * Read path: the anonymous Jenkins REST API (as delivery-series.mjs and
 * scripts/count-pipeline-failures.sh read it), job path
 * `<jenkinsJob ?: pipeline>/<jenkinsBranch ?: BRANCH_NAME>` (as the
 * orchestrator dispatches it), revision = BuildData's lastBuiltRevision.SHA1.
 */

import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const SHA = /^[0-9a-f]{40}$/;
const TREE = 'builds[number,result,building,actions[lastBuiltRevision[SHA1]]]{0,15}';

/** The finished result of the newest build whose revision is `sha`, or null. */
export function verdictForSha(builds, sha) {
  if (!Array.isArray(builds)) return null;
  for (const b of builds) {
    const revs = (Array.isArray(b?.actions) ? b.actions : [])
      .map(a => a?.lastBuiltRevision?.SHA1)
      .filter(Boolean);
    if (!revs.includes(sha)) continue;
    // Newest match decides: a re-run still in flight is no evidence yet.
    if (b.building || typeof b.result !== 'string' || !b.result) return null;
    return { result: b.result, build: b.number };
  }
  return null;
}

/** Pipelines whose baseline only records a dispatch, with their job path. */
export function dispatchOnlyTargets(manifests, baselines, branch) {
  const out = [];
  for (const { content } of manifests) {
    if (!content || content.longRunning !== true || !content.pipeline) continue;
    const sha = baselines?.[content.pipeline];
    if (typeof sha !== 'string' || !SHA.test(sha)) continue;
    out.push({
      name: content.pipeline,
      sha,
      job: `${content.jenkinsJob || content.pipeline}/${content.jenkinsBranch || branch}`,
    });
  }
  return out;
}

export function buildsUrl(jenkinsUrl, job) {
  const base = jenkinsUrl.endsWith('/') ? jenkinsUrl : `${jenkinsUrl}/`;
  const path = job.split('/').map(p => `job/${encodeURIComponent(p)}`).join('/');
  return `${base}${path}/api/json?tree=${encodeURIComponent(TREE)}`;
}

/**
 * @param {{targets: Array<{name,sha,job}>, fetchJson: (url) => Promise<any>,
 *          jenkinsUrl: string, log: (line) => void}} args
 * @returns {Promise<object>} the verdict rows (possibly empty)
 */
export async function recordVerdicts({ targets, fetchJson, jenkinsUrl, log }) {
  const rows = {};
  for (const t of targets) {
    try {
      const body = await fetchJson(buildsUrl(jenkinsUrl, t.job));
      const v = verdictForSha(body?.builds, t.sha);
      if (v) {
        rows[t.name] = { sha: t.sha, result: v.result, build: v.build };
        log(`[verdicts] ${t.name}: build #${v.build} of ${t.sha.slice(0, 8)} recorded ${v.result}`);
      } else {
        log(`[verdicts] ${t.name}: no finished build of ${t.sha.slice(0, 8)} — no row (stays dispatch-only)`);
      }
    } catch (err) {
      log(`[verdicts] ${t.name}: read failed (${err?.message ?? err}) — no row (stays dispatch-only)`);
    }
  }
  return rows;
}

async function defaultFetchJson(url) {
  const res = await fetch(url, { signal: AbortSignal.timeout(15_000) });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

async function main(argv) {
  const [statePath, outPath = 'pipeline-verdicts.json'] = argv;
  const out = resolve(process.cwd(), outPath);
  let rows = {};
  try {
    const state = JSON.parse(readFileSync(statePath, 'utf8'));
    const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
    const { loadManifests } = await import('./manifest-utils.mjs');
    const targets = dispatchOnlyTargets(loadManifests(root), state.baselines ?? {}, process.env.BRANCH_NAME || 'dev');
    rows = await recordVerdicts({
      targets,
      fetchJson: defaultFetchJson,
      jenkinsUrl: process.env.JENKINS_URL || 'https://jenkins.ethosengine.com/',
      log: line => console.log(line),
    });
  } catch (err) {
    console.log(`[verdicts] skipped (${err?.message ?? err}) — no rows recorded`);
    rows = {};
  }
  try {
    writeFileSync(out, `${JSON.stringify(rows, null, 2)}\n`);
  } catch (err) {
    console.log(`[verdicts] could not write ${outPath} (${err?.message ?? err})`);
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main(process.argv.slice(2));
  process.exit(0);
}
