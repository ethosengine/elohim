#!/usr/bin/env node
/**
 * pipeline-trajectory.mjs — deterministic roll-up of recent orchestrator
 * runs and their downstream pipeline outcomes.
 *
 * Purpose: surface CI patterns over time (cascade-storm, persistent failure,
 * supersede waste, baseline drift, stale-dispatch) without having to
 * re-discover the same facts via ad-hoc curl every shift.
 *
 * Usage:
 *   JENKINS_URL=... node pipeline-trajectory.mjs [opts]
 *
 *   --builds <N>          how many orchestrator builds to include (default 10)
 *   --branch <name>       orchestrator branch (default 'dev')
 *   --pipelines <list>    comma-separated downstream pipeline jobs to track.
 *                          Defaults to all non-manualOnly pipelines from
 *                          orchestrator-strategy.mjs PIPELINES (single source
 *                          of truth — keeps this tool from drifting from the
 *                          orchestrator's actual dispatch set).
 *   --json                emit structured JSON instead of a table
 *   --since-baseline      additionally compute baseline-lag in builds + commits
 *                          (extra Jenkins reads — slower; default off)
 *   --with-stages         additionally fetch per-stage results via wfapi —
 *                          surfaces persistent-stage-failure (which stage is
 *                          the gating leg) and stage-time-dominant (which
 *                          stage is consuming the build budget). Extra
 *                          fetches; default off.
 *
 * Read-only. No side effects. Anonymous-read against the orchestrator's
 * OIDC-fronted Jenkins (sending auth headers triggers an OIDC redirect).
 */

import process from 'node:process';
import { nonManualPipelines } from '../orchestrator-strategy.mjs';
import { isSuccess, isFailure, isWasted } from '../pipeline-results.mjs';

// Tracks pipelines whose Jenkins job was unreachable (404 / network error).
// Module-level so it accumulates across calls; fine in single-run mode and
// harmless in --watch mode (drift is stable across iterations).
const unreachablePipelines = new Set();

const JENKINS_URL = process.env.JENKINS_URL;
if (!JENKINS_URL) {
  console.error('FATAL: JENKINS_URL must be set');
  process.exit(2);
}
// This Jenkins is OIDC-fronted with anonymous read enabled for the orchestrator
// + downstream jobs; sending an auth header triggers a redirect to the OIDC
// provider and breaks the read. See memory: jenkins_mcp_anonymous_mode.

// ─── arg parsing ──────────────────────────────────────────────────────────
const args = process.argv.slice(2);
function argVal(name, def) {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : def;
}
function argFlag(name) {
  return args.includes(name);
}
const N = Number(argVal('--builds', '10'));
const BRANCH = argVal('--branch', 'dev');
const DEFAULT_PIPELINES = nonManualPipelines().join(',');
const PIPELINES = argVal('--pipelines', DEFAULT_PIPELINES).split(',').filter(Boolean);
const EMIT_JSON = argFlag('--json');
const COMPUTE_BASELINE_LAG = argFlag('--since-baseline');
const WITH_STAGES = argFlag('--with-stages');

// ─── pattern thresholds ───────────────────────────────────────────────────
// Tuned for a 10-build window. Adjust if --builds is wildly different.
const PATTERN_THRESHOLDS = {
  // persistent-failure: at least this fraction of completed builds failed
  persistentFailureFraction: 0.5,
  // supersede-waste: require BOTH an absolute count AND a fraction of the window
  supersedeWasteAbsolute: 3,
  supersedeWasteFraction: 0.3,
  // orchestrator-failure-streak: zero successes in window when ≥ this many built
  orchestratorStreakMinCompleted: 3,
  // baseline-drift: distinct baselines in window
  baselineDriftMin: 5,
};

// ─── Jenkins fetchers ─────────────────────────────────────────────────────
const RETRY_DELAYS_MS = [0, 1500, 4000]; // immediate, then backoff
async function jenRequest(path, { allow404 = false } = {}) {
  let lastErr;
  for (const delay of RETRY_DELAYS_MS) {
    if (delay) await new Promise(r => setTimeout(r, delay));
    let res;
    try {
      res = await fetch(`${JENKINS_URL}${path}`);
    } catch (e) {
      // Network error — retryable.
      lastErr = e;
      continue;
    }
    if (res.ok) return await res.json();
    if (allow404 && res.status === 404) return null;
    // Retry on transient server errors only; 4xx (other than allowed 404) is terminal.
    if (res.status >= 500 && res.status < 600) {
      lastErr = new Error(`${res.status} ${res.statusText} on ${path}`);
      continue;
    }
    throw new Error(`${res.status} ${res.statusText} on ${path}`);
  }
  throw lastErr;
}
async function jen(path) {
  return jenRequest(path);
}
async function jenArtifact(path) {
  try {
    return await jenRequest(path, { allow404: true });
  } catch {
    return null;
  }
}

