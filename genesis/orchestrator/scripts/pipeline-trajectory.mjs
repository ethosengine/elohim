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
 *   JENKINS_URL=... JENKINS_TOKEN=... node pipeline-trajectory.mjs [opts]
 *
 *   --builds <N>          how many orchestrator builds to include (default 10)
 *   --branch <name>       orchestrator branch (default 'dev')
 *   --pipelines <list>    comma-separated downstream pipeline jobs to track
 *                          (default: storybook,holochain,edge,elohim,genesis,sophia)
 *   --json                emit structured JSON instead of a table
 *   --since-baseline      additionally compute baseline-lag in builds + commits
 *                          (extra Jenkins reads — slower; default off)
 *
 * Read-only. No side effects.
 */

import process from 'node:process';

const JENKINS_URL = process.env.JENKINS_URL;
const JENKINS_TOKEN = process.env.JENKINS_TOKEN;
if (!JENKINS_URL || !JENKINS_TOKEN) {
  console.error('FATAL: JENKINS_URL and JENKINS_TOKEN must be set');
  process.exit(2);
}

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
const PIPELINES = argVal(
  '--pipelines',
  'elohim-storybook,elohim-holochain,elohim-edge,elohim,elohim-genesis,elohim-sophia',
).split(',').filter(Boolean);
const EMIT_JSON = argFlag('--json');
const COMPUTE_BASELINE_LAG = argFlag('--since-baseline');

// ─── Jenkins fetchers ─────────────────────────────────────────────────────
async function jen(path) {
  const res = await fetch(`${JENKINS_URL}${path}`, {
    headers: { 'Jenkins-Token': JENKINS_TOKEN },
  });
  if (!res.ok) throw new Error(`${res.status} ${res.statusText} on ${path}`);
  return res.json();
}
async function jenArtifact(path) {
  const res = await fetch(`${JENKINS_URL}${path}`, {
    headers: { 'Jenkins-Token': JENKINS_TOKEN },
  });
  if (!res.ok) return null;
  try {
    return await res.json();
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
  } catch {
    return [];
  }
}

async function getOrchestratorArtifact(branch, buildNum, name) {
  return jenArtifact(
    `/job/elohim-orchestrator/job/${branch}/${buildNum}/artifact/${name}`,
  );
}

// ─── derivation ───────────────────────────────────────────────────────────
function pipelineBuildForOrchestrator(orchestratorBuild, pipelineBuilds, orchestratorJob = 'elohim-orchestrator') {
  // A pipeline build "belongs to" an orchestrator build if its upstream cause
  // names that orchestrator's build number.
  for (const pb of pipelineBuilds) {
    const causes = (pb.actions || []).flatMap(a => a.causes || []);
    for (const c of causes) {
      if (
        c.upstreamProject &&
        c.upstreamProject.includes(orchestratorJob) &&
        c.upstreamBuild === orchestratorBuild.number
      ) {
        return pb;
      }
    }
  }
  return null;
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

  // Compose per-build rows.
  const rows = orchBuilds.map(ob => {
    const downstream = {};
    for (const job of PIPELINES) {
      downstream[job] = pipelineBuildForOrchestrator(ob, pipelineBuildLists[job] || []);
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
      const success = stream.filter(s => s.result === 'SUCCESS' || s.result === 'UNSTABLE').length;
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
    const failures = completed.filter(s => s.result === 'FAILURE').length;
    if (completed.length >= 3 && failures >= Math.ceil(completed.length / 2)) {
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
    const aborted = t.stream.filter(s => s.result === 'ABORTED').length;
    if (aborted >= 2) {
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
  const orchSuccess = orchCompleted.filter(
    r => r.result === 'SUCCESS' || r.result === 'UNSTABLE',
  ).length;
  if (orchCompleted.length >= 3 && orchSuccess === 0) {
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
    if (drift >= 5) {
      patterns.push({
        kind: 'baseline-drift',
        rate: `${drift} builds with different __global__ in last ${orchBuilds.length}`,
        note: 'baseline has shifted significantly — verify lastCompleted() is advancing as expected',
      });
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

  if (patterns.length > 0) {
    console.log('');
    console.log('## patterns detected');
    for (const p of patterns) {
      console.log(`  ⚠ ${p.kind}${p.pipeline ? ` (${p.pipeline})` : ''}: ${p.rate} — ${p.note}`);
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
