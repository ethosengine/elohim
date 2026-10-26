// A gate run as a placement Task under ark (`ark task spec` / `ark task run`).
//
// The gate's closure material (gate-evidence.mjs `closureMaterial`, the bytes `closureKey`
// hashes) becomes the Task's input, and ark computes every CID from it: the input root, the
// method CID and the exec binding. ark places the Task in the device's own envelope, looks the
// derivation up in brit under the same reuse rules as `reusableHit`, and runs
// run-local-gate.sh only on a miss, its output passed through unchanged so gate-tap.mjs still
// reads it. The brit attestation stays the verdict record and gate-runner.mjs stays its writer.
//
// Nothing here decides a verdict. When ark is unavailable the gate runs as it always has, with
// one visible line saying why.

import { spawnSync } from 'child_process';
import { createHash } from 'crypto';
import { accessSync, constants, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'fs';
import { isAbsolute, join, resolve } from 'path';
import { attestationStep } from './gate-evidence.mjs';

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
export function prepareTask(project, { root, env, childEnv, args, evidence, reuse, device, spawn = spawnSync, now = Date.now }) {
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
