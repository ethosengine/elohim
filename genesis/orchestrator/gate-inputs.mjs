#!/usr/bin/env node
// gate-inputs: which of a manifest step's two input classes a device gate selects and keys on, and
// (for now) a shadow that says what the narrower class would change.
//
// A step declares `sources` (what any executor of the step reads) and `buildProcess` (the files that
// define the process of the executor that runs it: a pipeline file, a CI image-build script). The
// class boundary is the EXECUTOR, not the field name:
//   - a gate project's OWN `inputs` block (`gate.projects.<name>.inputs`) is the gate's, both
//     classes; there `buildProcess` names the gate's own scripts, and nothing is ever dropped from it;
//   - a step the gate BORROWS from a pipeline (`gate.projects.<name>.steps`, or every step of the
//     manifest when a project names neither) would contribute its sources class only: its `sources`
//     minus its own `buildProcess`, each with a trailing `@identifier` suffix stripped (rakia-core
//     `inputs.rs` `step_input_patterns(…, Sources)`, the rule `brit build … --inputs sources` runs).
// CI dispatch is untouched: a pipeline file IS an input of the CI executor.
//
// The contract. An input class is an implementor of three things, and this module owns both:
//   name        'all' | 'sources'
//   britArgs    what `brit build affected` and `brit build fingerprint` are given
//   stepReasons (inputs, changedFiles) → graph-walker-shaped reasons a borrowed step is selected by
// `LEGACY_ALL` is today's bytes everywhere: no flag (never `--inputs all`, which strips suffixes and
// so names other bytes), and graph-walker's matcher as it was. `SOURCES` is the rule above. Selection
// (graph-walker's local gate path), propagation (gate-oracle) and the key (gate-evidence) take the
// class from ONE frozen policy per run; today that class is always LEGACY_ALL.
//
// Switch: GATE_INPUTS=shadow|off, default shadow. There is deliberately no `on` yet: `on`, like any
// other value, reads as shadow.
//   off     no probe, no line.
//   shadow  decides, records and times as off does. When brit is capable it says, once per push,
//           which borrowed steps `all` selects and `sources` would not, and per gate when, since the
//           integration base, the step fingerprints moved under all and are unchanged under sources.
//           That line speaks of step fingerprints only: the manifest, the recipe, the project tree
//           and the audited reads are the rest of a gate's key, and it does not compare them.
//           Both lines can only be non-empty when a changed path is a step's OWN buildProcess file
//           (the two classes differ by exactly those files), so that is decided first, from the
//           change set and a cached `git diff --name-only` of the range, with no brit spawn. Only a
//           gate whose borrowed step's buildProcess file changed fingerprints, and only those steps.
//           The work, the capability probe included, runs inside a per-process budget of the
//           shadow's own elapsed time (SHADOW_BUDGET, summed over its calls, so a gate running
//           between two calls spends none of it): every subprocess gets min(what remains, per-call
//           cap) and SIGKILL at it. A call that fails for want of the flag ends shadow's printing for
//           the process with one line.
// Capability is `brit build affected --help` and `brit build fingerprint --help` both naming
// `--inputs` and `sources`, probed once per resolved brit executable (realpath, device, inode, size,
// mtime), each probe at most PROBE_TIMEOUT_MS and killed at it. Under pre-push the hook runs
// `gate-inputs.mjs resolve` first, which writes `{ mode, brit, capable }` atomically to the file
// GATE_INPUTS_STATE names; a gate child accepts it only for the same mode and the same brit identity,
// and otherwise probes for itself, so the probe is paid once per push when the handoff holds and once
// per process when it does not. A failed or unknown probe prints `[gate] inputs: --inputs unavailable;
// selection and keys use legacy all` (without the probe's cause) once per push or per process, and
// shadow prints nothing else.
// Cost. A push that changes no step's buildProcess file pays the probe (two `--help`, at most 2 s
// each, once per push with the state handoff) and per gate process at most two cheap git reads
// (merge-base, then one cached `diff --name-only`; none when the base is the gated commit), and no
// other brit spawn. When such a file changed, the shadow's subprocess time is bounded by its budget:
// at most 4 s of probes, 5 s for selection's second `brit build affected`, and 10 s per gate process
// whose borrowed step was touched, so 9 + 10M s for M such gates with the handoff, 13 + 10M s
// without (synchronous JS between spawns is not preempted and is not counted). A standalone
// `just gate` runs every project in one process, which shares one 10 s budget.
//
// The shadow is an instrument with an end. Once a capable brit is installed it costs, on a push that
// changes a step's buildProcess file, three brit fingerprint spawns per touched borrowed step and one
// more `brit build affected`. It retires when `on` lands, or when enough pushes have printed its lines to decide that
// `on` is not wanted, whichever comes first; then the switch and the shadow path are deleted, not left.
//
// What `on` needs first (independent review, astra-inputs-review.receipt.md in the wall-clock
// sprint's campaign directory):
//   1. A differential fixture of the JS sources rule against a real capable brit. The JS matcher is
//      graph-walker's picomatch, not rakia's globset; they differ for `src/**` over `src/.config`,
//      `src/*` over `src/nested/a.rs`, a `.epr-meta` path (JS drops it), and a gitlink a glob
//      reaches (`vendor/lib/src/*.rs` over `vendor/lib`). Share the native matcher or refuse those.
//   2. A decision that a capability lost mid-push refuses the gate with a named cure, instead of
//      downgrading: a downgrade after selection cannot restore a gate selection already omitted.
//   3. Selection and execution agreeing in a standalone changed-project `just gate`, whose project
//      list is computed before any gate runs.
//   4. The executor-boundary contract test checking a gate's audited recipe reads, not only the
//      filenames its justfile names: a recipe that runs a script that reads `scripts/ci/x.sh` passes
//      it today.
//   5. The five parked manifest entries corrected (pipeline-registry.test.mjs PARKED): two gate-only
//      TypeScript steps list a lockfile, a workspace file and a codegen script as buildProcess, and
//      their recipes read them. Under SOURCES a lockfile bump would not select or re-key those two
//      gates. The correction is an Edge build input in CI's eyes and waits for a deliberate roll.
// The first run under the new key is a genuine run, and there is no bridge from legacy evidence: a
// verdict is reusable only for an identical closure, and identical closures recur within hours (a
// retry, a re-gate after a moved base, a stacked landing, a sibling's check), so the cost is at
// most one run of each gate whose closure did not change, once. A promoted record would cost a
// derived-record format and its lifecycle (a red arriving between the check and the write; a later
// legacy failure the promoted green would outlive).

