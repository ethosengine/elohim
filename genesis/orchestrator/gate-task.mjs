// A gate run as a placement Task under ark (`ark task spec` / `ark task run`).
//
// The gate's closure material (gate-evidence.mjs `closureMaterial`, the bytes `closureKey`
// hashes) becomes the Task's input, and ark computes every CID from it: the input root, the
// method CID and the exec binding. Reuse is decided before a Task exists, by gate-runner.mjs's
// `decideReuse`, the same decision the direct path makes; the Task is handed reuse mode null, so
// ark places it in the device's own envelope and runs run-local-gate.sh, its output passed
// through unchanged so gate-tap.mjs still reads it. The brit attestation stays the verdict record
// and gate-runner.mjs stays its writer.
//
// Nothing here decides a verdict. When ark is unavailable the gate runs as it always has, with
// one visible line saying why.

import { spawnSync } from 'child_process';
import { createHash } from 'crypto';
import {
  accessSync, chmodSync, closeSync, constants, mkdirSync, mkdtempSync, openSync, readFileSync, readdirSync, realpathSync,
  renameSync, rmSync, statSync, writeFileSync,
} from 'fs';
import { isAbsolute, join, resolve } from 'path';
import { fileURLToPath } from 'url';
import { attestationStep } from './gate-evidence.mjs';
import { SUITE_LIST, blobAt, git, materialise, suiteInventory } from './suite-inventory.mjs';

/** The projects a Task runs for when GATE_TASK is unset: the proving gate only. */
export const TASK_DEFAULT_PROJECTS = ['orchestrator'];

/** ark's exit when the device's floor did not carry the Task here: nothing ran. */
export const EXIT_NOT_CARRIED = 75;

/**
 * The environment a Task child declares, by name, from the gate's own child environment. A
 * Task child never inherits ark's environment (env_scrub), and what it declares is bound into
 * the method CID, so this list is part of the method: no secrets, no per-run values.
 */
export const TASK_ENV = [
  'RUSTUP_HOME', 'CARGO_HOME', 'CARGO_TARGET_POOL_ROOT', 'CARGO_TARGET_POOL_FAMILY',
  'EPR_BIN', 'JENKINS_URL', 'NODE_EXTRA_CA_CERTS', 'PNPM_HOME',
  'XDG_CONFIG_HOME', 'XDG_CACHE_HOME', 'XDG_DATA_HOME', 'XDG_STATE_HOME', 'LANG',
  // The orchestrator suite's expectation (gated sha and key digests, colon-joined: no braces). It
  // varies per commit, so a Task's method does too; gate-runner.mjs decides reuse, so no derivation look-up depends on it.
  'GATE_SUITE_EXPECT',
  // The fresh directory one attempt of the named half writes its result to (per run).
  'GATE_SUITE_RESULT',
];

/** How many Task directories are kept under .eprfs/status/ark/tasks. */
const KEEP = 20;

/** GATE_TASK=on|off; unset runs a Task only for TASK_DEFAULT_PROJECTS. */
export function taskEnabled(project, env = process.env) {
  const declared = (env.GATE_TASK || '').trim();
  if (declared === 'off') return false;
  if (declared === 'on') return project.run?.kind !== 'attested';
  return TASK_DEFAULT_PROJECTS.includes(project.name);
}

function executable(path) {
  try {
    accessSync(path, constants.X_OK);
    return statSync(path).isFile();
  } catch {
    return false;
  }
}

/**
 * The ark binary: ARK_BIN when set, else the one `just gate elohim-ark` builds in this
 * checkout's elohim pool slot, provided no tracked ark source is newer than it. A stale or
 * missing binary is a reason, never a build: gate-runner does not start cargo.
 */
