// The attested gate — a submodule pin is gated by the pinned commit's own
// upstream attestation. Reads the pin (never the worktree), reads the named
// check at that SHA, and records the read as a pin-attestation@1 observation.
//
// Reading is free, trusting is a choice: an unreachable read passes as
// `claimed` (memoized-derivation Law III); a red, cancelled, pending or absent
// check refuses. Nothing here ever runs the component's suite.

import { accessSync, constants, statSync } from 'fs';
import { spawnSync } from 'child_process';
import { delimiter, isAbsolute, join } from 'path';

export const PIN_ATTESTATION_MEASURE = 'pin-attestation@1';

const RED = new Set(['failure', 'cancelled', 'timed_out', 'action_required', 'stale', 'startup_failure']);

function isExecutable(candidate) {
  try {
    if (!statSync(candidate).isFile()) return false;
    accessSync(candidate, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

export function resolveGhBin(env = process.env) {
  const declared = (env.GH_BIN || '').trim();
  if (declared) return isExecutable(isAbsolute(declared) ? declared : join(process.cwd(), declared)) ? declared : null;
  for (const dir of (env.PATH || '').split(delimiter).filter(Boolean)) {
    const candidate = join(dir, 'gh');
    if (isExecutable(candidate)) return candidate;
  }
  return null;
}

export class ManifestError extends Error {}

export function pinnedSha(root, dir, { spawn = spawnSync } = {}) {
  const result = spawn('git', ['-C', root, 'rev-parse', `HEAD:${dir}`], { encoding: 'utf8' });
  const sha = (result?.stdout || '').trim();
  if (result?.status !== 0 || !/^[0-9a-f]{40}$/.test(sha)) {
    throw new ManifestError(`${dir} is not a gitlink at HEAD (${(result?.stderr || '').trim()})`);
  }
  return sha;
}

export function worktreeDirty(root, dir, { spawn = spawnSync } = {}) {
  const result = spawn('git', ['-C', join(root, dir), 'status', '--porcelain'], { encoding: 'utf8' });
  return result?.status === 0 && (result.stdout || '').trim().length > 0;
}

export function readCheck(attestation, sha, { ghBin, spawn = spawnSync } = {}) {
  if (!ghBin) return { read: 'failed', reason: 'gh not found' };
  const result = spawn(ghBin, ['api', `repos/${attestation.repo}/commits/${sha}/check-runs?per_page=100`], { encoding: 'utf8' });
  if (!result || result.status !== 0) return { read: 'failed', reason: (result?.stderr || 'gh exited non-zero').trim() };
  let runs;
  try {
    runs = JSON.parse(result.stdout).check_runs || [];
  } catch {
    return { read: 'failed', reason: 'unparsable check-runs response' };
  }
  const named = runs
    .filter(r => r.name === attestation.check)
    .sort((a, b) => String(b.started_at || '').localeCompare(String(a.started_at || '')));
  if (named.length === 0) return { read: 'ok', status: 'absent', conclusion: 'absent' };
  return { read: 'ok', status: named[0].status, conclusion: named[0].conclusion };
}

export function judge(read, attestation, sha) {
  const short = sha.slice(0, 12);
  const where = `${attestation.repo}@${short} ${attestation.check}`;
  if (read.read !== 'ok') {
    return { outcome: 'pass', tier: 'claimed', conclusion: 'unreachable', line: `attested: claimed — ${read.reason}` };
  }
  if (read.status === 'absent') {
    return { outcome: 'refuse', tier: 'witnessed', conclusion: 'absent', line: `attested: REFUSED — ${where} has no run at ${short} — a pin without its attestation is not a green pin` };
  }
  if (read.status !== 'completed') {
    return { outcome: 'refuse', tier: 'witnessed', conclusion: 'pending', line: `attested: REFUSED — ${where} not yet concluded (${read.status}) at ${short}` };
  }
  if (read.conclusion === 'success') {
    return { outcome: 'pass', tier: 'witnessed', conclusion: 'success', line: `attested: ${where} success` };
  }
  const label = RED.has(read.conclusion) ? read.conclusion : `${read.conclusion} (not success)`;
  return { outcome: 'refuse', tier: 'witnessed', conclusion: 'failure', line: `attested: REFUSED — ${where} ${label} at ${short} — a pin without its attestation is not a green pin` };
}

export function observationArgs(subject, attestation, verdict) {
  return [
    'flow', 'note', '--kind', 'observation',
    '--measure', PIN_ATTESTATION_MEASURE,
    '--subject', subject, '--value', '1', '--unit', 'reads',
    '--env', `provider=${attestation.provider}`,
    '--env', `check=${attestation.check}`,
    '--env', `conclusion=${verdict.conclusion}`,
    '--env', `tier=${verdict.tier}`,
  ];
}

/** Run one attested gate project. Returns the exit status (0 pass, 1 refuse, 2 manifest error). */
export function runAttested(project, { root, env = process.env, spawn = spawnSync, log, runEpr }) {
  const say = log || (line => process.stdout.write(`${line}\n`));
  const attestation = project.run.attestation;
  let sha;
  try {
    sha = pinnedSha(root, project.dir, { spawn });
  } catch (error) {
    say(`gate ${project.name}: ${error.message}`);
    return 2;
  }
  if (worktreeDirty(root, project.dir, { spawn })) {
    say('attested: worktree dirty — the committed pin is what is read; the component’s own gate governs the worktree');
  }
  const verdict = judge(readCheck(attestation, sha, { ghBin: resolveGhBin(env), spawn }), attestation, sha);
  say(verdict.line);
  try {
    const record = runEpr || (args => spawn(env.EPR_BIN || 'epr', args, { cwd: root, stdio: 'ignore', timeout: 30000 })?.status);
    record(observationArgs(project.dir, attestation, verdict));
  } catch { /* best-effort; the verdict never depends on the observation */ }
  return verdict.outcome === 'pass' ? 0 : 1;
}