import { spawnSync } from 'child_process';
import { readFileSync, realpathSync, renameSync, statSync, writeFileSync } from 'fs';
import { resolve } from 'path';
import picomatch from 'picomatch';

export const INPUTS_MODES = Object.freeze(['off', 'shadow']);
export const INPUTS_STATE_ENV = 'GATE_INPUTS_STATE';
export const UNAVAILABLE_LINE = '[gate] inputs: --inputs unavailable; selection and keys use legacy all';
/** Each `--help` probe is bounded and killed at it; a probe that does not answer in time is legacy. */
export const PROBE_TIMEOUT_MS = 2_000;
/** Shadow's own elapsed time per process, summed over its calls, and the cap on any one of its subprocesses. */
export const SHADOW_BUDGET = Object.freeze({ totalMs: 10_000, callMs: 5_000 });

const stderrLog = line => process.stderr.write(`${line}\n`);

/** `GATE_INPUTS=shadow|off`, default shadow. There is deliberately no `on` yet; it reads as shadow. */
export function inputsMode(env = process.env) {
  return (env.GATE_INPUTS || '').trim() === 'off' ? 'off' : 'shadow';
}

// A governance path is never a source change: `.epr-meta` (the compose-gate manifest file, or a
// directory of habit atoms and rules) sits inside the tree it governs, so a source glob matches it.
// Mirrors build-graph.groovy's isGovernancePath; orchestrator #1987 rolled the edge fleet for an edit
// to elohim/elohim-storage/src/.epr-meta.
export function isGovernancePath(filePath) {
  const normalized = filePath.startsWith('./') ? filePath.slice(2) : filePath;
  return normalized.split('/').includes('.epr-meta');
}

/**
 * Today's matcher (graph-walker's, unchanged): one `source:` reason per source pattern that a
 * changed file matches, then one `buildProcess:` reason per entry whose file (cut at its first `@`)
 * is in the change set. A gate project's own `inputs` block is always matched by this, with
 * governance paths included.
 */