export function resolveArk({ root, env = process.env, spawn = spawnSync } = {}) {
  const declared = (env.ARK_BIN || '').trim();
  if (declared) {
    const path = isAbsolute(declared) ? declared : resolve(root, declared);
    return executable(path) ? { bin: path } : { why: `ARK_BIN ${path} is not executable` };
  }
  const slot = spawn('bash', ['-c', 'source "$1/genesis/agentic/bin/pool-lib.sh" && slot_path "$(detect_family "$1")" elohim dev', '_', root],
    { cwd: root, encoding: 'utf8', env });
  const dir = slot.status === 0 ? String(slot.stdout || '').trim() : '';
  if (!dir) return { why: 'the elohim pool slot is unresolvable' };
  const bin = join(dir, 'debug', 'ark');
  if (!executable(bin)) return { why: `no ark at ${bin}; run just gate elohim-ark or set ARK_BIN` };
  const built = statSync(bin).mtimeMs;
  const files = spawn('git', ['ls-files', '-z', 'elohim/ark'], { cwd: root, encoding: 'utf8' });
  if (files.status !== 0) return { why: 'ark sources unreadable' };
  for (const file of String(files.stdout).split('\0').filter(Boolean)) {
    if (!file.endsWith('.rs') && !file.endsWith('Cargo.toml')) continue;
    let mtime = 0;
    try { mtime = statSync(resolve(root, file)).mtimeMs; } catch { continue; }
    if (mtime > built) return { why: `${bin} is older than ${file}; run just gate elohim-ark or set ARK_BIN` };
  }
  return { bin };
}

/** The Task child's declared environment: the cargo env pairs, then TASK_ENV from the gate's. */
export function taskEnv(childEnv) {
  const env = {};
  try {
    Object.assign(env, JSON.parse(childEnv.GATE_CARGO_ENV || '{}'));
  } catch { /* run-local-gate.sh refuses a malformed value on the direct path */ }
  for (const key of TASK_ENV) {
    if (childEnv[key] !== undefined && childEnv[key] !== '') env[key] = String(childEnv[key]);
  }
  const dropped = [];
  for (const [key, value] of Object.entries(env)) {
    // `{…}` is ark's template syntax in argv and env; such a value cannot be passed literally.
    if (/[{}]/.test(value)) {
      delete env[key];
      dropped.push(key);
    }
  }
  return { env, dropped };
}

/** The request `ark task spec` turns into a TaskSpec: the method's identity, the child, no outputs. */
export function taskRequest(project, args, { root, env }) {
  const runner = resolve(root, 'genesis/orchestrator/run-local-gate.sh');
  return {
    method: {
      name: `brit-gate:${project.name}`,
      tools: { node: process.version },
      step: attestationStep(project),
      deterministic: true,
    },
    child: {
      artifact: { pinned: { sha256: createHash('sha256').update(readFileSync(runner)).digest('hex') } },
      argv: ['run-local-gate.sh', ...args],
      env,
    },
  };
}

function prune(dir) {
  try {
    const old = readdirSync(dir).sort().reverse().slice(KEEP);
    for (const name of old) rmSync(join(dir, name), { recursive: true, force: true });
  } catch { /* an unreadable directory keeps everything */ }
}

/**
 * Writes the Task's files and asks ark for its spec. Returns `{ bin, argv, status, derivation }`
 * for gateSpawn, or `{ why }` when the Task cannot be prepared.
 */
export function prepareTask(project, {
  root, env, childEnv, args, evidence, reuse, device, spawn = spawnSync, now = Date.now,
  write = text => process.stdout.write(text),
}) {
  if (!evidence?.material) return { why: `no closure material (evidence ${evidence?.line || 'unavailable'})` };
  const ark = resolveArk({ root, env, spawn });
  if (ark.why) return { why: ark.why };
  const tasks = resolve(root, '.eprfs/status/ark/tasks');
  const dir = join(tasks, `${new Date(now()).toISOString().replace(/[-:.]/g, '')}-${project.name}-${process.pid}`);
  mkdirSync(dir, { recursive: true });
  prune(tasks);
  const { env: declared, dropped } = taskEnv(childEnv);
  const files = {
    material: join(dir, 'material.json'),
    request: join(dir, 'request.json'),
    spec: join(dir, 'spec.json'),
    reuse: join(dir, 'reuse.json'),
    status: join(dir, 'status.jsonl'),
  };
  writeFileSync(files.material, evidence.material);
  writeFileSync(files.request, JSON.stringify(taskRequest(project, args, { root, env: declared })));
  writeFileSync(files.reuse, JSON.stringify(reuse));
  const made = spawn(ark.bin, ['task', 'spec', '--request', files.request, '--material', files.material],
    { cwd: root, encoding: 'utf8' });
  if (made.status !== 0) {
    return { why: `ark task spec refused: ${String(made.stderr || '').trim().split('\n').pop() || `exit ${made.status}`}` };
  }
  let spec;
  try {
    spec = JSON.parse(made.stdout);
  } catch {
    return { why: 'ark task spec printed no JSON' };
  }
  writeFileSync(files.spec, JSON.stringify(spec.spec));
  // Track A, shadow only and opt-in (GATE_SOURCE_VIEW=shadow): a candidate input root over the
  // gate's real source files, printed beside the legacy key. It never changes the Task, its key or
  // its verdict, and any failure is one line; its time does count in the gate's recorded seconds.
  if (sourceViewMode(env) === 'shadow') {
    try {
      write(`${sourceViewShadow(project, { root, ark: ark.bin, commit: reuse?.commit, evidence, spawn, now })}\n`);
    } catch (error) {
      write(`[gate-task] source-view shadow: unmeasured (${oneLine(error?.message || error)})\n`);
    }
  }
  const argv = [
    'task', 'run', '--spec', files.spec, '--material', files.material,
    '--artifact', resolve(root, 'genesis/orchestrator/run-local-gate.sh'),
    '--status', files.status, '--reuse', files.reuse, '--repo', root, '--data-root', root,
    ...(device ? ['--device', device] : []),
  ];
  return { bin: ark.bin, argv, status: files.status, derivation: spec.derivation, dropped };
}