// ─── data collection ──────────────────────────────────────────────────────
async function getOrchestratorBuilds(branch, n) {
  const data = await jen(
    `/job/elohim-orchestrator/job/${branch}/api/json` +
      `?tree=builds[number,result,timestamp,duration,actions[causes[shortDescription]]]{0,${n}}`,
  );
  return (data.builds || []).slice(0, n);
}

async function getPipelineBuilds(jobName, branch, n) {
  try {
    const data = await jen(
      `/job/${jobName}/job/${branch}/api/json` +
        `?tree=builds[number,result,timestamp,duration,actions[causes[upstreamBuild,upstreamProject,shortDescription]]]{0,${n * 3}}`,
    );
    return data.builds || [];
  } catch (e) {
    // Visible warning, not silent empty — empty rows otherwise look like
    // "pipeline has nothing recent" instead of "we couldn't see it."
    console.error(`WARN: ${jobName}/${branch} unreachable (${e.message}) — row will render as not-built`);
    unreachablePipelines.add(jobName);
    return [];
  }
}

async function getOrchestratorArtifact(branch, buildNum, name) {
  return jenArtifact(
    `/job/elohim-orchestrator/job/${branch}/${buildNum}/artifact/${name}`,
  );
}

/**
 * Returns the stages of a build, each with { name, result, duration }.
 * Uses the wfapi (Pipeline Stage View) plugin which exposes:
 *   /job/<job>/job/<branch>/<build>/wfapi/describe
 * Returns [] if wfapi unavailable or the build has no stages.
 *
 * wfapi status vocabulary differs slightly from build result vocabulary:
 *   - FAILED (past-tense)  → normalized to FAILURE for pipeline-results.mjs
 *   - NOT_EXECUTED         → normalized to NOT_BUILT
 *   - IN_PROGRESS          → normalized to null (matches build.result==null convention)
 *   - SUCCESS, UNSTABLE, ABORTED → passed through
 */
async function getBuildStages(jobName, branch, buildNumber) {
  try {
    const data = await jen(
      `/job/${jobName}/job/${branch}/${buildNumber}/wfapi/describe`,
    );
    return (data.stages || []).map(s => ({
      name: s.name,
      result: s.status === 'FAILED' ? 'FAILURE'
            : s.status === 'NOT_EXECUTED' ? 'NOT_BUILT'
            : s.status === 'IN_PROGRESS' ? null
            : s.status,
      duration: s.durationMillis,
    }));
  } catch (e) {
    console.error(`WARN: stages for ${jobName}#${buildNumber} unreachable (${e.message})`);
    return [];
  }
}

// ─── derivation ───────────────────────────────────────────────────────────
function buildUpstreamIndex(pipelineBuilds, orchestratorJob = 'elohim-orchestrator') {
  // Build a Map from orchestrator build number -> downstream pipeline build.
  // Single pre-pass per pipeline; subsequent lookups are O(1) instead of the
  // O(N×M) scan the original pipelineBuildForOrchestrator performed.
  const index = new Map();
  for (const pb of pipelineBuilds) {
    const causes = (pb.actions || []).flatMap(a => a.causes || []);
    for (const c of causes) {
      if (c.upstreamProject && c.upstreamProject.includes(orchestratorJob)) {
        index.set(c.upstreamBuild, pb);
      }
    }
  }
  return index;
}

function resultGlyph(result) {
  if (result === 'SUCCESS') return '✓';
  if (result === 'UNSTABLE') return '⚠';
  if (result === 'FAILURE') return '✗';
  if (result === 'ABORTED') return '×';
  if (result === 'NOT_BUILT') return '·';
  if (result == null) return '↻';
  return '?';
}
function shortPipelineName(name) {
  return name.replace(/^elohim-/, '');
}
function durationMin(ms) {
  if (ms == null || ms === 0) return '-';
  return Math.round(ms / 60000) + 'm';
}
function shortSha(sha) {
  if (!sha) return '-';
  return String(sha).slice(0, 8);
}
function isoDay(ms) {
  return new Date(ms).toISOString().slice(0, 10);
}
function isoMin(ms) {
  return new Date(ms).toISOString().slice(0, 16).replace('T', ' ');
}