export function matchInputs(inputs, changedFiles, { includeGovernance = false } = {}) {
  const reasons = [];
  if (!inputs) return reasons;

  for (const pattern of inputs.sources || []) {
    const matcher = picomatch(pattern);
    for (const file of changedFiles) {
      if (!includeGovernance && isGovernancePath(file)) continue;
      if (matcher(file)) {
        reasons.push(`source: ${file}`);
        break;
      }
    }
  }

  for (const ref of inputs.buildProcess || []) {
    const fileName = ref.split('@')[0];
    if (changedFiles.includes(fileName)) reasons.push(`buildProcess: ${ref}`);
  }
  return reasons;
}

/**
 * rakia-core `strip_function_suffix`: only a final `@` followed by an identifier
 * (`[A-Za-z][A-Za-z0-9_]*`) with a non-empty path before it is a suffix, so `node_modules/@scope/pkg`,
 * `ci/file@1x` and `@build` are returned whole.
 */
export function stripFunctionSuffix(entry) {
  const at = entry.lastIndexOf('@');
  if (at <= 0) return entry;
  return /^[A-Za-z][A-Za-z0-9_]*$/.test(entry.slice(at + 1)) ? entry.slice(0, at) : entry;
}

/** The sources class of a step's inputs, as patterns: rakia-core `step_input_patterns(…, Sources)`. */
export function sourcesPatterns(inputs) {
  return {
    include: [...(inputs?.sources || [])],
    exclude: (inputs?.buildProcess || []).map(stripFunctionSuffix),
  };
}

function sourcesStepReasons(inputs, changedFiles) {
  const reasons = [];
  if (!inputs) return reasons;
  const { include, exclude } = sourcesPatterns(inputs);
  const excluded = exclude.map(pattern => picomatch(pattern));
  for (const pattern of include) {
    const matcher = picomatch(pattern);
    for (const file of changedFiles) {
      if (isGovernancePath(file)) continue;
      if (matcher(file) && !excluded.some(m => m(file))) {
        reasons.push(`source: ${file}`);
        break;
      }
    }
  }
  return reasons;
}

/** Today's bytes: no flag, today's matcher. */
export const LEGACY_ALL = Object.freeze({
  name: 'all',
  britArgs: Object.freeze([]),
  stepReasons: (inputs, changedFiles) => matchInputs(inputs, changedFiles),
});

/** A borrowed step's sources minus its own buildProcess. */
export const SOURCES = Object.freeze({
  name: 'sources',
  britArgs: Object.freeze(['--inputs', 'sources']),
  stepReasons: sourcesStepReasons,
});