/** The status lines ark appended, oldest first; an unreadable file is no lines. */
export function readTaskStatus(path) {
  try {
    return readFileSync(path, 'utf8').split('\n').filter(Boolean).map(line => JSON.parse(line));
  } catch {
    return [];
  }
}

/**
 * What a finished `ark task run` means to the gate: `{ answered }`, `{ unanswered }`
 * (interrupted: no verdict), or `{ fallback }` (ark answered nothing usable, or the floor did not
 * carry the Task here, so the gate runs directly).
 */
export function taskAnswer(statuses, exit) {
  const last = statuses.at(-1);
  if (!last || last.status === 'queued') return { fallback: `ark answered nothing (exit ${exit})` };
  if (last.status === 'answered') return { answered: last.value };
  if (last.status === 'unanswered') {
    return exit === EXIT_NOT_CARRIED
      ? { fallback: `not carried here (${JSON.stringify(last.value.why)})` }
      : { unanswered: last.value };
  }
  return { fallback: `unexpected status ${last.status}` };
}

// ---------------------------------------------------------------------------------------------
// Source view (Track A, shadow). The legacy input root is a tree of the closure material's JSON
// fields, so it moves whenever any keyed field does. The candidate is a projected view of the
// files the gate actually reads: the orchestrator suite's inventory at the gated commit
// (suite-inventory.mjs, the same function the key and the view builder use), materialised from
// git objects into a scratch checkout and captured by `ark task source-root`. Each run prints the
// two side by side and, against the last run's pair, whether each moved: agreement is about
// change, never about equal hashes. Nothing reads the line to decide anything.

/** The projects whose concrete source list is known: the suite inventory is orchestrator's. */
export const SOURCE_VIEW_PROJECTS = ['orchestrator'];

/** The whole shadow, inventory and capture included, is killed past this. */
export const SOURCE_VIEW_BUDGET_MS = 20_000;

/** A scratch run directory older than this is a leftover and is swept. */
export const SOURCE_VIEW_SWEEP_MS = 10 * 60_000;

/**
 * GATE_SOURCE_VIEW=shadow|off; unset is off. The shadow runs synchronously before the Task and
 * gate-runner.mjs counts that time in the gate's seconds, so it is opt-in.
 */
export function sourceViewMode(env = process.env) {
  return (env?.GATE_SOURCE_VIEW || '').trim() === 'shadow' ? 'shadow' : 'off';
}

/** Where the shadow's scratch run directories live: the checkout's own volume (see below). */
export function sourceViewBase(root) {
  return resolve(root, '.eprfs/status/ark/source-view');
}

/** Removes `run-*` scratch directories (and their output files) older than SOURCE_VIEW_SWEEP_MS. */
export function sweepSourceView(base, { now = Date.now } = {}) {
  let names = [];
  try { names = readdirSync(base); } catch { return 0; }
  let swept = 0;
  for (const name of names) {
    if (!name.startsWith('run-')) continue;
    const path = join(base, name);
    try {
      if (now() - statSync(path).mtimeMs < SOURCE_VIEW_SWEEP_MS) continue;
      rmSync(path, { recursive: true, force: true });
      swept++;
    } catch { /* gone already, or unreadable: the next sweep tries again */ }
  }
  return swept;
}

const oneLine = text => String(text).split('\n').map(line => line.trim()).filter(Boolean).pop() || 'no detail';
const short = (text, n = 12) => String(text || '').slice(0, n);

/**
 * The orchestrator gate's concrete files at `commit`, materialised under `dest`: every
 * superproject blob the view holds and every entry taken from a declared pin, repository-relative.
 */
