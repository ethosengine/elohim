// The attested gate — a submodule pin is gated by the pinned commit's own
// upstream attestation. Reads the pin (never the worktree), reads the named
// check at that SHA, and records the read as a pin-attestation@1 observation.
//
// Reading is free, trusting is a choice: an unreachable read passes as
// `claimed` (memoized-derivation Law III); a red, cancelled, pending or absent
// check refuses — and "the forge has never seen this commit" is absent, not
// unreachable. Nothing here ever runs the component's suite.

import { accessSync, constants, readFileSync, statSync } from 'fs';
import { spawnSync } from 'child_process';
import { delimiter, isAbsolute, join, resolve } from 'path';
import { fileURLToPath } from 'url';

export const PIN_ATTESTATION_MEASURE = 'pin-attestation@1';
/** A reachable green is 0, a witnessed refusal is 1, a manifest error is 2, and claimed is typed. */
export const CLAIMED_EXIT_CODE = 3;
/** The floor never blocks on the network: a stalled forge read degrades to `claimed`. */
export const GH_TIMEOUT_MS = 60_000;

const RED = new Set(['failure', 'cancelled', 'timed_out', 'action_required', 'stale', 'startup_failure']);
/** A definitive "no such commit / no such repo" from the forge — the pin has no attestation. */
const DEFINITIVE_ABSENT = /HTTP (404|422)\b/;
const GITLINK_MODE = '160000';

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

/**
 * A git hook exports GIT_DIR (and friends) pinned to the superproject; a git spawned
 * with those set answers about the superproject regardless of `-C`. Gate children get
 * the same scrub in gate-runner.mjs (gateChildEnv).
 */
export function scrubbedGitEnv(env = process.env) {
  const out = { ...env };
  for (const k of ['GIT_DIR', 'GIT_WORK_TREE', 'GIT_INDEX_FILE', 'GIT_PREFIX']) delete out[k];
  return out;
}

export class ManifestError extends Error {}

/** The pin: `ls-tree HEAD -- <dir>` must answer with a gitlink (mode 160000). */
export function pinnedSha(root, dir, { spawn = spawnSync, env = process.env } = {}) {
  const result = spawn('git', ['-C', root, 'ls-tree', 'HEAD', '--', dir], { encoding: 'utf8', env: scrubbedGitEnv(env) });
  const line = (result?.stdout || '').trim().split('\n')[0] || '';
  const match = /^(\d{6}) (\w+) ([0-9a-f]{40})\t/.exec(line);
  if (result?.status !== 0 || !match) {
    throw new ManifestError(`${dir} is not a gitlink at HEAD (${(result?.stderr || 'no such path').trim()})`);
  }
  if (match[1] !== GITLINK_MODE) {
    throw new ManifestError(`${dir} is not a gitlink at HEAD (mode ${match[1]} ${match[2]})`);
  }
  return match[3];
}

/** The superproject's own view: does the pin path differ from HEAD (new head or dirty content)? */
export function worktreeDirty(root, dir, { spawn = spawnSync, env = process.env } = {}) {
  const result = spawn('git', ['-C', root, 'status', '--porcelain', '--', dir], { encoding: 'utf8', env: scrubbedGitEnv(env) });
  return result?.status === 0 && (result.stdout || '').trim().length > 0;
}

export function readCheck(attestation, sha, { ghBin, spawn = spawnSync, now = Date.now } = {}) {
  if (!ghBin) return { read: 'failed', reason: 'gh not found' };
  // Filter by name on the server: a busy commit carries more check runs than one page holds (brit's
  // main tip had 132 on 2026-10-24), and a named run past the first page would read as absent.
  const began = now();
  const query = `check_name=${encodeURIComponent(attestation.check)}&per_page=100`;
  const result = spawn(ghBin, ['api', `repos/${attestation.repo}/commits/${sha}/check-runs?${query}`], {
    encoding: 'utf8',
    timeout: GH_TIMEOUT_MS,
  });
  if (!result || result.status !== 0) {
    if (result?.error?.code === 'ETIMEDOUT') return { read: 'failed', reason: `gh timed out after ${GH_TIMEOUT_MS / 1000}s` };
    const stderr = (result?.stderr || '').trim();
    if (DEFINITIVE_ABSENT.test(stderr)) return { read: 'ok', status: 'absent', conclusion: 'absent' };
    return { read: 'failed', reason: stderr || result?.error?.message || 'gh exited non-zero' };
  }
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
  const read = { read: 'ok', status: named[0].status, conclusion: named[0].conclusion };
  if (read.status !== 'completed') return read;
  // One budget for both reads: the second gets what is left of 60 s, at most 15 s. It bounds the two reads, not the process:
  // start-up, child termination and the observation runAttested records afterwards are outside it, and the bin store's caller allows 75 s.
  const timeout = Math.min(SECOND_READ_MAX_MS, GH_TIMEOUT_MS - (now() - began));
  return { ...read, ...newerRun(attestation, sha, named[0].check_suite?.id, { ghBin, spawn, timeout }) };
}

