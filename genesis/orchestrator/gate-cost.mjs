// Gate cost — fold recorded full runs into an expectation and a pure decision
// about deferring a slow, locally reliable gate to feature-branch CI.

import { readFileSync } from 'fs';
import { GATE_CYCLE_MEASURE, minutes } from './gate-cycle.mjs';

// genesis/orchestrator/Jenkinsfile:1894 — Groovy ==~ matches the whole branch name.
export const CI_BRANCH_PATTERN = /^(?:dev|dev-.+|feat-.+|claude\/.+)$/;

/** Read the flows sidecar; unrelated records are ignored, malformed lines counted. */
export function readCycleRows(flowsPath, readFile = readFileSync) {
  const rows = [];
  let skipped = 0;
  for (const line of readFile(flowsPath, 'utf8').split('\n')) {
    if (!line.trim()) continue;
    try {
      const { record } = JSON.parse(line);
      const tags = record?.classifiedAs;
      if (!Array.isArray(tags)) throw new Error('missing classifiedAs');
      if (!tags.includes(`measure:${GATE_CYCLE_MEASURE}`)) continue;
      const slot = prefix => tags.find(tag => typeof tag === 'string' && tag.startsWith(prefix))?.slice(prefix.length);
      const rawValue = slot('value:');
      const value = rawValue?.trim() ? Number(rawValue) : NaN;
      const kind = slot('env:run-kind=');
      const exit = slot('env:exit=');
      const project = slot('env:project=');
      // Native flow notes put the bare subject path immediately after run:observation.
      const subject = tags[1];
      const occurredAt = record.occurredAt;
      if (!Number.isFinite(value) || value < 0 || !kind || !['ok', 'fail'].includes(exit) ||
          project === '' || typeof subject !== 'string' || !subject ||
          typeof occurredAt !== 'string' || !Number.isFinite(Date.parse(occurredAt))) {
        throw new Error('malformed gate cycle');
      }
      rows.push({ value, kind, exit, project, subject, occurredAt });
    } catch { skipped += 1; }
  }
  return { rows, skipped };
}

/** Legacy subjects count only when the caller has established they are unshared. */
export function costOf(rows, { project, subjectFallback }) {
  const counted = rows.filter(row =>
    (row.project === project.name ||
      (row.project === undefined && subjectFallback != null && row.subject === subjectFallback)) &&
    ['full-gate', 'pre-push'].includes(row.kind))
    .sort((a, b) => Date.parse(a.occurredAt) - Date.parse(b.occurredAt));
  const successful = counted.filter(row => row.exit === 'ok').slice(-10)
    .map(row => row.value).sort((a, b) => a - b);
  const recent = counted.slice(-20);
  const runs = successful.length;
  return {
    runs,
    mean: runs ? successful.reduce((sum, value) => sum + value, 0) / runs : null,
    p90: runs ? successful[Math.ceil(runs * 0.9) - 1] : null,
    max: runs ? successful[runs - 1] : null,
    fails: recent.filter(row => row.exit === 'fail').length,
    window: recent.length,
  };
}

export function formatExpectation(name, cost) {
  if (!cost.runs) return `[gate] ${name}: no recorded runs yet`;
  return `[gate] ${name}: expect ~${minutes(cost.mean)} (p90 ${minutes(cost.p90)}, max ${minutes(cost.max)}; ${cost.runs} runs) · failed locally ${cost.fails} of last ${cost.window}`;
}

/** Every refusal keeps the gate local; the first unsatisfied condition explains why. */
export function deferDecision({ cost, targets, branches, ciBranchPattern, env, softSeconds = 600, minRuns = 10 }) {
  if (env.GATE_DEFER === 'off') return { defer: false, why: 'GATE_DEFER=off' };
  if (env.FORCE_HEAVY_GATES === '1') return { defer: false, why: 'FORCE_HEAVY_GATES=1' };
  const protectedTarget = targets.find(target => ['refs/heads/dev', 'refs/heads/main'].includes(target));
  if (protectedTarget) return { defer: false, why: `${protectedTarget} requires local gates` };
  // Clone for each match so global/sticky expressions cannot mutate caller state.
  const nonCiBranch = branches.find(branch => !new RegExp(ciBranchPattern).test(branch));
  if (!branches.length || nonCiBranch !== undefined) {
    return { defer: false, why: nonCiBranch === undefined ? 'no CI branch supplied' : `${nonCiBranch} does not match the CI branch pattern` };
  }
  if (cost.runs < minRuns) return { defer: false, why: `${cost.runs} successful runs; need ${minRuns}` };
  if (cost.p90 === null || !(cost.p90 >= softSeconds)) {
    return { defer: false, why: `p90 ${cost.p90} s does not reach the ${softSeconds} s soft ceiling` };
  }
  if (cost.fails !== 0) return { defer: false, why: `failed locally ${cost.fails} of last ${cost.window}` };
  return { defer: true, why: `feature-branch CI covers this push; p90 ${minutes(cost.p90)} over ${cost.runs} successful runs, failed locally 0 of last ${cost.window}` };
}