export function sourceViewFiles({ root, commit, dest }) {
  const inventory = suiteInventory({ root, commit, listText: blobAt(root, commit, SUITE_LIST) });
  materialise(inventory, dest, { root });
  // materialise writes modes through the host umask, and capture hashes the executable bit, so each
  // regular file is set explicitly from its git mode (chmod is not subject to the umask).
  const files = [];
  const fix = (path, mode) => {
    if (mode === '100755' || mode === '100644') chmodSync(join(dest, path), mode === '100755' ? 0o755 : 0o644);
    files.push(path);
  };
  for (const entry of inventory.entries) fix(entry.path, entry.mode);
  for (const pin of inventory.pins) for (const entry of pin.entries) fix(`${pin.path}/${entry.path}`, entry.mode);
  return { files: files.sort(), inventory: inventory.digest };
}

/**
 * Runs in the child the parent spawns under SOURCE_VIEW_BUDGET_MS: resolves the commit, builds the
 * scratch checkout and asks ark for the projected root. Returns `{ commit, candidate, files }`.
 */
export function sourceViewCandidate({ root, commit = 'HEAD', ark, deadline, scratch, scratchBase, spawn = spawnSync }) {
  const sha = git(['rev-parse', '--verify', `${commit}^{commit}`], { cwd: root }).trim();
  // The scratch checkout and store sit on the checkout's own filesystem by default: capture syncs
  // every object, and on this workspace's overlay /tmp that alone outran the budget (23 s for 810
  // files against 8 s on the checkout's volume). They are siblings, so neither contains the other.
  // The parent hands the child its scratch directory and removes it itself: SIGKILL at the cap
  // skips this function's `finally`.
  const owned = !scratch;
  if (owned) {
    const base = scratchBase || sourceViewBase(root);
    mkdirSync(base, { recursive: true });
    scratch = mkdtempSync(join(realpathSync(base), 'run-'));
  }
  try {
    const checkout = join(scratch, 'checkout');
    mkdirSync(checkout);
    const { files } = sourceViewFiles({ root, commit: sha, dest: checkout });
    const list = join(scratch, 'files.json');
    writeFileSync(list, JSON.stringify(files));
    const timeout = Math.max(1000, (deadline ?? Date.now() + SOURCE_VIEW_BUDGET_MS) - Date.now());
    const made = spawn(ark, ['task', 'source-root', '--checkout', checkout, '--files', list, '--store', join(scratch, 'store')],
      { encoding: 'utf8', timeout, killSignal: 'SIGKILL', maxBuffer: 1024 * 1024 });
    if (made.error || made.status !== 0) {
      throw new Error(`ark task source-root: ${made.error?.message || oneLine(made.stderr || `exit ${made.status}`)}`);
    }
    const printed = JSON.parse(made.stdout);
    if (typeof printed?.projected !== 'string') throw new Error('ark task source-root printed no projected root');
    return { commit: sha, candidate: printed.projected, files: files.length };
  } finally {
    // A scratch handed in by the parent holds the parent's output files; the parent removes it.
    if (owned) rmSync(scratch, { recursive: true, force: true });
  }
}

/** Where the previous run's pair is kept, per project, to say what moved. */
function shadowStatePath(root) {
  return resolve(root, '.eprfs/status/ark/source-view-shadow.json');
}

/** The previous pairs, per project; a missing, unreadable or malformed file is no previous run. */
export function readShadowState(path) {
  try {
    const state = JSON.parse(readFileSync(path, 'utf8'));
    return state && typeof state === 'object' && !Array.isArray(state) ? state : {};
  } catch {
    return {};
  }
}

/** Writes the state whole: a temporary file in the same directory, then a rename over the old. */
export function writeShadowState(path, state) {
  const tmp = `${path}.${process.pid}.${Date.now()}.tmp`;
  try {
    writeFileSync(tmp, JSON.stringify(state));
    renameSync(tmp, path);
  } finally {
    rmSync(tmp, { force: true });
  }
}

/** SIGKILLs every process left in the group `pid` leads; an empty or absent group is no error. */
function killGroup(pid) {
  if (!Number.isInteger(pid) || pid <= 0) return;
  try { process.kill(-pid, 'SIGKILL'); } catch { /* the group is already empty */ }
}