/** The second read's share of the first read's budget: the bin store kills this process at 75 s, so both reads fit one 60 s. */
export const SECOND_READ_MAX_MS = 15_000;

/** GitHub's run states that mean a run is still on its way; `completed` is done; any other word is not understood. */
const IN_FLIGHT = new Set(['queued', 'in_progress', 'waiting', 'requested', 'pending']);

const idOf = v => (typeof v === 'string' && v !== '') || (typeof v === 'number' && Number.isFinite(v)) ? String(v) : null;
const startOf = r => {
  for (const v of [r.run_started_at, r.created_at]) {
    const t = typeof v === 'string' ? Date.parse(v) : NaN;
    if (Number.isFinite(t)) return t;
  }
  return null;
};

/**
 * The newest named check is only the newest reading if no later run of its workflow is still on
 * its way: that run's aggregate check does not exist until it concludes, so the check list cannot
 * show it. brit 02d4654fa73d carried two runs of `ci` on 2026-11-21; the gate read the older run's
 * green while the push-to-main run was still running, and that run ended red (backlog row 71,
 * 2026-11-22). A re-run keeps its run id and suite and yields a new check only when it finishes, so
 * the judged run itself not being completed also means the check read belongs to an older attempt.
 * Returns the fields to merge into the read: a non-completed status and `waitsOn` when such a run
 * exists, `unchecked` (a reason) when this read cannot say, nothing otherwise. Anything uncertain
 * fails open: an unreachable or malformed read never refuses; it is named on the line instead.
 */
function newerRun(attestation, sha, suiteId, { ghBin, spawn, timeout }) {
  if (timeout <= 0) return { unchecked: 'the time budget was spent' };
  const result = spawn(ghBin, ['api', `repos/${attestation.repo}/actions/runs?head_sha=${sha}&per_page=100`], {
    encoding: 'utf8',
    timeout,
  });
  if (!result || result.status !== 0) {
    if (result?.error?.code === 'ETIMEDOUT') return { unchecked: `gh timed out after ${timeout / 1000}s` };
    const stderr = (result?.stderr || '').trim().split('\n')[0];
    return { unchecked: stderr || result?.error?.message || 'gh exited non-zero' };
  }
  let runs;
  try {
    runs = JSON.parse(result.stdout).workflow_runs;
  } catch {
    return { unchecked: 'unparsable workflow-runs response' };
  }
  if (!Array.isArray(runs) || runs.some(r => !r || typeof r !== 'object')) return { unchecked: 'unparsable workflow-runs response' };
  const suite = idOf(suiteId);
  const judged = suite === null ? null : runs.find(r => idOf(r.check_suite_id) === suite);
  if (!judged) return { unchecked: 'the judged run is not in the workflow-run list' };
  const workflow = idOf(judged.workflow_id);
  const judgedStart = startOf(judged);
  if (workflow === null || judgedStart === null || (judged.status !== 'completed' && !IN_FLIGHT.has(judged.status))) {
    return { unchecked: 'the judged run lacks a workflow id, status or start time' };
  }
  // Display fields come from the API: only a non-empty string name and a normalised id reach the line.
  const waitsOn = r => ({ status: r.status, conclusion: null, waitsOn: { name: typeof r.name === 'string' && r.name !== '' ? r.name : 'unnamed workflow', id: idOf(r.id) ?? 'unknown run', status: r.status } });
  // A completed judged run's `conclusion` is not consulted: the newest named check stays the newest reading even if it started
  // before the run's latest attempt. A re-run of only the failed jobs legitimately leaves an earlier green check in place, and
  // telling that from a full re-run cancelled before the check ran needs the run's job list, which this gate does not read
  // (backlog steward-device-arrival-open-items row 71).
  if (judged.status !== 'completed') return { ...waitsOn(judged), waitsOn: { ...waitsOn(judged).waitsOn, rerun: true } };
  let unreadable = false;
  const waiting = [];
  for (const r of runs) {
    if (r === judged || idOf(r.workflow_id) !== workflow) continue;
    const start = startOf(r);
    if (r.status === 'completed') continue;
    if (!IN_FLIGHT.has(r.status) || start === null) { unreadable = true; continue; }
    if (start > judgedStart) waiting.push({ r, start });
  }
  if (waiting.length > 0) return waitsOn(waiting.sort((a, b) => b.start - a.start)[0].r);
  return unreadable ? { unchecked: 'a run of the same workflow could not be read, so newer runs were not fully checked' } : {};
}