// ─── main ─────────────────────────────────────────────────────────────────
async function main() {
  const orchBuilds = await getOrchestratorBuilds(BRANCH, N);
  if (orchBuilds.length === 0) {
    console.error('No orchestrator builds found.');
    process.exit(1);
  }

  // Fetch each tracked pipeline's recent builds in parallel.
  const pipelineBuildLists = Object.fromEntries(
    await Promise.all(
      PIPELINES.map(async jobName => [jobName, await getPipelineBuilds(jobName, BRANCH, N)]),
    ),
  );

  // For each orchestrator build, find its baseline (from pipeline-baselines.json).
  // Optional because it adds N artifact fetches.
  const baselines = {};
  if (COMPUTE_BASELINE_LAG) {
    await Promise.all(
      orchBuilds.map(async b => {
        const pb = await getOrchestratorArtifact(BRANCH, b.number, 'pipeline-baselines.json');
        baselines[b.number] = pb;
      }),
    );
  }

  // Build an index per pipeline once, then O(1) lookup per orchestrator build.
  const pipelineIndexes = Object.fromEntries(
    PIPELINES.map(job => [job, buildUpstreamIndex(pipelineBuildLists[job] || [])]),
  );

  // Compose per-build rows.
  const rows = orchBuilds.map(ob => {
    const downstream = {};
    for (const job of PIPELINES) {
      downstream[job] = pipelineIndexes[job].get(ob.number) || null;
    }
    return {
      number: ob.number,
      result: ob.result,
      duration: ob.duration,
      timestamp: ob.timestamp,
      baseline: COMPUTE_BASELINE_LAG ? baselines[ob.number] : null,
      downstream,
    };
  });

  // Conditionally fetch per-build stage data in parallel (--with-stages).
  // Builds that didn't complete (pb.result == null) are skipped — still in progress.
  if (WITH_STAGES) {
    await Promise.all(
      rows.flatMap(r =>
        PIPELINES.flatMap(job => {
          const pb = r.downstream[job];
          if (!pb || pb.result == null) return [];
          return [
            getBuildStages(job, BRANCH, pb.number).then(stages => {
              pb.stages = stages;
            }),
          ];
        }),
      ),
    );
  }

  // ─── trajectories (per-pipeline streams) ────────────────────────────────
  const trajectories = Object.fromEntries(
    PIPELINES.map(job => {
      const stream = rows.map(r => {
        const pb = r.downstream[job];
        return {
          orchestrator: r.number,
          pipelineBuild: pb?.number ?? null,
          result: pb?.result ?? null,
          duration: pb?.duration ?? null,
        };
      });
      const completed = stream.filter(s => s.result != null);
      const success = stream.filter(s => isSuccess(s.result)).length;
      return [
        job,
        {
          stream,
          successRate:
            completed.length === 0 ? null : `${success}/${completed.length}`,
        },
      ];
    }),
  );

  // ─── patterns ───────────────────────────────────────────────────────────
  const patterns = [];

  // Persistent failure: same pipeline FAILURE in M of last N completed.
  for (const [job, t] of Object.entries(trajectories)) {
    const completed = t.stream.filter(s => s.result != null);
    const failures = completed.filter(s => isFailure(s.result)).length;
    if (completed.length >= 3 && failures >= Math.ceil(completed.length * PATTERN_THRESHOLDS.persistentFailureFraction)) {
      patterns.push({
        kind: 'persistent-failure',
        pipeline: job,
        rate: `${failures}/${completed.length}`,
        note: 'failed in more than half of recent completed builds',
      });
    }
  }

  // Supersede waste: ABORTED runs adjacent to short-duration runs.
  for (const [job, t] of Object.entries(trajectories)) {
    const aborted = t.stream.filter(s => isWasted(s.result)).length;
    if (
      aborted >= PATTERN_THRESHOLDS.supersedeWasteAbsolute &&
      aborted / t.stream.length >= PATTERN_THRESHOLDS.supersedeWasteFraction
    ) {
      patterns.push({
        kind: 'supersede-waste',
        pipeline: job,
        rate: `${aborted} aborted in last ${t.stream.length}`,
        note: 'orchestrator runs are superseding in-flight pipeline builds',
      });
    }
  }

  // Orchestrator failure streak.
  const orchCompleted = rows.filter(r => r.result != null && r.result !== 'NOT_BUILT');
  const orchSuccess = orchCompleted.filter(r => isSuccess(r.result)).length;
  if (orchCompleted.length >= PATTERN_THRESHOLDS.orchestratorStreakMinCompleted && orchSuccess === 0) {
    patterns.push({
      kind: 'orchestrator-failure-streak',
      rate: `0/${orchCompleted.length}`,
      note: 'no orchestrator UNSTABLE-or-better in window — cascade-storm risk',
    });
  }

  // Baseline drift (only when --since-baseline computed).
  if (COMPUTE_BASELINE_LAG) {
    const latestBaselineSha = baselines[orchBuilds[0]?.number]?.__global__;
    let drift = 0;
    for (const b of orchBuilds) {
      const sha = baselines[b.number]?.__global__;
      if (sha && sha !== latestBaselineSha) drift++;
    }
    if (drift >= PATTERN_THRESHOLDS.baselineDriftMin) {
      patterns.push({
        kind: 'baseline-drift',
        rate: `${drift} builds with different __global__ in last ${orchBuilds.length}`,
        note: 'baseline has shifted significantly — verify lastCompleted() is advancing as expected',
      });
    }
  }

  // Registry vs cluster drift — surface pipelines whose Jenkins job was
  // unreachable (persistent 404 / network error). High-signal because
  // transient 5xx errors are retried; only jobs that don't exist on this
  // branch reach the catch path. Run scripts/registry-cluster-drift.mjs for
  // the full bidirectional report (registry-only + cluster-only).
  for (const job of unreachablePipelines) {
    patterns.push({
      kind: 'registry-cluster-drift',
      pipeline: job,
      rate: 'no Jenkins builds visible',
      note: 'pipeline declared in registry but Jenkins job unreachable — run scripts/registry-cluster-drift.mjs',
    });
  }

  if (WITH_STAGES) {
    // Persistent-failure at stage granularity — far more actionable than
    // pipeline-level because it names the gating leg.
    for (const job of PIPELINES) {
      const stagesByName = new Map();
      for (const r of rows) {
        const pb = r.downstream[job];
        if (!pb?.stages) continue;
        for (const s of pb.stages) {
          if (!stagesByName.has(s.name)) stagesByName.set(s.name, []);
          stagesByName.get(s.name).push(s);
        }
      }
      for (const [name, history] of stagesByName) {
        const completed = history.filter(s => s.result != null);
        const failures = completed.filter(s => isFailure(s.result)).length;
        if (
          completed.length >= 3 &&
          failures / completed.length >= PATTERN_THRESHOLDS.persistentFailureFraction
        ) {
          patterns.push({
            kind: 'persistent-stage-failure',
            pipeline: job,
            stage: name,
            rate: `${failures}/${completed.length}`,
            note: `stage '${name}' in ${shortPipelineName(job)} failed in more than half of recent builds — gating leg`,
          });
        }
      }
    }

    // Stage-duration outliers — surface stages that take >30% of total build time.
    // Track which (pipeline, stageName) tuples we've already emitted so a
    // dominant stage shows once, not N times.
    const dominantSeen = new Set();
    for (const job of PIPELINES) {
      for (const r of rows) {
        const pb = r.downstream[job];
        if (!pb?.stages || !pb.duration) continue;
        for (const s of pb.stages) {
          if (s.duration && s.duration / pb.duration >= 0.3) {
            const key = `${job}::${s.name}`;
            if (!dominantSeen.has(key)) {
              dominantSeen.add(key);
              patterns.push({
                kind: 'stage-time-dominant',
                pipeline: job,
                stage: s.name,
                rate: `${Math.round(100 * s.duration / pb.duration)}% of build`,
                note: `stage '${s.name}' dominates ${shortPipelineName(job)} wall-time — sharding/cache target`,
              });
            }
          }
        }
      }
    }
  }

  // ─── output ─────────────────────────────────────────────────────────────
  if (EMIT_JSON) {
    console.log(JSON.stringify({ rows, trajectories, patterns }, null, 2));
    return;
  }

  // Table view.
  const colW = {
    num: 5,
    result: 11,
    dur: 6,
    when: 17,
    base: 9,
  };
  const headerCols = ['#', 'result', 'dur', 'when'];
  if (COMPUTE_BASELINE_LAG) headerCols.push('baseline');
  for (const p of PIPELINES) headerCols.push(shortPipelineName(p));

  const widths = [colW.num, colW.result, colW.dur, colW.when];
  if (COMPUTE_BASELINE_LAG) widths.push(colW.base);
  for (const _ of PIPELINES) widths.push(10);

  function pad(s, w, align = 'left') {
    s = String(s ?? '');
    if (s.length >= w) return s.slice(0, w);
    const padding = ' '.repeat(w - s.length);
    return align === 'right' ? padding + s : s + padding;
  }

  console.log(`# orchestrator/${BRANCH} — last ${orchBuilds.length} builds`);
  console.log('');
  console.log(headerCols.map((h, i) => pad(h, widths[i])).join('  '));
  console.log(widths.map(w => '-'.repeat(w)).join('  '));

  for (const r of rows) {
    const cells = [
      pad('#' + r.number, widths[0]),
      pad(r.result || 'RUNNING', widths[1]),
      pad(durationMin(r.duration), widths[2], 'right'),
      pad(isoMin(r.timestamp), widths[3]),
    ];
    let ci = 4;
    if (COMPUTE_BASELINE_LAG) {
      cells.push(pad(shortSha(r.baseline?.__global__), widths[ci++]));
    }
    for (const p of PIPELINES) {
      const pb = r.downstream[p];
      if (!pb) {
        cells.push(pad('  .', widths[ci++]));
      } else {
        const glyph = resultGlyph(pb.result);
        const dur = durationMin(pb.duration);
        cells.push(pad(`${glyph} #${pb.number} ${dur}`.padEnd(widths[ci]).slice(0, widths[ci]), widths[ci++]));
      }
    }
    console.log(cells.join('  '));
  }

  console.log('');
  console.log('## per-pipeline trajectory (most recent first)');
  for (const [job, t] of Object.entries(trajectories)) {
    const stream = t.stream
      .map(s => resultGlyph(s.result))
      .join(' ');
    const rate = t.successRate ?? '0/0';
    console.log(`  ${pad(shortPipelineName(job), 12)} ${pad(stream, 24)}  rate=${rate}`);
  }

  if (WITH_STAGES) {
    console.log('');
    console.log('## per-stage trajectory (within each pipeline, most recent first)');
    for (const job of PIPELINES) {
      // Collect stages across the row stream, keyed by stage name.
      const stagesByName = new Map();
      for (const r of rows) {
        const pb = r.downstream[job];
        if (!pb?.stages) continue;
        for (const s of pb.stages) {
          if (!stagesByName.has(s.name)) stagesByName.set(s.name, []);
          stagesByName.get(s.name).push(s);
        }
      }
      if (stagesByName.size === 0) continue;
      console.log(`  ── ${shortPipelineName(job)} ──`);
      for (const [name, history] of stagesByName) {
        const stream = history.map(s => resultGlyph(s.result)).join(' ');
        const completed = history.filter(s => s.result != null);
        const success = completed.filter(s => isSuccess(s.result)).length;
        const rate = completed.length === 0 ? '0/0' : `${success}/${completed.length}`;
        const maxDur = Math.max(...history.map(s => s.duration ?? 0));
        console.log(`    ${pad(name, 32)} ${pad(stream, 24)}  rate=${rate}  max=${durationMin(maxDur)}`);
      }
    }
  }

  if (patterns.length > 0) {
    console.log('');
    console.log('## patterns detected');
    for (const p of patterns) {
      const ctx = p.pipeline
        ? p.stage
          ? ` (${shortPipelineName(p.pipeline)} / ${p.stage})`
          : ` (${p.pipeline})`
        : '';
      console.log(`  ⚠ ${p.kind}${ctx}: ${p.rate} — ${p.note}`);
    }
  } else {
    console.log('');
    console.log('## patterns detected: none');
  }

  console.log('');
  console.log('# legend: ✓ SUCCESS  ⚠ UNSTABLE  ✗ FAILURE  × ABORTED  · NOT_BUILT  ↻ RUNNING');
}

main().catch(e => {
  console.error('ERROR:', e.message);
  process.exit(1);
});