/** Whether a brit call failed for want of `--inputs` (clap's unknown-argument or invalid-value refusal). */
export function refusesInputsFlag(result) {
  if (!result || result.status === 0 || result.status == null) return false;
  const text = String(result.stderr || '');
  return /--inputs/.test(text) && /(unexpected argument|unrecognized|unknown argument|invalid value|wasn't expected|found argument)/i.test(text);
}

/** The brit executable's identity: what a cached or handed-down capability answer is bound to. */
export function britIdentity(britBin) {
  if (!britBin) return null;
  try {
    const path = realpathSync(resolve(britBin));
    const s = statSync(path);
    return { path, stamp: `${s.dev}:${s.ino}:${s.size}:${Math.trunc(s.mtimeMs)}` };
  } catch {
    return null;
  }
}

/** Both `--help` texts name `--inputs` and `sources`: `{ capable, why }`. Never throws. */
export function probeBrit(britBin, { spawn = spawnSync } = {}) {
  for (const verb of ['affected', 'fingerprint']) {
    let r;
    try {
      r = spawn(britBin, ['build', verb, '--help'], { encoding: 'utf8', timeout: PROBE_TIMEOUT_MS, killSignal: 'SIGKILL' });
    } catch (error) {
      return { capable: false, why: `brit build ${verb} --help could not run (${String(error?.message || error).split('\n')[0]})` };
    }
    if (r?.error) return { capable: false, why: `brit build ${verb} --help could not run (${r.error.code || r.error.message})` };
    if (r?.status !== 0) return { capable: false, why: `brit build ${verb} --help exited ${r?.status ?? 'without a status'}` };
    const help = String(r.stdout || '');
    if (!/--inputs\b/.test(help) || !/\bsources\b/.test(help)) return { capable: false, why: `brit build ${verb} has no --inputs` };
  }
  return { capable: true, why: null };
}

function policyOf({ mode, capable, why, britBin }) {
  const usable = mode === 'shadow' && capable === true;
  return Object.freeze({
    mode,
    /** The class selection, propagation and the key use: LEGACY_ALL until `on` exists. */
    cls: LEGACY_ALL,
    /** The class the shadow compares with, or null. */
    shadow: usable ? SOURCES : null,
    capable,
    why: why ?? null,
    britBin: britBin || null,
    line: mode === 'shadow' && !usable ? UNAVAILABLE_LINE : null,
  });
}

// One run's policy per process: the key is the mode and the brit's identity.
let current = null;
const probes = new Map();
let budget = { ...SHADOW_BUDGET, spentMs: 0, exhausted: false, announced: false, ended: null };

/** Reset the per-process policy, probe cache and shadow budget (tests; a gate process runs once). */
export function clearGateInputsCache({ totalMs = SHADOW_BUDGET.totalMs, callMs = SHADOW_BUDGET.callMs } = {}) {
  current = null;
  probes.clear();
  budget = { totalMs, callMs, spentMs: 0, exhausted: false, announced: false, ended: null };
}

const sameIdentity = (a, b) => Boolean(a && b && a.path === b.path && a.stamp === b.stamp);

/** The push's answer, if the state file holds one for this mode and this brit; else null. */
function readState(env, mode, identity) {
  const path = env[INPUTS_STATE_ENV];
  if (!path) return null;
  try {
    const state = JSON.parse(readFileSync(path, 'utf8'));
    if (state?.v !== 1 || state.mode !== mode || typeof state.capable !== 'boolean' || !sameIdentity(state.brit, identity)) return null;
    return state;
  } catch {
    return null;
  }
}

/** Written whole or not at all: a temporary file beside it, then a rename. A failure leaves children to probe. */
function writeState(env, state) {
  const path = env[INPUTS_STATE_ENV];
  if (!path) return;
  const tmp = `${path}.${process.pid}.tmp`;
  try {
    writeFileSync(tmp, `${JSON.stringify({ v: 1, mode: state.mode, brit: state.brit, capable: state.capable })}\n`);
    renameSync(tmp, path);
  } catch { /* the next process probes for itself */ }
}

/**
 * The run's policy: an immutable `{ mode, cls, shadow, capable, why, britBin, line }`, resolved once
 * per process for a mode and brit identity. Under pre-push the state file carries the push's answer
 * to every gate child, validated on read; its writer printed the policy's line, so a child that
 * accepts it prints nothing.
 */
export function resolveGateInputs({ env = process.env, britBin = null, spawn = spawnSync, log = stderrLog } = {}) {
  const mode = inputsMode(env);
  const identity = britIdentity(britBin);
  const key = JSON.stringify([mode, identity]);
  if (current && current.key === key) return current.policy;
  if (mode === 'off') {
    current = { key, policy: policyOf({ mode, capable: null, why: 'GATE_INPUTS=off', britBin }) };
    return current.policy;
  }
  const state = identity ? readState(env, mode, identity) : null;
  let answer;
  if (state) answer = { capable: state.capable, why: state.capable ? null : 'the push\'s probe found no --inputs' };
  else if (!identity) answer = { capable: false, why: 'no brit executable (BRIT_BIN or PATH)' };
  else if (probes.has(key)) answer = probes.get(key);
  else {
    // The probe is shadow's work: it runs inside the process's shadow budget. A spent budget is not a
    // fact about this brit, so it is neither remembered nor handed to a child.
    const probed = inShadowBudget(log, bound => probeBrit(britBin, { spawn: bound(spawn) }));
    answer = probed || { capable: false, why: 'the shadow budget was spent before the probe finished', transient: true };
    if (!answer.transient) probes.set(key, answer);
  }
  const policy = policyOf({ mode, britBin, capable: answer.capable, why: answer.why });
  if (!state) {
    if (policy.line) log(policy.line);
    if (identity && !answer.transient) writeState(env, { mode, brit: identity, capable: answer.capable });
  }
  current = { key, policy };
  return policy;
}

/** A shadow call failed for want of the flag: shadow says so once and prints nothing more this process. */
export function endShadow(reason, log = stderrLog) {
  if (budget.ended) return;
  budget.ended = reason;
  log(`[gate] inputs (shadow): brit refused --inputs (${reason}); no further inputs lines this run`);
}

/**
 * Run one piece of shadow work inside the process's shadow budget: SHADOW_BUDGET.totalMs of the
 * shadow's OWN elapsed time, summed over every call, so a gate that runs for minutes between two
 * shadow calls spends none of it. `work(bound)` gets `bound(spawn)`, a spawn that, before every call,
 * refuses once the budget is spent (an EBUDGET error, as a spawn that never ran) and otherwise passes
 * timeout = min(the caller's, the per-call cap, what remains), with SIGKILL at it. Returns work's
 * value, or null when shadow has ended or the budget is spent (said once). Never throws.
 */
export function inShadowBudget(log, work, { now = Date.now } = {}) {
  if (budget.ended) return null;
  const stop = () => {
    budget.exhausted = true;
    if (budget.announced) return;
    budget.announced = true;
    log(`[gate] inputs (shadow): stopped: the shadow's ${budget.totalMs / 1000} s budget for this run is spent; no further inputs lines`);
  };
  if (budget.exhausted || budget.spentMs >= budget.totalMs) {
    stop();
    return null;
  }
  const started = now();
  const remaining = () => budget.totalMs - budget.spentMs - (now() - started);
  const bound = spawn => (bin, args, opts = {}) => {
    const left = remaining();
    if (left <= 0) {
      budget.exhausted = true;
      return { status: null, signal: null, stdout: '', stderr: '', error: Object.assign(new Error('the shadow budget is spent'), { code: 'EBUDGET' }) };
    }
    return spawn(bin, args, { ...opts, timeout: Math.min(opts.timeout ?? budget.callMs, budget.callMs, left), killSignal: 'SIGKILL' });
  };
  let value = null;
  try {
    value = work(bound);
  } catch {
    value = null;
  }
  budget.spentMs += now() - started;
  if (budget.exhausted) {
    stop();
    return null;
  }
  return value;
}

/**
 * The first changed path that one of a step's OWN buildProcess entries names (suffix stripped,
 * matched as SOURCES subtracts it), or null. Only such a change can move a step's legacy fingerprint
 * and leave its sources one: the two classes differ by exactly those files.
 */
export function processFileTouched(inputs, changedFiles) {
  const matchers = (inputs?.buildProcess || []).map(entry => picomatch(stripFunctionSuffix(entry)));
  if (matchers.length === 0) return null;
  return changedFiles.find(file => matchers.some(m => m(file))) ?? null;
}

/** Whether any step of any manifest has a buildProcess entry the change set touches. */
export function anyProcessFileTouched(manifests, changedFiles) {
  return manifests.some(({ content }) => Object.values(content.steps || {}).some(step => processFileTouched(step.inputs, changedFiles) !== null));
}

/** The steps a gate project borrows: its `steps`, or every step when it names neither steps nor inputs. */
export function borrowedSteps(manifests) {
  const out = new Set();
  for (const { content } of manifests) {
    for (const config of Object.values(content.gate?.projects || {})) {
      const steps = config.steps || (config.inputs ? [] : Object.keys(content.steps || {}));
      for (const step of steps) out.add(`${content.pipeline}:${step}`);
    }
  }
  return out;
}

/** The shadow's per-push line: borrowed steps the legacy class selects and SOURCES would not. */
export function selectionShadowLine(manifests, legacyStale, sourcesStale, legacyProjects, sourcesProjects) {
  const borrowed = borrowedSteps(manifests);
  const steps = [...legacyStale.keys()].filter(q => borrowed.has(q) && !sourcesStale.has(q)).sort();
  if (steps.length === 0) return null;
  const kept = new Set(sourcesProjects.map(p => p.name));
  const gates = legacyProjects.map(p => p.name).filter(name => !kept.has(name));
  return `[gate] inputs (shadow): all selects and sources would not: ${steps.join(' ')}`
    + (gates.length ? `; gates sources would not run: ${gates.join(' ')}` : '; every gate is still selected');
}

// ---- CLI: `gate-inputs.mjs resolve` (pre-push, once per push) ----------------------------------

const isMain = import.meta.url === `file://${process.argv[1]}` ||
  import.meta.url === `file://${resolve(process.argv[1] || '')}`;

if (isMain) {
  const [verb] = process.argv.slice(2);
  if (verb !== 'resolve') {
    process.stderr.write('usage: gate-inputs.mjs resolve   (writes the run\'s policy to $GATE_INPUTS_STATE; the line goes to stderr)\n');
    process.exit(2);
  }
  // gate-oracle.mjs imports this module, so it is loaded once this one has finished evaluating.
  import('./gate-oracle.mjs').then(({ resolveBritBin }) => {
    resolveGateInputs({ env: process.env, britBin: resolveBritBin(process.env) });
    process.exit(0);
  }, () => process.exit(0));
}