export function judge(read, attestation, sha) {
  const verdict = judgeRead(read, attestation, sha);
  // The second read could not say whether a newer run is coming: the verdict stands, and the line says so.
  return read.unchecked ? { ...verdict, line: `${verdict.line} (newer runs not checked: ${read.unchecked})` } : verdict;
}

function judgeRead(read, attestation, sha) {
  const short = sha.slice(0, 12);
  const where = `${attestation.repo}@${short} ${attestation.check}`;
  if (read.read !== 'ok') {
    return { outcome: 'pass', tier: 'claimed', conclusion: 'unreachable', line: `attested: claimed — ${read.reason}` };
  }
  if (read.status === 'absent') {
    return { outcome: 'refuse', tier: 'witnessed', conclusion: 'absent', line: `attested: REFUSED — ${where} has no run at ${short} — a pin without its attestation is not a green pin` };
  }
  if (read.waitsOn) {
    const { name, id, status, rerun } = read.waitsOn;
    const what = rerun ? 'a re-run of the same run' : 'a newer run of the same workflow';
    return { outcome: 'refuse', tier: 'witnessed', conclusion: 'pending', line: `attested: REFUSED — ${where} not yet concluded at ${short}: ${what} (${name}, run ${id}, ${status}) has not produced this check yet` };
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

/** Judge one registered gitlink SHA without changing gate semantics or recording an observation. */
export function judgePin(dir, sha, { root = process.cwd(), env = process.env, spawn = spawnSync } = {}) {
  const registryPath = resolve(root, 'genesis/orchestrator/manifests/runtime/workspace-device.manifest.json');
  let registry;
  try {
    registry = JSON.parse(readFileSync(registryPath, 'utf8'));
  } catch (error) {
    throw new ManifestError(`cannot read followed-tool registry ${registryPath}: ${error.message}`);
  }
  const entry = Object.values(registry.tools || {}).find(tool => tool?.source?.path === dir);
  const attestation = entry?.source?.attestation;
  if (!attestation?.provider || !attestation?.repo || !attestation?.check) {
    throw new ManifestError(`${dir} has no complete followed-tool attestation`);
  }
  return judge(readCheck(attestation, sha, { ghBin: resolveGhBin(env), spawn }), attestation, sha);
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

/** Run one attested gate project. Returns 0 witnessed pass, 1 refusal, 2 manifest error, 3 claimed. */
export function runAttested(project, { root, env = process.env, spawn = spawnSync, log, runEpr }) {
  const say = log || (line => process.stdout.write(`${line}\n`));
  const attestation = project.run?.attestation;
  if (!attestation?.provider || !attestation?.repo || !attestation?.check) {
    say(`gate ${project.name}: attested project without a complete attestation (provider/repo/check)`);
    return 2;
  }
  let sha;
  try {
    sha = pinnedSha(root, project.dir, { spawn, env });
  } catch (error) {
    say(`gate ${project.name}: ${error.message}`);
    return 2;
  }
  if (worktreeDirty(root, project.dir, { spawn, env })) {
    say('attested: worktree dirty — the committed pin is what is read; the component’s own gate governs the worktree');
  }
  const verdict = judge(readCheck(attestation, sha, { ghBin: resolveGhBin(env), spawn }), attestation, sha);
  say(verdict.line);
  try {
    const record = runEpr || (args => spawn(env.EPR_BIN || 'epr', args, { cwd: root, stdio: 'ignore', timeout: 30000 })?.status);
    record(observationArgs(project.dir, attestation, verdict));
  } catch { /* best-effort; the verdict never depends on the observation */ }
  if (verdict.tier === 'claimed') return CLAIMED_EXIT_CODE;
  return verdict.outcome === 'pass' ? 0 : 1;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  const dirAt = args.indexOf('--judge');
  const shaAt = args.indexOf('--sha');
  if (dirAt < 0 || shaAt < 0 || !args[dirAt + 1] || !/^[0-9a-f]{40}$/.test(args[shaAt + 1])) {
    process.stderr.write('usage: gate-attest.mjs --judge <gitlink-dir> --sha <full-sha>\n');
    process.exitCode = 2;
  } else {
    try {
      const verdict = judgePin(args[dirAt + 1], args[shaAt + 1]);
      process.stdout.write(`${JSON.stringify(verdict)}\n`);
      process.exitCode = verdict.tier === 'claimed'
        ? CLAIMED_EXIT_CODE
        : verdict.outcome === 'pass' ? 0 : 1;
    } catch (error) {
      process.stderr.write(`${error.message}\n`);
      process.exitCode = 2;
    }
  }
}