/** `vs=…` and, against the previous pair, whether each key moved and whether they agree. */
export function sourceViewComparison(previous, current) {
  if (!previous?.legacy || !previous?.candidate) return 'vs=none';
  const legacy = previous.legacy === current.legacy ? 'same' : 'changed';
  const candidate = previous.candidate === current.candidate ? 'same' : 'changed';
  return `vs=${short(previous.commit)} legacy:${legacy} candidate:${candidate} ${legacy === candidate ? 'agree' : 'DISAGREE'}`;
}

/**
 * One shadow line for `project`. The candidate is computed in a child process killed at
 * SOURCE_VIEW_BUDGET_MS, so a stuck git or ark costs the gate at most that; every failure is
 * `unmeasured (<reason>)`. Never throws for a failure it can name.
 */
export function sourceViewShadow(project, {
  root, ark, commit, evidence, spawn = spawnSync, now = Date.now, execPath = process.execPath,
  budgetMs = SOURCE_VIEW_BUDGET_MS,
}) {
  const prefix = '[gate-task] source-view shadow:';
  if (!SOURCE_VIEW_PROJECTS.includes(project.name)) return `${prefix} unmeasured (no concrete source list for ${project.name})`;
  const legacy = evidence?.key || (evidence?.material ? createHash('sha256').update(evidence.material).digest('hex') : null);
  if (!legacy) return `${prefix} unmeasured (no legacy key)`;
  const started = now();
  const base = sourceViewBase(root);
  mkdirSync(base, { recursive: true });
  sweepSourceView(base);
  // The parent owns the run's scratch directory and the child's output files, and removes all of
  // them however the child ends. Output goes to files, not pipes, so a descendant that outlives
  // the child cannot hold spawnSync open past the cap.
  const scratch = mkdtempSync(join(realpathSync(base), 'run-'));
  const outPath = join(scratch, 'shadow.out');
  const errPath = join(scratch, 'shadow.err');
  let ran;
  let out = '';
  let err = '';
  try {
    const request = JSON.stringify({ root, commit: commit || 'HEAD', ark, scratch, deadline: Date.now() + budgetMs - 500 });
    const outFd = openSync(outPath, 'w');
    const errFd = openSync(errPath, 'w');
    try {
      // detached: the child leads its own process group, so the cap can kill every descendant.
      ran = spawn(execPath, [fileURLToPath(import.meta.url), 'source-view', request],
        { cwd: root, stdio: ['ignore', outFd, errFd], detached: true, timeout: budgetMs, killSignal: 'SIGKILL' });
    } finally {
      killGroup(ran?.pid);
      closeSync(outFd);
      closeSync(errFd);
    }
    try { out = readFileSync(outPath, 'utf8'); } catch { /* no output */ }
    try { err = readFileSync(errPath, 'utf8'); } catch { /* no output */ }
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
  const ms = Math.round(now() - started);
  // A fake spawn in tests may answer with its own streams.
  const stdout = out || String(ran?.stdout || '');
  const stderr = err || String(ran?.stderr || '');
  if (ran?.error?.code === 'ETIMEDOUT' || (ran?.signal && ran?.status === null)) {
    return `${prefix} unmeasured (timed out after ${budgetMs / 1000} s)`;
  }
  if (ran?.error || ran?.status !== 0) {
    return `${prefix} unmeasured (${oneLine(ran?.error?.message || stderr || `exit ${ran?.status}`)})`;
  }
  let got;
  try { got = JSON.parse(stdout); } catch { return `${prefix} unmeasured (the shadow printed no JSON)`; }
  if (typeof got?.candidate !== 'string' || typeof got?.commit !== 'string') return `${prefix} unmeasured (the shadow printed no candidate)`;
  const current = { commit: got.commit, legacy, candidate: got.candidate };
  const statePath = shadowStatePath(root);
  const state = readShadowState(statePath);
  const comparison = sourceViewComparison(state[project.name], current);
  try {
    writeShadowState(statePath, { ...state, [project.name]: current });
  } catch { /* an unwritable state only loses the next comparison */ }
  return `${prefix} project=${project.name} commit=${short(got.commit)} legacy=${short(legacy)} candidate=${got.candidate} files=${got.files} ms=${ms} ${comparison}`;
}

// `node gate-task.mjs source-view <request-json>`: the child sourceViewShadow spawns.
if (process.argv[2] === 'source-view' && process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    process.stdout.write(`${JSON.stringify(sourceViewCandidate(JSON.parse(process.argv[3] || '{}')))}\n`);
  } catch (error) {
    process.stderr.write(`${oneLine(error?.message || error)}\n`);
    process.exitCode = 1;
  }
}
