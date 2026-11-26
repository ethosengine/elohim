#!/usr/bin/env node
// One local gate path for humans and pre-push. Detection and execution both
// come from build-manifest.json; no project-name command switch lives here.

import { appendFileSync, mkdtempSync, readFileSync, realpathSync, rmSync, statSync, writeFileSync } from 'fs';
import { tmpdir } from 'os';
import { spawnSync } from 'child_process';
import { createHash } from 'crypto';
import { resolve, dirname, basename, join, relative } from 'path';
import { fileURLToPath } from 'url';
import { loadManifests } from './manifest-utils.mjs';
import { loadGateRegistry } from './pipeline-registry.mjs';
import { projectsFromStale, staleSteps, walkGraph } from './graph-walker.mjs';
import {
  LEGACY_ALL, SOURCES, anyProcessFileTouched, endShadow, inShadowBudget, processFileTouched, resolveGateInputs,
  selectionShadowLine,
} from './gate-inputs.mjs';
import { filterChanged } from './ci-ignore.mjs';
import { COST_FIELDS, classifyFailure, recordCycle, subjectFor } from './gate-cycle.mjs';
import { CI_BRANCH_PATTERN, costOf, deferDecision, formatExpectation, readCycleUnion } from './gate-cost.mjs';
import {
  ageOf, attestedOutputParses, deviceRoot, emitGateVerdict, findEvidence, gateEvidence, gateKeyMaterial, gateKeyMaterials, isHabitAtom, isSiblingGovernancePath, patternDigest, recordEvidence, resolveBin, reusableHit,
  selfAgentId, siblingGovernanceChange, siblingVerdicts, stepFingerprints,
} from './gate-evidence.mjs';
import { britAffected, resolveBritBin } from './gate-oracle.mjs';
import { landerWait } from './gate-carry-claim.mjs';
import { deviceEpr } from './device-epr.mjs';
import { recipeKeyShadow } from './gate-recipe-key.mjs';
import { enrichmentsIn, gradeLine, gradeMode, gradeOf, gradeStep, rawModes } from './gate-grade.mjs';
import { readBuildNote } from './scripts/brit-attest.mjs';
import { CLAIMED_EXIT_CODE, runAttested } from './gate-attest.mjs';
import { affectedPackages, loadMetadata, metadataToolchain } from './cargo-affected.mjs';
import { isReadPattern, SUITE_READ_LISTS, readSet } from './gate-input-audit.mjs';
import { crateKeys, EPRFS_RECIPE_STEP_COVERAGE } from './crate-keys.mjs';
import { appendEvent, closeRun, formatHarvest, harvest, liveStopIds, openRun, outlook, resolveSession, stopRequested } from './gate-live.mjs';
import { emitLive, formatRow, oneHopDependents, onRedPolicy, runPreflight } from './gate-preflight.mjs';
import { prepareTask, readTaskStatus, taskAnswer, taskEnabled } from './gate-task.mjs';
import { SUITE_EXPECT_ENV, SUITE_RESULT_ENV, dirtyAfterRun, encodeExpectation, readResult, suiteDecision } from './suite-inventory.mjs';

// GATE_ROOT lets a fixture repository drive the CLI (the a2o pin-attestation story).
const ROOT = process.env.GATE_ROOT
  ? resolve(process.env.GATE_ROOT)
  : resolve(dirname(fileURLToPath(import.meta.url)), '../..');

/**
 * 'rakia' since 2026-09-23: the shadow run over the pin-moving commit 691b28cdf printed
 * `[gate] oracle-diff: +elohim-storage +elohim-app` — exactly the depth-one consumers of the
 * rakia and sophia pins — and over the full push range the two oracles agreed. The habit atom
 * records the line. GATE_ORACLE=shadow|path remain available.
 */
export function oracleMode(env = process.env) {
  const declared = (env.GATE_ORACLE || '').trim();
  return ['shadow', 'rakia', 'path'].includes(declared) ? declared : 'rakia';
}

function nameSet(projects) {
  return new Set(projects.map(p => p.name));
}

function shortDid(did) {
  const text = String(did || 'unknown');
  return text.length > 24 ? `${text.slice(0, 16)}…${text.slice(-4)}` : text;
}

function oracleDiffLine(pathProjects, oracleProjects) {
  const a = nameSet(pathProjects);
  const b = nameSet(oracleProjects);
  const plus = [...b].filter(n => !a.has(n));
  const minus = [...a].filter(n => !b.has(n));
  if (plus.length === 0 && minus.length === 0) return null;
  const parts = [...plus.map(n => `+${n}`), ...minus.map(n => `-${n}`)];
  return `[gate] oracle-diff: ${parts.join(' ')}`;
}

export function selectGateProjects(registry, target) {
  if (registry.has(target)) return [registry.get(target)];

  const normalized = target.replace(/^\.\//, '').replace(/\/$/, '');
  const matches = [...registry.values()]
    .filter(project => normalized === project.dir || normalized.startsWith(`${project.dir}/`))
    .sort((a, b) => b.dir.length - a.dir.length);
  if (matches.length === 0) throw new Error(`Unknown gate project or path: ${target}`);
  const longest = matches[0].dir.length;
  return matches.filter(project => project.dir.length === longest);
}

/**
 * A target a person names runs every gate project with the same dir, steps and inputs as it: the
 * orchestrator suite's two halves (E11), so `just gate orchestrator` runs the whole suite. Pre-push
 * runs each selected project by name and `--recipe` takes exactly one, so neither is expanded.
 */
export function withHalves(registry, projects, { env = process.env } = {}) {
  if (env.GATE_RUN_KIND === 'pre-push' || projects.length !== 1) return projects;
  const [one] = projects;
  const shape = p => JSON.stringify([p.dir, p.steps ?? null, p.inputs ?? null]);
  return [one, ...[...registry.values()].filter(p => p.name !== one.name && shape(p) === shape(one))];
}

/**
 * The gate projects a change set selects. The input class of every borrowed step comes from the
 * run's one policy (gate-inputs.mjs): `opts.inputs` pins a class (reuseSelector does), `opts.policy`
 * hands one in, and otherwise it is resolved here. In shadow, with a capable brit, one line says
 * which borrowed steps the legacy class selects and sources would not, asked only when a changed
 * path is some step's own buildProcess file and bounded by shadow's budget; it changes nothing
 * chosen (`announce: false` keeps a reuse selector quiet). The policy's own lines go to stderr,
 * apart from `log`, the oracle's diagnostics.
 */
export function projectsForChanges(root, changedFiles, opts = {}) {
  const env = opts.env || process.env;
  const mode = opts.oracle || oracleMode(env);
  // Diagnostics go to STDERR: the pre-push hook parses this command's stdout as
  // project names (`--names`), so a line on stdout becomes a bogus gate target.
  const log = opts.log || (line => process.stderr.write(`${line}\n`));
  const inputsLog = opts.inputsLog || (line => process.stderr.write(`${line}\n`));
  const spawn = opts.spawn || spawnSync;
  const manifests = loadManifests(root);
  const registry = loadGateRegistry(root);
  const files = filterChanged(changedFiles);
  const policy = opts.inputs ? null
    : (opts.policy || resolveGateInputs({ env, britBin: resolveBritBin(env), spawn, log: inputsLog }));

  const select = (cls, run = spawn) => {
    const byPath = walkGraph(manifests, files, { gateInputs: cls }).projects;
    if (mode === 'path') return { cls, byPath, stale: null, reason: null, refused: false };
    let reason = 'no reason reported';
    let refused = false;
    const ask = opts.ask
      ? () => opts.ask(cls)
      : () => britAffected(root, files, {
        britBin: resolveBritBin(env), spawn: run, inputs: cls,
        onUnavailable: (r, info) => { reason = r; refused = Boolean(info?.refusedFlag); },
      });
    return { cls, byPath, stale: ask(), reason, refused };
  };
  const selected = select(opts.inputs || policy.cls);

  let chosen = selected.byPath;
  const oracleAvailable = mode !== 'path' && selected.stale !== null;
  const reason = selected.reason || 'no reason reported';
  if (mode !== 'path') {
    if (selected.stale === null) {
      // Said in both modes: in shadow mode an absent binary would otherwise mean the
      // flip evidence silently never arrives.
      log(`[gate] brit build affected unavailable (${reason}) — path-only selection`);
      // The path walk does not follow dependencies. Name the projects one hop downstream that it
      // therefore lacks, so whoever carries or lands this change knows; nothing is added to the
      // selection, and a walk that cannot be made says nothing more.
      try {
        const lacking = (opts.dependents || oneHopDependents)(root, files).map(project => project.name)
          .filter(name => !chosen.some(project => project.name === name));
        if (lacking.length) log(`[gate] path-only selection may be missing ${lacking.join(' ')}: one hop downstream of the change, selected only by the graph oracle — not selected here`);
      } catch { /* the fallback line above stands alone */ }
    } else {
      const byOracle = projectsFromStale(manifests, selected.stale, files);
      if (mode === 'rakia') {
        chosen = byOracle;
      } else {
        const diff = oracleDiffLine(selected.byPath, byOracle);
        if (diff) log(diff);
      }
    }
  }
  // Keep the array contract; reuse callers also need to know whether selection fell back.
  opts.onOracle?.(oracleAvailable, oracleAvailable ? null : reason);

  // Shadow: the same question under SOURCES, inside its budget; it changes nothing chosen above.
  // The classes differ by exactly the files a step lists as its own buildProcess. When no changed path
  // is one, no step is directly stale under one class and not the other, and so no step's one-hop
  // upstream reason can differ either (an upstream reason needs a directly stale step, under the
  // same class). That holds for every step, not only borrowed ones, because a step a gate does not
  // borrow can still select a borrowed dependent by upstream; so the check spans every manifest's
  // steps, and when it finds nothing the second `brit build affected` is not asked.
  if (policy?.shadow && opts.announce !== false && anyProcessFileTouched(manifests, files)) {
    inShadowBudget(inputsLog, bound => {
      const shadow = select(policy.shadow, bound(spawn));
      if (shadow.refused) {
        endShadow(shadow.reason, inputsLog);
        return;
      }
      const byOracle = mode === 'rakia' && selected.stale !== null && shadow.stale !== null;
      const legacyStale = byOracle ? selected.stale : staleSteps(manifests, files, LEGACY_ALL);
      const sourcesStale = byOracle ? shadow.stale : staleSteps(manifests, files, policy.shadow);
      const sourcesProjects = byOracle ? projectsFromStale(manifests, shadow.stale, files) : shadow.byPath;
      const line = selectionShadowLine(manifests, legacyStale, sourcesStale, chosen, sourcesProjects);
      if (line) inputsLog(line);
    });
  }

  return chosen.map(project => {
    const registered = registry.get(project.name);
    if (!registered) throw new Error(`Detected unregistered gate project: ${project.name}`);
    return { ...registered, reasons: project.reasons };
  });
}

/**
 * The one reuse decision, for the Task path and the direct path alike. The sibling channel is
 * asked first, so a red from either device for the same closure runs the gate before this
 * workspace's own green is accepted; a sibling green stands in only when the push leaves the
 * sibling-reuse governance paths alone.
 */
export function decideReuse(project, evidence, reuseContext, { root = ROOT, env = process.env, spawn = spawnSync, observe = null } = {}) {
  let candidates = [];
  let governance = { changed: false };
  if (reuseContext.mode === 'reuse' && reuseContext.enabled && reuseContext.selfId && evidence.key) {
    const read = siblingVerdicts(project, evidence.key, { root, env, spawn });
    candidates = read.candidates;
    // GATE_GRADE's shadow reads the same answer instead of asking the flow store a second time.
    // The observer is handed a copy after the read and can change nothing decided below.
    if (observe) {
      try { observe({ candidates: [...candidates], error: read.error || null }); } catch { /* a shadow is never a red */ }
    }
    if (candidates.some(candidate => candidate.success === true)) governance = siblingGovernanceChange({ root, spawn });
  }
  const decision = reusableHit(evidence, {
    ...reuseContext,
    candidates,
    // Admission is established by `epr peer verdicts`; a sibling red always counts, a sibling green
    // only when governance did not change.
    admitCandidate: candidate => candidate.source === 'sibling' && (candidate.success !== true || !governance.changed),
    governanceWhy: governance.changed ? governance.why : null,
  });
  if (!decision.reuse || decision.independence === 'sibling') return decision;
  // Just before this workspace's green is accepted: its attested output parses as a CID, as ark
  // required, and a fresh read of this closure's rows holds no newer local red.
  if (!attestedOutputParses(decision.row)) return { reuse: false, why: 'the attested output is not a CID' };
  const fresh = findEvidence(project, evidence.key, { buildRefBin: evidence.buildRefBin, root, spawn, warn: () => {} });
  if (fresh.error) return { reuse: false, why: `rows unreadable just before reuse (${fresh.error})` };
  if (fresh.unverifiable.length > 0) return { reuse: false, why: 'an unverifiable note arrived under this step' };
  const at = Date.parse(decision.row.attestation?.builtAt || 0);
  if (fresh.rows.some(row => row.attestation?.success === false && Date.parse(row.attestation?.builtAt || 0) > at)) {
    return { reuse: false, why: 'a newer local red for this closure arrived' };
  }
  return decision;
}

/**
 * Reuse needs the dependency oracle for changes outside the attested closure too. `inputs` is the
 * input class the key being reused was computed under (gate-inputs.mjs), so selection and key agree;
 * omitted, the run's policy decides.
 */
export function reuseSelector(project, gateCommit, { root = ROOT, env = process.env, spawn = spawnSync, inputs = null } = {}) {
  return commit => {
    let diff;
    try {
      if (!commit) return { ok: false, message: 'attested commit unreadable' };
      diff = spawn('git', ['diff', '--name-only', commit, gateCommit], { cwd: root, encoding: 'utf8' });
      if (diff.status !== 0) return { ok: false, message: 'attested commit unreadable' };
    } catch {
      return { ok: false, message: 'attested commit unreadable' };
    }
    // A habit atom is a ledger no recipe reads (gate-evidence.mjs isHabitAtom). Only a suite list's
    // named half provably keys without them (its inventory drops them and it has no step
    // fingerprint), so only there is a DELTA line no change since the attested commit.
    const exempt = Boolean(SUITE_READ_LISTS[project.name]);
    const files = diff.stdout.split('\n').filter(Boolean).filter(file => !(exempt && isHabitAtom(file)));
    if (files.length === 0) return { ok: true };
    try {
      let oracleAvailable = false;
      let why = '';
      const projects = projectsForChanges(root, files, {
        env, spawn, oracle: 'rakia', announce: false, ...(inputs ? { inputs } : {}),
        onOracle: (available, reason) => { oracleAvailable = available; why = reason || ''; },
      });
      if (!oracleAvailable) return { ok: false, message: `graph oracle unavailable${why ? ` (${why})` : ''}; reuse needs it` };
      if (projects.some(p => p.name === project.name)) {
        return { ok: false, message: `changed since attested commit and selects this gate: ${files[0]}` };
      }
      return { ok: true };
    } catch {
      return { ok: false, message: 'graph oracle unavailable; reuse needs it' };
    }
  };
}

// One integration base and one changed-path list per gated commit, per process.
const shadowRanges = new Map();

/**
 * The borrowed steps of `project` whose OWN buildProcess file changed between the integration base
 * and `gateCommit` ({ base, steps }), read with git alone and no brit spawn. Only these steps can
 * have a fingerprint that moved under the legacy class and not under sources. `gateCommit` is the
 * full sha runProject resolved; `spawn` is shadow's bounded spawn, so these reads are inside its
 * budget and killed at their timeout. Per gated commit and process: `git merge-base <commit>
 * origin/dev` (then `dev` only if that fails), and, only when the base is not the gated commit, one
 * `git diff --name-only --no-renames -z`, NUL-split, so a renamed or deleted file shows under its old
 * name and no path is quoted. No base, or a base that is the gated commit, is no step. A read the
 * budget refused is not remembered.
 */
export function pushRange({ root, gateCommit, spawn = spawnSync }) {
  const key = `${resolve(root)}\0${gateCommit}`;
  if (!shadowRanges.has(key)) {
    let range = { base: null, files: [] };
    let refused = false;
    for (const ref of ['origin/dev', 'dev']) {
      const r = spawn('git', ['merge-base', gateCommit, ref], { cwd: root, encoding: 'utf8' });
      if (r?.error?.code === 'EBUDGET') { refused = true; break; }
      const base = r?.status === 0 ? String(r.stdout || '').trim() : '';
      if (!base) continue;
      if (base === gateCommit) { range = { base, files: [] }; break; }
      const diff = spawn('git', ['diff', '--name-only', '--no-renames', '-z', base, gateCommit], { cwd: root, encoding: 'utf8' });
      if (diff?.error?.code === 'EBUDGET') { refused = true; break; }
      range = { base, files: diff?.status === 0 ? String(diff.stdout || '').split('\0').filter(Boolean) : [] };
      break;
    }
    if (refused) return null;
    shadowRanges.set(key, range);
  }
  return shadowRanges.get(key);
}

export function touchedProcessSteps(project, { root, gateCommit, spawn = spawnSync }) {
  const range = pushRange({ root, gateCommit, spawn });
  if (!range) return { base: null, steps: [] };
  const { base, files } = range;
  if (!base || files.length === 0 || !project.steps?.length) return { base, steps: [] };
  let manifest;
  try { manifest = JSON.parse(readFileSync(resolve(root, project.manifestPath), 'utf8')); } catch { return { base, steps: [] }; }
  return { base, steps: project.steps.filter(step => processFileTouched(manifest.steps?.[step]?.inputs, files) !== null) };
}

/**
 * GATE_INPUTS=shadow's per-gate line: since the integration base, the step fingerprints of the
 * touched steps (touchedProcessSteps) moved under the legacy class and are unchanged under sources.
 * It says nothing about the rest of the key (manifest, recipe, project tree, audited reads). Every
 * subprocess goes through `run`, shadow's bounded spawn. `{ lines }`, or `{ refused }` when brit
 * refused the flag after all. Changes nothing the decision, the key or the record reads.
 */
function inputsShadow(project, evidence, { root, gateCommit, base, steps, run }) {
  const touched = { ...project, steps };
  const fingerprints = (cls, commit) => stepFingerprints(touched, commit, { britBin: evidence.britBin, root, spawn: run, inputs: cls });
  const sourcesNow = fingerprints(SOURCES, gateCommit);
  if (sourcesNow.refusedFlag) return { refused: sourcesNow.error };
  if (sourcesNow.error) return { lines: [] };
  const allBase = fingerprints(LEGACY_ALL, base);
  const sourcesBase = fingerprints(SOURCES, base);
  if (sourcesBase.refusedFlag) return { refused: sourcesBase.error };
  if (allBase.error || sourcesBase.error) return { lines: [] };
  const moved = (before, after) => after.filter(row => before.find(b => b.step === row.step)?.fingerprint !== row.fingerprint).map(row => row.step);
  const allMoved = moved(allBase.fingerprints, evidence.fingerprints.filter(row => steps.includes(row.step)));
  const sourcesMoved = moved(sourcesBase.fingerprints, sourcesNow.fingerprints);
  return allMoved.length > 0 && sourcesMoved.length === 0
    ? { lines: [`[gate] ${project.name} inputs (shadow): since ${base.slice(0, 9)} the step fingerprints moved under all (${allMoved.join(' ')}) and are unchanged under sources`] }
    : { lines: [] };
}

const READ_SHADOW_VARIANTS = Object.freeze([
  { name: 'real', label: 'real', variant: {} },
  { name: 'fix1', label: 'fix 1', variant: { excludeDependencyTests: true } },
  { name: 'fix2', label: 'fix 2', variant: { resolveFormatJoin: true } },
  { name: 'both', label: 'both', variant: { excludeDependencyTests: true, resolveFormatJoin: true } },
]);

/**
 * Compare closure movement under the two proposed read-set fixes. This returns telemetry only.
 * Neither fix may decide a gate until the proof covers: dependency tests/** support reached through
 * #[path], format! joins with variable suffixes, absent resolved paths, and eprfs actually running
 * and blocking on the same push whose closure the shadow compares.
 */
export function readShadowComparison(project, evidence, {
  root, base, commit, env, cargoEnv, spawn, inputs = null,
  clock = () => process.hrtime.bigint(), reportTimings = false, check = null,
} = {}) {
  try {
    const material = JSON.parse(evidence.material);
    if (material.reads?.scope === 'tree' && Object.hasOwn(material, 'native')) {
      return {
        line: `[gate] ${project.name} read-shadow: no reading (tree-keyed gate: its closure is the checkout, not a commit)`,
        rows: [],
      };
    }
  } catch { /* canonical material validation below names malformed evidence */ }
  const shared = {};
  const options = {
    root, env, cargoEnv, spawn, inputs, britBin: evidence.britBin,
    boundedSuite: true, variants: READ_SHADOW_VARIANTS, clock, shared, suiteCheck: check,
  };
  const beforeBatch = gateKeyMaterials(project, { ...options, commit: base });
  const afterBatch = gateKeyMaterials(project, { ...options, commit });
  const results = READ_SHADOW_VARIANTS.map((entry, index) => {
    const before = beforeBatch.results[index].closure;
    const after = afterBatch.results[index].closure;
    if (!before.key || !after.key) {
      throw new Error(oneLine(after.line || before.line || `${entry.label} key unavailable`));
    }
    return { ...entry, key: after.key, moved: before.key !== after.key };
  });
  const real = results[0];
  if (real.key !== evidence.key) throw new Error('real variant differs from the gate evidence key');
  const differs = results.slice(1).some(result => result.moved !== real.moved);
  if (!differs && !reportTimings) return null;
  const movement = differs ? results.map(result => {
    if (result.moved) return `${result.label} moved`;
    if (result.name === 'real') return 'real held';
    const proof = result.name === 'both' ? 'fixes 1 and 2 were sound' : `${result.label} were sound`;
    return `${result.label} would have held IF ${proof}`;
  }).join('; ') : `real and all variants ${real.moved ? 'moved' : 'held'} alike`;
  const sum = (field, name = null) => beforeBatch.timing[field][name] + afterBatch.timing[field][name];
  const timing = {
    totalMs: beforeBatch.timing.totalMs + afterBatch.timing.totalMs,
    readSetAuditSharedMs: beforeBatch.timing.readSetAuditSharedMs + afterBatch.timing.readSetAuditSharedMs,
    readSetAuditMs: Object.fromEntries(READ_SHADOW_VARIANTS.map(({ name }) => [name, sum('readSetAuditMs', name)])),
    fingerprintingMs: beforeBatch.timing.fingerprintingMs + afterBatch.timing.fingerprintingMs,
    suiteInventoryMs: beforeBatch.timing.suiteInventoryMs + afterBatch.timing.suiteInventoryMs,
    keyCompositionMs: Object.fromEntries(READ_SHADOW_VARIANTS.map(({ name }) => [name, sum('keyCompositionMs', name)])),
  };
  const ms = value => Math.max(0, Math.round(value));
  const audits = READ_SHADOW_VARIANTS.map(({ name, label }) => `${label} ${ms(timing.readSetAuditMs[name])}`).join(', ');
  const compositions = Object.values(timing.keyCompositionMs).reduce((a, b) => a + b, 0);
  const timingLine = `timing ${ms(timing.totalMs)} ms (read-set audit ${audits}, shared ${ms(timing.readSetAuditSharedMs)}; fingerprinting ${ms(timing.fingerprintingMs)}; suite inventory ${ms(timing.suiteInventoryMs)}; key composition ${ms(compositions)})`;
  return {
    line: `[gate] ${project.name} read-shadow: ${movement}; ${timingLine} — decides nothing; enablement requires proof for #[path]-included tests/** support, format! joins with variable suffixes, absent paths, and eprfs running and blocking on this same push`,
    rows: results.slice(1).map(result => ({
      name: result.name,
      key: result.key,
      keyChanged: result.moved
        ? 'moved'
        : `would have held IF ${result.name === 'both' ? 'fixes 1 and 2 were sound' : `${result.label} were sound`}`,
      timing: {
        totalMs: timing.totalMs,
        readSetAuditMs: timing.readSetAuditMs[result.name],
        readSetAuditSharedMs: timing.readSetAuditSharedMs,
        fingerprintingMs: timing.fingerprintingMs,
        suiteInventoryMs: timing.suiteInventoryMs,
        keyCompositionMs: timing.keyCompositionMs[result.name],
      },
    })),
  };
}

// The rakia-validated manifest schema accepts `run.cargo.env` since rakia 2b2cedb
// (2026-09-23); a project's cap declares on its own manifest (elohim-storage does).
// genesis/agentic/pool-policy.json's `cargo_env_overrides` remains for projects that
// have not moved theirs yet. Read once per call; a missing/malformed file is not
// fatal to the gate.
function loadPoolPolicy(root) {
  try {
    return JSON.parse(readFileSync(resolve(root, 'genesis/agentic/pool-policy.json'), 'utf8')) || {};
  } catch {
    return {};
  }
}

// `run.cargo.env` travels to run-local-gate.sh as ONE serialized variable, never
// through argv: the positional contract is exactly four cargo args (workspace,
// targetDir, profile, rustflags) and every caller of run-local-gate.sh depends on
// that arity. The script parses this and exports each pair before `just`.
//
// Merge rule: the manifest's own `run.cargo.env` is the project's contract and
// always wins; `pool-policy.cargo_env_overrides[project.name]` only fills keys the
// manifest left undeclared, and `cargo_env_overrides["*"]` fills what both left.
// run-local-gate.sh exports declared values over the caller's environment. This lets pool-policy carry today's cap (schema-pinned
// out of the manifest) without silently overriding a project that later declares
// its own value once the rakia schema is widened.
//
// Declared targets (row 29.2, backlog steward-device-arrival-open-items.md row 29):
// pool-policy's top-level `declared_targets` names, per Rust target triple, the C
// baseline a cargo workspace's cc-built C/C++ is compiled at. A project whose
// run.cargo.workspace is listed receives CFLAGS_<triple> and CXXFLAGS_<triple> (dashes
// as underscores, the spelling cc reads) set to the target's `cflags`, and each variable of
// the target's `env` map (CC_ENABLE_DEBUG_OUTPUT=1: cc's command log, the evidence `epr
// artifact put` reads before it marks a release env:isa-c; turning it on costs each device one
// cold C rebuild and the gate one derivation-key change, by cc's rerun-if-env-changed). The flag keys are
// triple-suffixed, so a host building another target never reads them. Precedence: the
// record's value is KEPT over a manifest run.cargo.env or a cargo_env_overrides entry
// that sets the same key, and the disagreement is printed (once per project and key),
// never dropped silently: the baseline is a property of a binary every sibling runs, not
// a per-project tuning knob. The gate's outcome is not changed by the conflict.
const warnedTargetConflicts = new Set();

function declaredTargetEnv(project, policy) {
  const workspace = (project.run.cargo || {}).workspace;
  const env = {};
  if (!workspace) return env;
  for (const [triple, target] of Object.entries(policy.declared_targets || {})) {
    if (triple.startsWith('_') || !target || typeof target.cflags !== 'string') continue;
    if (!Array.isArray(target.workspaces) || !target.workspaces.includes(workspace)) continue;
    const suffix = triple.replace(/-/g, '_');
    env[`CFLAGS_${suffix}`] = target.cflags;
    env[`CXXFLAGS_${suffix}`] = target.cflags;
    // The target's declared `env` (E12 L2a rework, ruling 1): CC_ENABLE_DEBUG_OUTPUT=1, so cc logs
    // each compile command into its build script's output, the evidence put reads before it
    // stamps env:isa-c. Only string values are exported; the same precedence as the flags.
    // Expected once: cc emits cargo:rerun-if-env-changed=CC_ENABLE_DEBUG_OUTPUT, so the export
    // reruns every cc build script once on each device (one cold C rebuild per device) and changes
    // the gate's derivation-key inputs once; neither is a regression.
    const declaredEnv = target.env && typeof target.env === 'object' && !Array.isArray(target.env) ? target.env : {};
    for (const [key, value] of Object.entries(declaredEnv)) {
      if (typeof value === 'string') env[key] = value;
    }
  }
  return env;
}

function defaultTargetWarn(line) {
  if (warnedTargetConflicts.has(line)) return;
  warnedTargetConflicts.add(line);
  console.error(line);
}

export function gateChildEnv(project, baseEnv, root = ROOT, { warn = defaultTargetWarn } = {}) {
  const policy = loadPoolPolicy(root);
  const overrides = policy.cargo_env_overrides || {};
  // "*" is the all-cargo-projects default (the local gate's RUSTUP_TOOLCHAIN pin); only a
  // project that goes through the cargo path (declares run.cargo) receives it.
  const shared = project.run.cargo ? overrides['*'] || {} : {};
  const merged = { ...shared, ...(overrides[project.name] || {}), ...((project.run.cargo || {}).env || {}) };
  const targetEnv = declaredTargetEnv(project, policy);
  for (const [key, value] of Object.entries(targetEnv)) {
    if (Object.hasOwn(merged, key) && String(merged[key]) !== value) {
      warn(`gate ${project.name}: ${key}=${merged[key]} (manifest run.cargo.env or pool-policy cargo_env_overrides) `
        + `conflicts with pool-policy declared_targets; keeping the declared ${key}=${value}`);
    }
  }
  const declared = { ...merged, ...targetEnv };
  const childEnv = { ...baseEnv };
  // A git hook exports GIT_DIR (the pre-push pins it absolute so gate cwds resolve the repo);
  // a gate that spawns `git` in a TEMP repo then operates on the main repo instead
  // (eprfs flow_acceptance: "pathspec 'scope.md' did not match", 11 tests, 2026-09-11).
  // Gates run inside the checkout, so git discovers it from cwd — drop the hook's pins.
  for (const k of ['GIT_DIR', 'GIT_WORK_TREE', 'GIT_INDEX_FILE', 'GIT_PREFIX']) delete childEnv[k];
  // `git -c key=value push` reaches every child git through GIT_CONFIG_PARAMETERS (and
  // GIT_CONFIG_COUNT/KEY_n/VALUE_n): 2026-11-01, a push with `-c core.hooksPath=…` ran the pre-push
  // hook inside build-compute-worker's fixture repo, whose own `git push` then failed. Config scoped
  // to the push never applies to the temp repositories a gate creates.
  for (const k of Object.keys(childEnv)) {
    if (k === 'GIT_CONFIG_PARAMETERS' || k === 'GIT_CONFIG_COUNT' || /^GIT_CONFIG_(KEY|VALUE)_\d+$/.test(k)) delete childEnv[k];
  }
  // A caller's JUST_NO_DEPS or JUST_DRY_RUN (any JUST_* setting) would turn a gate recipe into a
  // no-op that exits 0; the recipe is the manifest's, never the caller's.
  for (const k of Object.keys(childEnv)) if (k.startsWith('JUST_')) delete childEnv[k];
  if (declared && Object.keys(declared).length > 0) {
    childEnv.GATE_CARGO_ENV = JSON.stringify(declared);
  } else {
    delete childEnv.GATE_CARGO_ENV;
  }
  return childEnv;
}

// A manifest's fixed `cargo.targetDir` (e.g. eprfs's /tmp/eprfs-gate-target) is the main
// checkout's: hooks read the `epr` it builds by that exact path. Linked worktrees get their OWN
// suffixed dir, so concurrent gates in two worktrees never rebuild the same binary under each
// other (a sponsorship gate once tested a sibling worktree's `epr` mid-run, 2026-09-25).
export function worktreeTargetDir(targetDir, root = ROOT, git = gitDirs) {
  if (!targetDir) return targetDir;
  const dirs = git(root);
  if (!dirs || dirs.gitDir === dirs.commonDir) return targetDir;
  return `${targetDir}-wt-${basename(root)}`;
}

function gitDirs(root) {
  const read = arg => {
    const r = spawnSync('git', ['rev-parse', '--path-format=absolute', arg], { cwd: root, encoding: 'utf8' });
    return r.status === 0 ? r.stdout.trim() : null;
  };
  const gitDir = read('--git-dir');
  const commonDir = read('--git-common-dir');
  return gitDir && commonDir ? { gitDir, commonDir } : null;
}

/** The stable lock sits beside the slot, so deleting the slot cannot replace the locked inode. */
export function slotGateLockPath(slot) {
  if (!slot) return '';
  let backing;
  try {
    backing = realpathSync(slot);
  } catch {
    const result = spawnSync('readlink', ['-m', '--', slot], { encoding: 'utf8' });
    backing = result.status === 0 && String(result.stdout || '').trim()
      ? String(result.stdout).trim()
      : resolve(slot);
  }
  return `${backing}.gate.lock`;
}

function cargoSlotPath(cargo, targetDir, root, env) {
  if (targetDir) return targetDir;
  if (!cargo.workspace) return '';
  const result = spawnSync('bash', [
    '-c',
    'source "$1/genesis/agentic/bin/pool-lib.sh" && slot_path "$(detect_family "$1")" "$2" "$3"',
    '_', root, cargo.workspace, cargo.profile || 'dev',
  ], { cwd: root, encoding: 'utf8', env });
  return result.status === 0 ? String(result.stdout || '').trim() : '';
}

// Findings belong to this device's shared ledger, while recipes and measures
// still come from the checkout being gated. Non-git fixtures remain local.
export function architectureLedgerPath(root, git = gitDirs) {
  const dirs = git(root);
  const main = dirs && dirs.gitDir !== dirs.commonDir ? dirname(dirs.commonDir) : root;
  return resolve(main, '.claude/data/architecture-findings.jsonl');
}

// One snapshot per repository per process, including unreadable sidecars.
const cycleRowsByRoot = new Map();

export function costLineAndDefer(project, { rows, subjectFallback = null, env = {} }) {
  const cost = costOf(rows, { project, subjectFallback });
  const decision = env.GATE_RUN_KIND !== 'pre-push'
    ? { defer: false, why: 'only pre-push runs defer' }
    : !(project.steps?.length > 0)
      ? { defer: false, why: 'project has no CI pipeline steps' }
      : deferDecision({
        cost,
        targets: (env.GATE_PUSH_TARGETS || '').split(/\s+/).filter(Boolean),
        branches: (env.GATE_PUSH_BRANCHES || '').split(/\s+/).filter(Boolean),
        ciBranchPattern: CI_BRANCH_PATTERN,
        env,
      });
  return { line: formatExpectation(project.name, cost), ...decision };
}

// `recipe` runs another recipe of the project in the gate's exact cargo slot and env, with no
// evidence or cycle record: evidence carries a test verdict across commits, never the bytes, so
// pre-push uses this to build what it lands (eprfs's `_eprfs-deliver`).
const TAP = resolve(dirname(fileURLToPath(import.meta.url)), 'gate-tap.mjs');
/** A gate stopped on request exits non-zero, so whatever runs it (the hook, `just gate`) refuses. */
export const GATE_STOPPED_EXIT = 1;
/** A gate that could not acquire its shared Cargo slot did not run and has no verdict. */
// Not 75: gate-task.mjs EXIT_NOT_CARRIED=75 means ark did not carry the Task and the gate runs direct; a
// slot-busy recipe under ark must not read as that and be re-run.
export const GATE_SLOT_BUSY_EXIT = 73;

/**
 * Run a gate command through gate-tap.mjs, which forwards its bytes unchanged, keeps its
 * exit status, appends failure lines to the live run (GATE_LIVE_RUN) as they print, and
 * returns the classifyFailure signals on fd 3. `tap: false` (or GATE_TAP=off) runs the
 * command directly, as an injected spawn in a fixture does.
 */
function gateSpawn(project, command, argv, options, { spawn, env, tap }) {
  // A cargo-slot holder is owned by the tap even when live failure tapping is disabled. Bypassing
  // it would run the recipe without the lock. Injected-spawn fixtures (`tap: false`) remain direct.
  if (!tap || (env.GATE_TAP === 'off' && !options.env?.GATE_SLOT_LOCK_PATH)) {
    return { result: spawn(command, argv, options), tapped: null };
  }
  const live = env.GATE_LIVE_RUN ? ['--run', env.GATE_LIVE_RUN] : [];
  const result = spawn(process.execPath, [TAP, '--project', project.name, '--dir', project.dir, ...live, '--', command, ...argv],
    { ...options, stdio: ['inherit', 'inherit', 'inherit', 'pipe'] });
  let tapped = null;
  try { tapped = JSON.parse(String(result.output?.[3] || '').trim().split('\n').pop()); } catch { /* no summary */ }
  return { result, tapped };
}

/** Lifecycle events for the live run; fail-open, and absent when no run is open. */
function liveEvent(env, event) {
  if (env.GATE_LIVE_RUN) appendEvent(env.GATE_LIVE_RUN, event);
}

export const GATE_READING_MEASURE = 'gate-step-reading@1';
const invocationGateReadings = [];

function finiteSeconds(value) {
  const number = Number(value);
  return Number.isFinite(number) && number > 0 ? number : 0;
}

function oneLine(value) {
  return String(value ?? '').replace(/[\r\n]+/g, ' ').trim();
}

function slotWaitResult(path) {
  if (!path) return { measured: false, seconds: 0, holder: 'holder details unavailable' };
  try {
    const line = readFileSync(path, 'utf8').split('\n')[0];
    const at = line.indexOf('\t');
    const seconds = Number(at < 0 ? line : line.slice(0, at));
    if (!Number.isFinite(seconds) || seconds < 0) throw new Error('invalid slot wait seconds');
    return {
      measured: true,
      seconds,
      holder: oneLine(at < 0 ? '' : line.slice(at + 1)) || 'holder details unavailable',
    };
  } catch {
    return { measured: false, seconds: 0, holder: 'holder details unavailable' };
  }
}

class ShadowBudgetExhausted extends Error {
  constructor(budgetMs, phase) {
    super(`budget ${budgetMs} ms exhausted in ${phase}`);
    this.name = 'ShadowBudgetExhausted';
  }
}

function shadowBudgetMs(value) {
  const declared = String(value ?? '').trim();
  const configured = declared ? Number(declared) : Number.NaN;
  return Number.isFinite(configured) && configured >= 0 ? Math.floor(configured) : 15_000;
}

function crateShadowBudget(env, clock, initialSpent = 0) {
  const declared = String(env.GATE_CRATE_SHADOW_BUDGET_MS ?? '').trim();
  const budgetMs = shadowBudgetMs(declared);
  // The budget is spent only inside the shadow's own spans (`span`): discovery before the recipe
  // and the crate keys after it. The recipe between them never counts, so a long gate keeps its
  // crate rows; only shadow work beyond the budget drops them.
  let spent = initialSpent;
  let spanStart = null;
  let depth = 0;
  let exhausted = null;
  const now = () => Number(clock()) / 1e6;
  const used = () => spent + (spanStart === null ? 0 : now() - spanStart);
  const remaining = phase => {
    if (exhausted) throw exhausted;
    const left = budgetMs - used();
    if (left <= 0) {
      exhausted = new ShadowBudgetExhausted(budgetMs, phase);
      throw exhausted;
    }
    return left;
  };
  return {
    budgetMs,
    check: remaining,
    begin() {
      if (depth++ === 0) spanStart = now();
    },
    end() {
      if (depth === 0) return;
      if (--depth === 0) {
        spent += now() - spanStart;
        spanStart = null;
      }
    },
    spentMs: used,
    exhaust(phase) {
      spent = Math.max(spent, budgetMs);
      exhausted ||= new ShadowBudgetExhausted(budgetMs, phase);
    },
    spawn(run, phase, command, args, options = {}) {
      const left = remaining(phase);
      // spawnSync rejects a fractional timeout; a whole millisecond, at least 1, is always valid.
      const timeout = Math.max(1, Math.floor(Math.min(options.timeout ?? Infinity, left)));
      const result = run(command, args, { ...options, timeout });
      remaining(phase);
      return result;
    },
  };
}

function readShadowLedgerSpent(path, budgetMs, readFile = readFileSync) {
  try {
    const text = readFile(path, 'utf8');
    if (text === '') return 0;
    const body = text.endsWith('\n') ? text.slice(0, -1) : text;
    const rows = body.split('\n');
    if (rows.some(row => !/^(0|[1-9][0-9]*)$/.test(row))) return budgetMs;
    const spent = rows.reduce((sum, row) => sum + Number(row), 0);
    return Number.isSafeInteger(spent) ? spent : budgetMs;
  } catch {
    return budgetMs;
  }
}

/**
 * One budget for the shadows that run after a gate is recorded (the read-shadow and GATE_GRADE),
 * shared by this process, and by a hook push when its ledger is set. With both knobs off there is
 * no budget and no ledger I/O.
 */
export function readShadowPushBudget(env, clock, {
  readFile = readFileSync, appendFile = appendFileSync, removeFile = rmSync,
} = {}) {
  if (env.GATE_READ_SHADOW === 'off' && gradeMode(env) === 'off') return null;
  const budgetMs = shadowBudgetMs(env.GATE_READ_SHADOW_BUDGET_MS);
  const hasLedger = Object.hasOwn(env, 'GATE_READ_SHADOW_LEDGER');
  const ledger = hasLedger ? String(env.GATE_READ_SHADOW_LEDGER) : null;
  const now = () => Number(clock()) / 1e6;
  let budget = null;
  let accountingStarted = null;
  const startAccounting = () => { accountingStarted ??= now(); };
  const current = () => {
    if (budget) return budget;
    startAccounting();
    const prior = hasLedger ? readShadowLedgerSpent(ledger, budgetMs, readFile) : 0;
    const ledgerReadMs = hasLedger ? Math.max(0, now() - accountingStarted) : 0;
    budget = crateShadowBudget(
      { GATE_CRATE_SHADOW_BUDGET_MS: budgetMs }, clock, prior + ledgerReadMs,
    );
    return budget;
  };
  return {
    budgetMs,
    check(phase) {
      startAccounting();
      return current().check(phase);
    },
    begin() {
      startAccounting();
      current().begin();
    },
    end() { current().end(); },
    spentMs() { return current().spentMs(); },
    spawn(run, phase, command, args, options = {}) {
      return current().spawn(run, phase, command, args, options);
    },
    recordSpent(spentMs) {
      if (!hasLedger) {
        accountingStarted = null;
        return;
      }
      const wholeSpan = accountingStarted === null ? spentMs : now() - accountingStarted;
      const row = Math.max(0, Math.ceil(Math.max(spentMs, wholeSpan)));
      try {
        // Node's `a` flag opens with O_APPEND: concurrent runner rows cannot overwrite each other.
        appendFile(ledger, `${row}\n`, { encoding: 'utf8', flag: 'a' });
      } catch {
        // Absence is a fail-closed ledger state for the next runner. Exhaust this process too.
        current().exhaust('read-shadow ledger append');
        try { removeFile(ledger, { force: true }); } catch { /* an unreadable ledger still fails closed */ }
      } finally {
        accountingStarted = null;
      }
    },
  };
}

function dropCrateShadow(project, error, state, write) {
  if (state.dropped) return;
  state.dropped = true;
  write(`[gate] ${project} crate-shadow: dropped (${oneLine(error?.message || error)}); no crate rows\n`);
}

function dropGrade(project, error, state, write) {
  if (state.dropped) return;
  state.dropped = true;
  if (error instanceof ShadowBudgetExhausted) {
    write(`[gate] ${project} grade (shadow): skipped for budget (${oneLine(error.message)})\n`);
  } else {
    write(`[gate] ${project} grade (shadow): dropped (${oneLine(error?.message || error)})\n`);
  }
}

// The flow log's enrichment rows, read once per log state per process. A plain read: no `epr`
// process, and no flow-store lock is taken or held. A log over the cap is not read.
const GRADE_FLOW_LOG_CAP_BYTES = 64 * 1024 * 1024;
const gradeEnrichmentCache = new Map();
function gradeEnrichments(device, { readFile = readFileSync, statFile = statSync } = {}) {
  const log = resolve(device, '.eprfs/status/flows.jsonl');
  let stat;
  try { stat = statFile(log); } catch { return { rows: [], unread: null }; }
  if (stat.size > GRADE_FLOW_LOG_CAP_BYTES) return { rows: [], unread: 'enrichments (flow log over the read cap)' };
  const stamp = `${stat.size}:${stat.mtimeMs}`;
  const held = gradeEnrichmentCache.get(log);
  if (held?.stamp === stamp) return { rows: held.rows, unread: null };
  const rows = enrichmentsIn(readFile(log, 'utf8'));
  gradeEnrichmentCache.set(log, { stamp, rows });
  return { rows, unread: null };
}

function dropReadShadow(project, error, state, write) {
  if (state.dropped) return;
  state.dropped = true;
  if (error instanceof ShadowBudgetExhausted) {
    write(`[gate] ${project} read-shadow: skipped for budget (${oneLine(error.message)})\n`);
  } else {
    write(`[gate] ${project} read-shadow: dropped (${oneLine(error?.message || error)})\n`);
  }
}

function rfc3339(value) {
  try {
    const date = new Date(value);
    if (!Number.isNaN(date.getTime())) return date.toISOString();
  } catch { /* telemetry never changes a gate verdict */ }
  return new Date().toISOString();
}

/** The gate-live run is already one id per standalone invocation or pre-push. */
export function gateReadingRunId(env = process.env) {
  const explicit = oneLine(env.GATE_READING_RUN_ID);
  if (explicit) return explicit;
  const live = oneLine(env.GATE_LIVE_RUN);
  return live ? basename(live).replace(/\.jsonl$/, '') : invocationReadingRunId;
}

export function gateReadingRow(row) {
  return {
    measure: GATE_READING_MEASURE,
    value: finiteSeconds(row.seconds),
    unit: 'seconds',
    run_id: oneLine(row.runId),
    step: oneLine(row.step),
    ...(row.parent ? { parent: oneLine(row.parent) } : {}),
    key: oneLine(row.key),
    key_changed: oneLine(row.keyChanged),
    status: oneLine(row.status),
    outcome: oneLine(row.outcome),
    waited_seconds: finiteSeconds(row.waitedSeconds),
    lease_waited_seconds: finiteSeconds(row.leaseWaitedSeconds),
    synthesized: row.synthesized === true,
    started_at: rfc3339(row.startedAt),
    ended_at: rfc3339(row.endedAt),
    at: rfc3339(row.at || new Date()),
  };
}

/** Collect only. The native writer is called once when this runner process exits. */
export function emitGateReading(row, {
  env = process.env, collector = invocationGateReadings,
  append = appendFileSync,
} = {}) {
  if (env.GATE_READING === 'off' || !row.runId) return;
  const reading = gateReadingRow(row);
  if (env.GATE_READING_ROWS) {
    try { append(env.GATE_READING_ROWS, `${JSON.stringify(reading)}\n`, { mode: 0o600 }); }
    catch { /* shadow telemetry never changes a gate verdict */ }
    return;
  }
  // Under a push without a scratch file the hook reports the drop once; this runner keeps nothing.
  if (env.GATE_READING_PUSH) return;
  collector.push(reading);
}

/**
 * The per-step lines gate-steps.sh appends to GATE_STEP_TIMES (name, exit, start, end in epoch
 * seconds, tab-separated) as step readings. A line of any other shape is skipped: telemetry never
 * changes a verdict and never invents a step.
 */
export function parseStepTimes(text) {
  const steps = [];
  for (const line of String(text || '').split('\n')) {
    const fields = line.split('\t');
    if (fields.length !== 4 || !/^[A-Za-z0-9_-]+$/.test(fields[0]) || !/^\d{1,3}$/.test(fields[1])) continue;
    const started = Number(fields[2]);
    const ended = Number(fields[3]);
    if (!Number.isFinite(started) || !Number.isFinite(ended) || started <= 0 || ended < started) continue;
    steps.push({ name: fields[0], exit: Number(fields[1]), started, ended });
  }
  return steps;
}

/** One gate-step reading per recorded step, plus all-or-nothing per-crate shadow rows when present. */
export function emitStepReadings(project, reading, text, {
  env = process.env, collector, crateContext = null,
  crateShadowState = { dropped: false }, write = line => process.stdout.write(line),
} = {}) {
  const steps = parseStepTimes(text);
  for (const step of steps) {
    const parent = `${project}:${step.name}`;
    const seconds = step.ended - step.started;
    emitGateReading({
      ...reading,
      step: parent,
      parent: project,
      outcome: 'ran',
      status: step.exit === 0 ? 'green' : 'red',
      synthesized: false,
      waitedSeconds: 0,
      leaseWaitedSeconds: 0,
      seconds,
      startedAt: new Date(step.started * 1000),
      endedAt: new Date(step.ended * 1000),
    }, { env, collector });
  }
  if (!crateContext || crateShadowState.dropped) return;

  const crateRows = [];
  try {
    for (const step of steps) {
      if (project !== 'eprfs' || EPRFS_RECIPE_STEP_COVERAGE[step.name] !== 'workspace') continue;
      const parent = `${project}:${step.name}`;
      crateContext.budget?.check('crate keys');
      const keys = crateKeys({ ...crateContext, step: parent });
      crateContext.budget?.check('crate keys');
      const crates = Object.entries(keys);
      if (crates.length === 0) continue;
      for (const [crate, key] of crates) {
        crateRows.push({
          ...reading,
          step: `${parent}:${crate}`,
          parent,
          key,
          keyChanged: 'unknown',
          outcome: 'ran',
          status: step.exit === 0 ? 'green' : 'red',
          synthesized: false,
          waitedSeconds: 0,
          leaseWaitedSeconds: 0,
          seconds: 0,
          startedAt: new Date(step.started * 1000),
          endedAt: new Date(step.ended * 1000),
        });
      }
    }
  } catch (error) {
    dropCrateShadow(project, error, crateShadowState, write);
    return;
  }
  for (const row of crateRows) {
    emitGateReading(row, { env, collector });
  }
}

/** Best-effort by contract: one bounded native call, one warning, no changed gate status. */
export function appendGateReadings(rows, {
  root = ROOT, env = process.env, spawn = spawnSync,
  warn = text => process.stderr.write(text),
  write = text => process.stdout.write(text),
  makeScratch = prefix => mkdtempSync(prefix),
  writeRows = (path, bytes) => writeFileSync(path, bytes, { mode: 0o600 }),
  removeScratch = path => rmSync(path, { recursive: true, force: true }),
} = {}) {
  if (env.GATE_READING === 'off' || env.GATE_READING_ROWS || env.GATE_READING_PUSH || rows.length === 0) return;
  let scratch = null;
  try {
    scratch = makeScratch(join(tmpdir(), 'gate-reading-'));
    const from = join(scratch, 'rows.jsonl');
    writeRows(from, `${rows.map(row => JSON.stringify(row)).join('\n')}\n`);
    const result = spawn(env.TIMEOUT_BIN || 'timeout',
      ['-k', '1', '5', env.EPR_BIN || 'epr', 'flow', 'reading', 'append', '--from', from, '--root', root], {
        cwd: root, stdio: ['ignore', 'ignore', 'pipe'], encoding: 'utf8',
      });
    if (result?.status !== 0 || result?.error) {
      const reason = oneLine(String(result?.stderr || '').split('\n')[0] || '');
      const why = result?.error?.message || `exit ${result?.status ?? 'unknown'}${reason ? `: ${reason}` : ''}`;
      warn(`[gate-reading] run readings dropped (${oneLine(why)})\n`);
    } else {
      const runId = oneLine(rows.at(-1)?.run_id);
      if (runId) {
        const report = spawn(env.TIMEOUT_BIN || 'timeout',
          ['-k', '1', '5', env.EPR_BIN || 'epr', 'flow', 'report', 'reading', '--crate-shadow', '--run', runId, '--root', root], {
            cwd: root, stdio: ['ignore', 'pipe', 'ignore'], encoding: 'utf8',
          });
        if (report?.status === 0 && report.stdout) write(String(report.stdout));
      }
    }
  } catch (error) {
    warn(`[gate-reading] run readings dropped (${oneLine(error?.message || error)})\n`);
  } finally {
    if (scratch) {
      try { removeScratch(scratch); }
      catch { /* cleanup is telemetry too and cannot change the runner verdict */ }
    }
  }
}

function crateKeyContext({ root, project, cargo, metadata, commit, childEnv, readsOf, spawn, budget }) {
  if (!metadata || !metadataToolchain(metadata)) return null;
  budget.check('crate context');
  const run = (phase, command, args, options) => budget.spawn(spawn, phase, command, args, options);
  // The keys read the gated commit's trees; the recipe ran on this checkout. They describe what ran
  // only when the checkout is that commit and the crates' trees are clean: otherwise a green would
  // be recorded under a key for code that never ran, so no crate row is written at all.
  const head = run('crate context', 'git', ['rev-parse', '--verify', '--quiet', 'HEAD^{commit}'], { cwd: root, encoding: 'utf8' });
  if (head?.status !== 0 || String(head.stdout || '').trim() !== commit) return null;
  const repoPath = path => relative(root, resolve(root, path)).split('\\').join('/') || '.';
  const memberIds = new Set(metadata.workspace_members || []);
  const members = (metadata.packages || [])
    .filter(pkg => memberIds.has(pkg.id))
    .map(pkg => repoPath(dirname(pkg.manifest_path)));
  if (members.length !== memberIds.size) return null;
  const workspaceDir = (cargo.workspace || project.dir).replace(/\/$/, '');
  const status = run('crate context', 'git', ['status', '--porcelain', '--untracked-files=all', '--', workspaceDir, ...members], {
    cwd: root, encoding: 'utf8',
  });
  if (status?.status !== 0 || String(status.stdout || '').trim()) return null;
  const objectAt = (path, phase = 'crate context') => {
    const result = run(phase, 'git', ['rev-parse', '--verify', '--quiet', `${commit}:${path}`], {
      cwd: root, encoding: 'utf8',
    });
    if (result?.status === 1) return null;
    if (result?.status !== 0) throw new Error(`git object read failed for ${path}`);
    const oid = String(result.stdout || '').trim();
    if (!oid) throw new Error(`git object read returned no oid for ${path}`);
    return oid;
  };
  const workspace = cargo.workspace || project.dir;
  const workspaceRepoDir = repoPath(workspace.replace(/\/$/, ''));
  const repoFile = (dir, path) => dir === '.' ? path : `${dir}/${path}`;
  const lockPath = repoFile(workspaceRepoDir, 'Cargo.lock');
  const lock = run('crate context', 'git', ['show', `${commit}:${lockPath}`], { cwd: root, encoding: 'utf8' });
  if (lock?.status !== 0) return null;
  const lockDigest = createHash('sha256').update(String(lock.stdout || '')).digest('hex');
  const configNames = [
    '.cargo/config.toml', '.cargo/config', 'clippy.toml', '.clippy.toml',
    'rustfmt.toml', '.rustfmt.toml', 'rust-toolchain', 'rust-toolchain.toml',
  ];
  const configDirs = [];
  for (let dir = workspaceRepoDir; ; dir = dirname(dir)) {
    if (dir.startsWith('..')) throw new Error(`workspace is outside the repository: ${workspace}`);
    configDirs.push(dir);
    if (dir === '.') break;
  }
  const digestObjects = paths => createHash('sha256').update(JSON.stringify(paths.map(path => ({
    path,
    oid: objectAt(path),
  })))).digest('hex');
  const workspaceConfigDigest = digestObjects([
    repoFile(workspaceRepoDir, 'Cargo.toml'),
    ...configDirs.flatMap(dir => configNames.map(path => repoFile(dir, path))),
  ]);
  // Deliberately coarse: the project key already treats any root justfile edit as recipe identity,
  // so every crate step does too rather than attempting to parse one recipe's command body.
  const recipeDigest = digestObjects([
    'justfile',
    'genesis/orchestrator/gate-steps.sh',
    'genesis/orchestrator/cargo-lint.mjs',
    'genesis/orchestrator/run-local-gate.sh',
  ]);
  const readPaths = new Set();
  for (const member of members) {
    budget.check('read set');
    const reads = readsOf
      ? readsOf(member)
      : readSet({ root, project, sources: [`${member}/**`] }).paths;
    for (const path of reads || []) {
      const normalized = String(path).replace(/^\.\//, '').replace(/\\/g, '/');
      if (!members.some(dir => normalized === dir || normalized.startsWith(`${dir}/`))) readPaths.add(normalized);
    }
  }
  const outside = [...readPaths].sort().map(path => ({
    path,
    oid: isReadPattern(path)
      ? patternDigest(root, commit, path,
        (command, args, options) => run('crate context', command, args, options))
      : objectAt(path),
  }));
  const outsideReadsDigest = createHash('sha256').update(JSON.stringify(outside)).digest('hex');
  budget.check('crate context');
  const rustflags = Object.hasOwn(cargo, 'rustflags') ? cargo.rustflags : (childEnv.RUSTFLAGS ?? null);
  return {
    metadata,
    treeOf: dir => objectAt(repoPath(dir), 'crate keys'),
    toolchain: metadataToolchain(metadata),
    lockDigest,
    outsideReadsDigest,
    workspaceConfigDigest,
    recipeDigest,
    cargoEnv: { ...cargoEnvOf(childEnv), RUSTFLAGS: rustflags },
    budget,
  };
}

function keyReading(evidence) {
  if (!evidence?.key) {
    return {
      key: `none: ${oneLine(evidence?.line) || 'closure evidence returned no key'}`,
      keyChanged: 'unknown',
    };
  }
  return { key: evidence.key, keyChanged: 'unknown' };
}

export function runProject(project, printOnly, namesOnly, {
  root = ROOT, env = process.env, spawn = spawnSync,
  write = text => process.stdout.write(text), warn = text => process.stderr.write(text),
  readsOf, recipe = null, tap = spawn === spawnSync, clock = () => process.hrtime.bigint(),
  wallClock = () => new Date(), inputsPolicy = null, readingCollector = invocationGateReadings,
  readShadowOf = readShadowComparison, readShadowBudget = null, gradeDeps = {},
} = {}) {
  const cargo = project.run.cargo || {};
  const targetDir = worktreeTargetDir(cargo.targetDir || '', root);
  const slotDir = cargoSlotPath(cargo, targetDir, root, env);
  const slotLock = slotGateLockPath(slotDir);
  const gateRunEnv = (slotWaitPath = null) => {
    const child = gateChildEnv(project, env, root);
    if (slotLock) {
      child.GATE_SLOT_LOCK_PATH = slotLock;
      child.GATE_SLOT_HOLDER_PATH = resolve(slotDir, '.gate.holder');
    } else {
      delete child.GATE_SLOT_LOCK_PATH;
      delete child.GATE_SLOT_HOLDER_PATH;
    }
    if (slotWaitPath) child.GATE_SLOT_WAIT_RESULT = slotWaitPath;
    else delete child.GATE_SLOT_WAIT_RESULT;
    return child;
  };
  const args = [
    root,
    project.name,
    project.dir,
    project.run.kind,
    recipe || project.run.recipe,
    cargo.workspace || '',
    targetDir,
    cargo.profile || 'dev',
    Object.hasOwn(cargo, 'rustflags') ? cargo.rustflags : '__inherit__',
  ];

  const newSlotWaitAttempt = () => {
    if (!cargo.workspace && !targetDir) return { argv: args, dir: null, path: null };
    try {
      const dir = mkdtempSync(join(realpathSync(tmpdir()), 'gate-slot-wait-'));
      const path = join(dir, 'result.tsv');
      return { argv: [...args, path, slotLock], dir, path };
    } catch {
      return { argv: [...args, '', slotLock], dir: null, path: null };
    }
  };
  const collectSlotWait = attempt => {
    const result = slotWaitResult(attempt.path);
    if (attempt.dir) {
      try { rmSync(attempt.dir, { recursive: true, force: true }); } catch { /* telemetry */ }
    }
    return result;
  };
  const clearSlotWait = attempt => {
    if (!attempt.path) return;
    try { writeFileSync(attempt.path, ''); } catch { /* telemetry */ }
  };

  if (namesOnly) {
    write(`${project.name}\n`);
    return 0;
  }
  if (printOnly) {
    // resolvedCargoEnv is the same manifest∪pool-policy merge gateChildEnv would send
    // to run-local-gate.sh — a dry-run rail so `--print` proves the cap without
    // claiming the cargo berth (no spawnSync).
    const resolvedEnv = gateChildEnv(project, {}, root).GATE_CARGO_ENV;
    const resolvedCargoEnv = resolvedEnv ? JSON.parse(resolvedEnv) : {};
    write(`${JSON.stringify({ name: project.name, dir: project.dir, run: project.run, resolvedCargoEnv, resolvedTargetDir: targetDir || null, reasons: project.reasons || [] })}\n`);
    return 0;
  }

  const readingStarted = clock();
  const reading = {
    runId: gateReadingRunId(env),
    step: project.name,
    key: 'none: closure evidence not reached',
    keyChanged: 'unknown',
    waitedSeconds: 0,
    leaseWaitedSeconds: 0,
    startedAt: rfc3339(wallClock()),
  };
  let readShadow = null;
  let sharedReadShadowBudget = readShadowBudget;
  let runReadShadow = () => {};
  let readShadowRowsEmitted = false;
  const emitReadShadowRows = (outcome, status) => {
    if (!readShadow || readShadowRowsEmitted) return;
    readShadowRowsEmitted = true;
    const at = wallClock();
    for (const row of readShadow.rows) {
      const seconds = Number.isFinite(row.timing?.totalMs) ? row.timing.totalMs / 1000 : 0;
      emitGateReading({
        ...reading,
        step: `${project.name}:read-shadow:${row.name}`,
        parent: project.name,
        key: row.key,
        keyChanged: row.keyChanged,
        outcome,
        status,
        synthesized: false,
        waitedSeconds: 0,
        leaseWaitedSeconds: 0,
        seconds,
        startedAt: new Date(new Date(at).getTime() - seconds * 1000),
        endedAt: at,
      }, { env, collector: readingCollector });
    }
  };
  let readingComplete = false;
  // A gate stopped on request (`gate-live.mjs stop --now`; its tap ended the gate's process group)
  // has no verdict, so this records nothing for it: no closure evidence, no gate-verdict@1, no
  // cycle observation, no step or gate reading (the reading vocabulary is ran|reused|skipped, and
  // none of those is true of it), and no live gate-verdict. Its one live event is gate-stopped,
  // which closes the run interrupted. The exit status is non-zero, so the push is refused.
  const stoppedGate = (stopped, step = null) => {
    readingComplete = true;
    const by = stopped.by || 'a caller with no session';
    // "No verdict" is the claim, and no more: scratch the gate already wrote (step times, a slot's
    // build products) and a producer put it already made (eprfs-produce.sh) are not undone.
    write(`[gate] ${project.name}${step ? ` (${step})` : ''} stopped on request by ${by}: its process group was ended `
      + `(${stopped.signal || 'nothing left to signal'}); no verdict recorded (what it already wrote or put is not undone)\n`);
    if (stopped.escaped) {
      write(`[gate] ${project.name}: a process outside the gate's tree may still be running: `
        + `${stopped.escaped.length ? stopped.escaped.map(h => `pid ${h.pid}${h.cmd ? ` (${h.cmd})` : ''}`).join(', ') : 'none found carrying its environment'}\n`);
    }
    liveEvent(env, {
      kind: 'gate-stopped', project: project.name, ...(step ? { step } : {}), by: stopped.by || null,
      // The request this stop answers, so `gate-live.mjs stop` confirms only its own.
      requests: stopped.request ? [stopped.request] : [],
      signal: stopped.signal || null, ...(stopped.survivors?.length ? { survivors: stopped.survivors } : {}),
      ...(stopped.escaped ? { escaped: stopped.escaped.map(h => ({ pid: h.pid, ...(h.cmd ? { cmd: h.cmd.slice(0, 120) } : {}) })) } : {}),
      evidence: 'none',
    });
    return GATE_STOPPED_EXIT;
  };
  const emitSlotWaitReading = (slotWait, outcome) => {
    reading.waitedSeconds = slotWait.seconds;
    if (!slotWait.measured) return;
    emitGateReading({
      ...reading,
      step: `${project.name}:slot-wait`,
      parent: project.name,
      key: 'none: cargo slot admission is not closure evidence',
      keyChanged: 'unknown',
      outcome,
      status: 'none',
      synthesized: false,
      seconds: slotWait.seconds,
      endedAt: rfc3339(wallClock()),
    }, { env, collector: readingCollector });
  };
  const slotBusyGate = (slotWait = { measured: false, seconds: 0, holder: 'holder details unavailable' }) => {
    readingComplete = true;
    emitSlotWaitReading(slotWait, 'skipped');
    write(`[gate] ${project.name} NOT RUN: slot held by ${slotWait.holder}; no evidence, verdict, or cycle recorded\n`);
    liveEvent(env, {
      kind: 'not-run', project: project.name, reason: 'slot-held', resource: 'cargo-slot',
      holder: slotWait.holder, waited_seconds: slotWait.seconds,
    });
    return GATE_SLOT_BUSY_EXIT;
  };
  const complete = (status, outcome, seconds = null) => {
    if (!readingComplete) {
      readingComplete = true;
      const elapsed = Number(clock() - readingStarted) / 1e9;
      const executing = seconds === null
        ? Math.max(0, elapsed - finiteSeconds(reading.leaseWaitedSeconds) - finiteSeconds(reading.waitedSeconds))
        : seconds;
      emitGateReading({
        ...reading,
        outcome,
        status: outcome === 'skipped' ? 'none' : (status === 0 ? 'green' : 'red'),
        synthesized: false,
        seconds: executing,
        endedAt: rfc3339(wallClock()),
      }, { env, collector: readingCollector });
    }
    return status;
  };

  if (recipe) {
    if (project.run.kind === 'attested') {
      process.stderr.write(`gate ${project.name}: an attested project has no local recipe to run\n`);
      reading.key = 'none: an attested project has no alternate local recipe';
      return complete(2, 'skipped');
    }
    reading.key = `none: alternate recipe ${recipe} has no gate closure key`;
    write(`\n[gate] ${project.name} (${project.dir}) — recipe ${recipe}\n`);
    const slotAttempt = newSlotWaitAttempt();
    const recipeEnv = gateRunEnv(slotAttempt.path);
    const ran = gateSpawn(project, 'bash', [resolve(root, 'genesis/orchestrator/run-local-gate.sh'), ...slotAttempt.argv], {
      cwd: root,
      stdio: 'inherit',
      env: recipeEnv,
    }, { spawn, env, tap });
    const status = ran.result.status ?? 1;
    const slotWait = collectSlotWait(slotAttempt);
    if (status === GATE_SLOT_BUSY_EXIT) return slotBusyGate(slotWait);
    emitSlotWaitReading(slotWait, 'ran');
    return complete(status, 'ran');
  }

  if (project.run.kind === 'attested') {
    // No local recipe: the pinned commit's own upstream attestation is the gate.
    // Never reaches run-local-gate.sh (which keeps refusing unknown kinds).
    write(`\n[gate] ${project.name} (${project.dir}) — attested, no local recipe\n`);
    reading.key = 'none: attested project has no local closure key';
    liveEvent(env, { kind: 'gate-start', project: project.name, attested: true });
    const attestedStatus = runAttested(project, {
      root,
      env,
      spawn,
      log: line => write(`${line}\n`),
      runEpr: eprArgs => spawn(env.EPR_BIN || 'epr', eprArgs, { cwd: root, stdio: 'ignore', timeout: 30000 }).status,
    });
    if (attestedStatus === CLAIMED_EXIT_CODE) {
      // Claimed is admitted for landing until the debt-aware landing plane exists, but it is
      // not a pass. In particular, do not emit an exit-0 gate verdict: live-run harvest treats
      // those as green, and ordinary closure evidence must never become reusable from a claim.
      write(`[gate] ${project.name}: claimed (admitted with debt; no green evidence)\n`);
      return complete(0, 'ran');
    }
    liveEvent(env, { kind: 'gate-verdict', project: project.name, verdict: { exit: attestedStatus, seconds: 0 } });
    return complete(attestedStatus, 'ran');
  }

  write(`\n[gate] ${project.name} (${project.dir})\n`);
  if (!cycleRowsByRoot.has(root)) {
    // Main and sibling sidecars may share copied rows. Keep their append order and
    // deduplicate through the union, while distinguishing unknown from empty history.
    const union = readCycleUnion(root);
    cycleRowsByRoot.set(root, union.readableLedgers ? union.rows : null);
  }
  const rows = cycleRowsByRoot.get(root);
  let costDecision = null;
  if (rows !== null) {
    let subjectFallback = null;
    try {
      const subject = subjectFor(project, root);
      const shared = [...loadGateRegistry(root).values()].some(other =>
        other.name !== project.name && subjectFor(other, root) === subject);
      if (!shared) subjectFallback = subject;
    } catch { /* Without the registry, only explicitly named project rows are safe. */ }
    costDecision = costLineAndDefer(project, { rows, subjectFallback, env });
    write(`${costDecision.line}\n`);
  }
  const childEnv = gateRunEnv();
  let started = clock();
  const cycleEnv = { ...env, ...cargoEnvOf(childEnv) };
  const cycleDeps = {
    root,
    measuresPath: resolve(root, '.claude/epr-meta/measures.yaml'),
    ledgerPath: architectureLedgerPath(root),
    runEpr: eprArgs => spawn(env.EPR_BIN || 'epr', eprArgs, { cwd: root, stdio: 'ignore', timeout: 30000 }).status,
    print: line => write(`${line}\n`),
    now: () => new Date().toISOString().replace(/\.\d+Z$/, 'Z'),
  };
  if (costDecision?.defer) {
    write(`[gate] ${project.name} deferred to CI: ${costDecision.why}\n`);
    reading.key = `none: deferred before closure evidence — ${oneLine(costDecision.why)}`;
    const seconds = Number(clock() - started) / 1e9;
    recordCycle(project, seconds, 0, { ...cycleEnv, GATE_RUN_KIND: 'deferred' }, cycleDeps);
    liveEvent(env, { kind: 'gate-verdict', project: project.name, verdict: { exit: 0, seconds: 0, deferred: true } });
    return complete(0, 'skipped');
  }

  // Explicit crate gates carry the same style floor as changed-project selection.
  liveEvent(env, { kind: 'gate-start', project: project.name });
  if (project.run.cargo && project.name !== 'rust-style') {
    const styleArgs = [resolve(root, 'genesis/orchestrator/rust-style.sh'), '--scope', project.dir];
    if (env.GATE_COMMIT) styleArgs.push('--head', env.GATE_COMMIT);
    const { result: style, tapped: styleTapped } = gateSpawn(project, 'bash', styleArgs, {
      cwd: root, stdio: 'inherit', env: gateChildEnv(project, env, root),
    }, { spawn, env, tap });
    if (styleTapped?.stopped) return stoppedGate(styleTapped.stopped, 'rust-style');
    if (style.status !== 0) {
      liveEvent(env, { kind: 'gate-verdict', project: project.name, step: 'rust-style', verdict: { exit: style.status || 2, seconds: 0 } });
      reading.key = 'none: rust-style preflight failed before closure evidence';
      return complete(style.status || 2, 'ran');
    }
  }

  // A selection belongs to this project and this push, never to the caller's environment.
  delete childEnv.GATE_CARGO_PACKAGES;
  const narrowProjects = loadPoolPolicy(root).gate_narrow?.projects;
  const crateShadowEligible = Array.isArray(narrowProjects) && narrowProjects.includes(project.name);
  const shadowBudget = crateShadowBudget(env, clock);
  const crateShadowState = { dropped: false };
  const readsCache = new Map();
  // The repository listing every member scan reads is taken once, through the budget (its git
  // subprocess gets the remaining time as its timeout), instead of once per member, unbounded.
  let repositoryListing = null;
  const listRepository = listRoot => {
    if (repositoryListing) return repositoryListing;
    const result = shadowBudget.spawn(spawn, 'read set', 'git',
      ['ls-files', '-z', '--cached', '--others', '--exclude-standard', '--', '*.rs'],
      { cwd: listRoot, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 });
    if (result?.status !== 0) {
      throw new Error(result?.error?.message || String(result?.stderr || '') || 'git ls-files failed');
    }
    repositoryListing = [...new Set(String(result.stdout || '').split('\0').filter(Boolean))];
    return repositoryListing;
  };
  const memberReads = dir => {
    shadowBudget.check('read set');
    if (!readsCache.has(dir)) {
      const reads = readsOf
        ? readsOf(dir)
        : readSet({ root, project, sources: [`${dir}/**`], listFiles: listRepository }).paths;
      shadowBudget.check('read set');
      readsCache.set(dir, reads);
    }
    return readsCache.get(dir);
  };
  let narrowMetadata = null;
  const crateShadowDiscoveryStarted = clock();
  shadowBudget.begin();
  if (crateShadowEligible) {
    const mode = ['off', 'shadow', 'on'].includes(env.GATE_NARROW) ? env.GATE_NARROW : 'shadow';
    // `on` is refused until gate evidence keys a per-package verdict: the evidence store keys the
    // whole project closure, so a narrowed run's green would be recorded as the verdict of every
    // member it never ran (table 2, 2026-11-24). Refused before any evidence lookup, recipe or
    // record, so nothing narrowed can be reused or recorded.
    if (mode === 'on') {
      write(`[gate] ${project.name} REFUSED GATE_NARROW=on: gate evidence has no per-package verdict key yet, so a narrowed green would stand for the whole workspace; run with GATE_NARROW=shadow (the default) or off\n`);
      return 2;
    }
    let selection = { packages: null, why: 'no push base' };
    let files = null;
    if (env.GATE_NARROW_BASE === 'ok' && env.GATE_CHANGED_FILES) {
      try { files = readFileSync(env.GATE_CHANGED_FILES, 'utf8').split('\n').filter(Boolean); }
      catch { /* An unreadable push range must keep the full workspace. */ }
    }
    const workspaceDir = cargo.workspace || project.dir;
    if ((files && mode !== 'off') || env.GATE_READING !== 'off') {
      try {
        const metadataEnv = { ...childEnv, ...cargoEnvOf(childEnv), RUSTC_WRAPPER: '' };
        delete metadataEnv.CARGO_TARGET_DIR;
        if (Object.hasOwn(cargo, 'rustflags')) metadataEnv.RUSTFLAGS = cargo.rustflags;
        narrowMetadata = loadMetadata({ root, workspaceDir,
          spawn: (command, argv, options) => shadowBudget.spawn(
            spawn, 'metadata', command, argv, { ...options, env: metadataEnv },
          ),
        });
        shadowBudget.check('metadata');
      } catch (error) {
        if (error instanceof ShadowBudgetExhausted) {
          dropCrateShadow(project.name, error, crateShadowState, write);
        }
        // A telemetry discovery miss keeps the full workspace and emits no crate rows.
      }
    }
    if (files && mode === 'off') selection.why = 'narrowing disabled';
    else if (files) {
      try {
        if (narrowMetadata) shadowBudget.check('affected packages');
        selection = affectedPackages({ root, workspaceDir, files, metadata: narrowMetadata,
          readsOf: memberReads,
        });
        if (narrowMetadata) shadowBudget.check('affected packages');
      } catch (error) {
        if (error instanceof ShadowBudgetExhausted) {
          dropCrateShadow(project.name, error, crateShadowState, write);
        }
        selection = { packages: null, why: `narrowing unavailable (${String(error?.message || error).split('\n')[0]})` };
      }
    }
    const { packages, why } = selection;
    write(`[gate] ${project.name} narrow (${mode}): ${packages === null
      ? `full workspace — ${why}`
      : `${packages.join(' ')} (${packages.length} of ${narrowMetadata.workspace_members.length} members)`}\n`);
  }
  shadowBudget.end();
  started += clock() - crateShadowDiscoveryStarted;
  // Evidence by input closure (gate-evidence.mjs): has THIS closure — the step's declared
  // input bytes at the pushed commit, the recipe, the resolved cargo env — already been
  // gated, by whom, and when? Reuse requires an audited project and either this workspace's
  // signer or an admitted sibling verdict under D5's stricter fences.
  // The gated commit is resolved once to a full sha, and evidence, reuse, recording and
  // publication all name it: GATE_COMMIT is the commit being pushed (pre-push exports it), and
  // `just gate` asks about HEAD. Fail-open: an exception in the evidence lookup is one line.
  const revParse = rev => {
    const r = spawn('git', ['rev-parse', '--verify', '--quiet', `${rev}^{commit}`], { cwd: root, encoding: 'utf8' });
    const sha = r?.status === 0 ? String(r.stdout || '').trim() : '';
    return /^[0-9a-f]{40,64}$/.test(sha) ? sha : null;
  };
  const headSha = revParse('HEAD');
  const pushed = (env.GATE_COMMIT || '').trim();
  const gateCommit = pushed ? (revParse(pushed) || pushed) : (headSha || 'HEAD');
  // The run's input class (gate-inputs.mjs), read by this key and its reuse selector.
  // Resolving it (a probe at most once per push) is taken out of the gate's timed span.
  const policyStarted = clock();
  const inputsLine = line => write(`${line}\n`);
  let inputs = inputsPolicy;
  try {
    inputs ||= resolveGateInputs({ env: childEnv, britBin: resolveBin('brit', ['BRIT_BIN'], childEnv), spawn, log: inputsLine });
  } catch {
    inputs = null;
  }
  started += clock() - policyStarted;
  let evidence = { line: 'unavailable (evidence lookup threw)' };
  try {
    evidence = gateEvidence(project, { env: childEnv, root, commit: gateCommit, cargoEnv: cargoEnvOf(childEnv), spawn, warn, inputs });
  } catch (error) {
    evidence = { line: `unavailable (${String(error?.message || error).split('\n')[0]})` };
  }
  Object.assign(reading, keyReading(evidence));
  const inputsClass = inputs?.cls ?? LEGACY_ALL;
  write(`[gate] ${project.name} evidence: ${evidence.line}\n`);
  // GATE_READ_SHADOW (default on): after this gate's seconds, evidence and cycle are recorded,
  // compare its canonical closure at the integration base and gated commit under two proposed
  // audit fixes. One GATE_READ_SHADOW_BUDGET_MS budget (default 15000) belongs to the hook push
  // when its ledger is set, otherwise to this runner invocation; every subprocess, including suite
  // inventory recomputation, receives the remaining bound. The comparison decides nothing. Neither
  // fix may decide until proof covers
  // #[path]-included dependency tests/** support, format! joins with variable suffixes, absent
  // paths, and eprfs running and blocking on this same push.
  if (env.GATE_READ_SHADOW !== 'off' && evidence.key && evidence.material) {
    let scopeKnown = false;
    try { scopeKnown = ['paths', 'tree'].includes(JSON.parse(evidence.material).reads?.scope); } catch { /* no shadow */ }
    if (scopeKnown) {
      runReadShadow = (outcome, status) => {
        const readShadowState = { dropped: false };
        sharedReadShadowBudget ||= readShadowPushBudget(env, clock);
        let shadowStarted = false;
        let spentBefore = 0;
        try {
          sharedReadShadowBudget.check('read-shadow');
          spentBefore = sharedReadShadowBudget.spentMs();
          sharedReadShadowBudget.begin();
          shadowStarted = true;
          const bounded = (command, args, options) => sharedReadShadowBudget.spawn(
            spawn, 'read-shadow', command, args, options,
          );
          const { base } = touchedProcessSteps(project, { root, gateCommit, spawn: bounded });
          if (base) {
            sharedReadShadowBudget.check('read-shadow');
            readShadow = readShadowOf(project, evidence, {
              root, base, commit: gateCommit, env: childEnv, cargoEnv: cargoEnvOf(childEnv),
              spawn: bounded, inputs, clock, reportTimings: true,
              check: phase => sharedReadShadowBudget.check(phase || 'read-shadow traversal'),
            });
            sharedReadShadowBudget.check('read-shadow');
            if (readShadow) {
              write(`${readShadow.line}\n`);
              emitReadShadowRows(outcome, status);
            }
          }
        } catch (error) {
          dropReadShadow(project.name, error, readShadowState, write);
          readShadow = null;
        } finally {
          if (shadowStarted) {
            sharedReadShadowBudget.end();
            sharedReadShadowBudget.recordSpent(sharedReadShadowBudget.spentMs() - spentBefore);
          }
        }
      };
    }
  }
  // Shadow only (gate-recipe-key.mjs): whether a justfile edit since the integration base left this
  // root-just gate's recipe closure unchanged. It reads `evidence.material` and changes nothing the
  // reuse decision, the key or the record below reads; GATE_RECIPE_KEY=off silences it.
  // Its own span is taken out of the gate's timed span, so the durations recorded below (verdict
  // JSON, --duration-ms, the cycle row) are what they are with the shadow off.
  const shadowStarted = clock();
  try {
    const shadow = recipeKeyShadow(project, { root, commit: gateCommit, material: evidence.material,
      dirty: pushed && gateCommit !== headSha ? true : evidence.dirty, env, spawn });
    if (shadow) write(`[gate] ${project.name} recipe-key (shadow): ${shadow}\n`);
  } catch { /* a shadow is never a red */ }
  started += clock() - shadowStarted;
  // The recipe tests this checkout; a gated commit that is not its HEAD is neither reusable nor
  // recorded, on every entry path (pre-push also says so with GATE_EVIDENCE_RECORD=0).
  if (pushed && gateCommit !== headSha) {
    write(`[gate] ${project.name}: the gated commit ${String(gateCommit).slice(0, 9)} is not this checkout's HEAD ${String(headSha).slice(0, 9)}; not reusable, and nothing is recorded\n`);
    evidence = { ...evidence, dirty: true };
  }
  const enabled = loadPoolPolicy(root).gate_reuse?.projects;
  // The signer is asked of the binary gateEvidence resolved from childEnv, never one from process.env.
  const signerOptions = { env: childEnv, buildRefBin: evidence.buildRefBin };
  const taskOn = taskEnabled(project, env);
  const selfId = (taskOn || env.GATE_EVIDENCE === 'reuse') ? selfAgentId(root, spawn, signerOptions) : null;
  const reuseContext = {
    mode: env.GATE_EVIDENCE,
    enabled: Array.isArray(enabled) && enabled.includes(project.name),
    selfId: env.GATE_EVIDENCE === 'reuse' ? selfId : null,
    selectorCheck: reuseSelector(project, gateCommit, { root, env: childEnv, spawn, inputs: inputsClass }),
  };
  const carry = landerWait({ intent: env.GATE_LANDING_INTENT, project: project.name, closure: evidence.key,
    mode: env.GATE_CARRY_WAIT || 'shadow', ownStopAt: env.GATE_LANDING_STOP_AT, deps: {
      // A device read (sibling carries); readings, cycle timing and pin checks above stay on the session's epr.
      exec: (args, options) => spawn(deviceEpr({ env }), args, { ...options, cwd: root, env }),
      now: () => Date.now() / 1000, monotonic: () => Number(clock()) / 1e9,
      sleep: seconds => Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, seconds * 1000),
      log: line => write(`[gate] ${project.name} carry: ${line}\n`),
      verdict: () => siblingVerdicts(project, evidence.key, { root, env, spawn }).candidates,
    } });
  reading.leaseWaitedSeconds = carry.waitedS;
  // One admission decision for both run modes (decideReuse); ark is then asked only to run.
  const gradeOn = gradeMode(env) !== 'off' && Boolean(evidence.key);
  let observedSiblings = null;
  const decision = decideReuse(project, evidence, reuseContext, {
    root, env, spawn, ...(gradeOn ? { observe: read => { observedSiblings = read; } } : {}),
  });
  // The reason a reuse was declined, said the moment it is decided and under the evidence line's
  // own name, so a reader of `evidence: miss` finds the why beside it (arrival backlog row 101).
  // The decision above is untouched, and the `not reused:` line where the gate starts is kept.
  if (env.GATE_EVIDENCE === 'reuse' && !decision.reuse) {
    write(`[gate] ${project.name} evidence: not reused: ${decision.why}\n`);
  }
  // GATE_GRADE=shadow (default; `off` adds no read, spawn or write; there is no `on`): after this
  // gate's seconds, evidence and cycle are recorded, say what evidence this device holds for the
  // closure and how it would read under the PROPOSED graded table (gate-grade.mjs). It draws on the
  // shadow budget shared with the read-shadow, every subprocess it starts receives the remaining
  // bound, and an overrun or any failure is one line and no row. It is called only after
  // `complete`, so nothing it does reaches a decision, a record or the exit status.
  const runGrade = gradeOn ? (outcome, status, today) => {
    const gradeState = { dropped: false };
    let gradeStarted = false;
    let spentBefore = 0;
    try {
      sharedReadShadowBudget ||= readShadowPushBudget(env, clock);
      sharedReadShadowBudget.check('grade');
      spentBefore = sharedReadShadowBudget.spentMs();
      sharedReadShadowBudget.begin();
      gradeStarted = true;
      const bounded = (command, args, options) => sharedReadShadowBudget.spawn(spawn, 'grade', command, args, options);
      const check = () => sharedReadShadowBudget.check('grade');
      const unread = [];
      let range = pushRange({ root, gateCommit, spawn: bounded });
      check();
      // Each changed path's git mode, from git: a gitlink or a symlink is never read by its name.
      if (range?.base && range.files.length > 0) {
        const raw = bounded('git', ['diff', '--raw', '--no-abbrev', '--no-renames', '-z', range.base, gateCommit], { cwd: root, encoding: 'utf8' });
        check();
        range = { ...range, modes: raw?.status === 0 ? rawModes(raw.stdout) : null };
      }
      // This workspace's key: the one the reuse decision read, else one bounded brit-build-ref call.
      const gradeSelf = selfId ?? selfAgentId(root, bounded, signerOptions);
      check();
      // Sibling verdicts: what the reuse decision read, else one bounded `epr peer verdicts`.
      const siblingRead = observedSiblings
        ?? siblingVerdicts(project, evidence.key, { root, env, spawn: bounded });
      check();
      if (siblingRead.error) unread.push(`sibling verdicts (${siblingRead.error})`);
      const device = deviceRoot(root, bounded);
      check();
      const enriched = gradeEnrichments(device, gradeDeps);
      check();
      if (enriched.unread) unread.push(enriched.unread);
      // A sponsored runner's result: the harvested `ci:<pipeline>` note at the gated commit.
      const carriers = [];
      let pipeline = null;
      try { pipeline = JSON.parse(readFileSync(resolve(root, project.manifestPath), 'utf8')).pipeline || null; } catch { pipeline = null; }
      if (pipeline && evidence.buildRefBin) {
        const note = readBuildNote(`ci:${pipeline}`, gateCommit, { buildRefBin: evidence.buildRefBin, root, spawn: bounded });
        check();
        carriers.push({
          name: 'jenkins', state: note.state, reason: note.reason,
          success: note.node?.success, closure: note.node?.inputsHash,
        });
      }
      // After a run, the rows read before it no longer say what this device holds: one bounded,
      // brit-verified re-read of this closure's rows, so the verdict just recorded is counted from
      // its record and never from this process's own word. A reuse already read them fresh.
      let localRows = evidence.rows || [];
      let localUnverifiable = evidence.unverifiable || [];
      if (outcome === 'ran' && evidence.buildRefBin) {
        const fresh = findEvidence(project, evidence.key, { buildRefBin: evidence.buildRefBin, root, spawn: bounded, warn: () => {} });
        check();
        if (fresh.error) unread.push(`this closure's rows after the run (${fresh.error})`);
        else { localRows = fresh.rows; localUnverifiable = fresh.unverifiable; }
      }
      const grade = gradeOf({
        range, isGovernance: isSiblingGovernancePath, project: project.name, key: evidence.key,
        selfId: gradeSelf, local: localRows, unverifiable: localUnverifiable,
        siblings: siblingRead.candidates || [], enrichments: enriched.rows, carriers, unread,
      });
      write(`${gradeLine(project.name, grade, today)}\n`);
      const at = wallClock();
      const seconds = Math.max(0, (sharedReadShadowBudget.spentMs() - spentBefore) / 1000);
      // gate-step-reading@1 refuses unknown fields, so the grade is a sibling row kind, as the
      // crate and read shadows are: its class, count and reading ride in the step name.
      emitGateReading({
        ...reading, step: gradeStep(project.name, grade), parent: project.name, key: evidence.key,
        keyChanged: 'unknown', outcome, status, synthesized: false, waitedSeconds: 0,
        leaseWaitedSeconds: 0, seconds, startedAt: new Date(new Date(at).getTime() - seconds * 1000),
        endedAt: at,
      }, { env, collector: readingCollector });
    } catch (error) {
      dropGrade(project.name, error, gradeState, write);
    } finally {
      if (gradeStarted) {
        try {
          sharedReadShadowBudget.end();
          sharedReadShadowBudget.recordSpent(sharedReadShadowBudget.spentMs() - spentBefore);
        } catch { /* a shadow is never a red */ }
      }
    }
  } : () => {};
  // GATE_INPUTS=shadow: whether the step fingerprints are unchanged under sources. Read only, inside
  // shadow's budget, and its span is taken out of the gate's timed span.
  // No brit spawn unless a borrowed step's own buildProcess file changed: that is decided from git
  // first, and only then are the touched steps fingerprinted. Every read, git and brit, is inside the
  // budget and killed at its timeout.
  if (inputs?.shadow && evidence?.key && evidence.britBin && evidence.fingerprints?.length && !SUITE_READ_LISTS[project.name]) {
    const shadowSpan = clock();
    inShadowBudget(inputsLine, bound => {
      const run = bound(spawn);
      const { base, steps } = touchedProcessSteps(project, { root, gateCommit, spawn: run });
      if (steps.length === 0) return;
      const shadow = inputsShadow(project, evidence, { root, gateCommit, base, steps, run });
      if (shadow.refused) endShadow(shadow.refused, inputsLine);
      for (const line of shadow.lines || []) write(`${line}\n`);
    });
    started += clock() - shadowSpan;
  }
  if (decision.reuse) {
    const row = decision.row;
    const sibling = decision.independence === 'sibling';
    const signer = sibling
      ? `signer sibling ${shortDid(row.device)}, independence: sibling`
      : 'signer this workspace';
    // A sibling row's `at` is the author time of the HEAD of the checkout `epr flow note` wrote
    // from (the device's main checkout), not a clock reading, so no age is derived from it; the
    // sibling's own run time is what is known.
    const when = sibling
      ? `in ${Math.round(Number(row.seconds) || 0)} s on the sibling`
      : ageOf(row.attestation.builtAt);
    write(`[gate] ${project.name} reused: green ${when} at ${String(row.commit).slice(0, 9)} (${signer})\n`);
    const seconds = Number(clock() - started) / 1e9;
    recordCycle(project, seconds, 0, {
      ...cycleEnv, GATE_RUN_KIND: sibling ? 'sibling-reused' : 'reused',
    }, cycleDeps);
    liveEvent(env, { kind: 'gate-verdict', project: project.name, verdict: { exit: 0, seconds: 0, reused: true } });
    const completed = complete(0, 'reused', Math.max(0, seconds - finiteSeconds(reading.leaseWaitedSeconds)));
    runReadShadow('reused', 'green');
    runGrade('reused', 'green', sibling ? 'reuse sibling' : 'reuse own');
    return completed;
  }
  if (env.GATE_EVIDENCE === 'reuse') write(`[gate] ${project.name} not reused: ${decision.why}\n`);
  // A suite's named half is handed what its key expects (the gated sha and the inventory, package
  // and node digests); the builder reports in its result whether its view held those bytes.
  if (SUITE_READ_LISTS[project.name]) {
    if (evidence.suite) childEnv[SUITE_EXPECT_ENV] = encodeExpectation(evidence.suite);
    else delete childEnv[SUITE_EXPECT_ENV];
  }
  // A suite's named half writes one result per attempt into a fresh directory outside every
  // checkout (the OS temporary directory, which a Task child writes to as well), removed after.
  const suiteNamed = Boolean(SUITE_READ_LISTS[project.name]);
  let suiteResult = null;
  const newResultDir = () => {
    if (!suiteNamed) return null;
    const dir = mkdtempSync(join(realpathSync(tmpdir()), 'orchestrator-result-'));
    childEnv[SUITE_RESULT_ENV] = dir;
    return dir;
  };
  const collect = dir => {
    if (!dir) return;
    suiteResult = readResult(dir);
    rmSync(dir, { recursive: true, force: true });
  };
  let resultDir = newResultDir();
  // gate-steps.sh appends each step's seconds here (the OS temporary directory, which a Task child
  // writes to as well); read after the run into one reading row per step, then removed.
  let stepTimesDir = null;
  if (env.GATE_READING !== 'off') {
    try {
      stepTimesDir = mkdtempSync(join(realpathSync(tmpdir()), 'gate-step-times-'));
      childEnv.GATE_STEP_TIMES = join(stepTimesDir, 'steps.tsv');
    } catch { stepTimesDir = null; delete childEnv.GATE_STEP_TIMES; }
  } else delete childEnv.GATE_STEP_TIMES;
  const readStepTimes = () => {
    if (!stepTimesDir) return '';
    try { return readFileSync(childEnv.GATE_STEP_TIMES, 'utf8'); } catch { return ''; }
  };
  const clearStepTimes = () => {
    if (!stepTimesDir) return;
    try { writeFileSync(childEnv.GATE_STEP_TIMES, ''); } catch { /* telemetry */ }
  };
  const slotAttempt = newSlotWaitAttempt();
  if (slotAttempt.path) childEnv.GATE_SLOT_WAIT_RESULT = slotAttempt.path;
  else delete childEnv.GATE_SLOT_WAIT_RESULT;
  let task = null;
  if (taskOn) {
    task = prepareTask(project, {
      root, env, childEnv, args: slotAttempt.argv, evidence, spawn, device: selfId,
      // gate-runner made the reuse decision; the Task is asked to run, never to reuse.
      reuse: { mode: null, enabled: false, selfId: null, dirty: evidence.dirty ?? null, commit: gateCommit },
    });
    if (task.why) {
      write(`gate-task: ark unavailable (${task.why}) — ran direct\n`);
      task = null;
    } else if (suiteNamed && task.dropped.some(key => key === SUITE_RESULT_ENV || key === SUITE_EXPECT_ENV)) {
      // A Task whose child would not receive the result path or the expectation could never be read.
      write(`gate-task: ${task.dropped.filter(key => key === SUITE_RESULT_ENV || key === SUITE_EXPECT_ENV).join(' ')} would not reach the Task child (braces are ark template syntax) — ran direct\n`);
      task = null;
    } else if (task.dropped.length > 0) {
      write(`gate-task: not passed to the Task child (braces are ark template syntax): ${task.dropped.join(' ')}\n`);
    }
  }
  const direct = () => gateSpawn(project, 'bash', [resolve(root, 'genesis/orchestrator/run-local-gate.sh'), ...slotAttempt.argv], {
    cwd: root,
    stdio: 'inherit',
    env: childEnv,
  }, { spawn, env, tap });
  let ran;
  let answer = null;
  try {
    if (task) {
      ran = gateSpawn(project, task.bin, task.argv, { cwd: root, stdio: 'inherit', env: childEnv }, { spawn, env, tap });
      // A stopped Task never falls back to the direct path: the stop ends the gate, not one attempt
      // (it is reported below, once the step-times scratch is cleared).
      answer = ran.tapped?.stopped ? {} : taskAnswer(readTaskStatus(task.status), ran.result.status);
      collect(resultDir);
      resultDir = null;
      // A Task whose builder reported is decided from that result; the direct path runs only when
      // the Task produced no result at all (it never ran the builder).
      if (answer.fallback && suiteResult && ['pass', 'fail'].includes(suiteResult.verdict)) {
        write(`gate-task: ${answer.fallback}, but the named half reported (${suiteResult.verdict}): its result stands, no second attempt\n`);
        answer = { answered: { hit: false } };
      }
      if (answer.fallback) {
        write(`gate-task: ${answer.fallback} — ran direct\n`);
        task = null;
        answer = null;
        // A fresh directory for the second attempt: the first attempt's result never stands in.
        suiteResult = null;
        resultDir = newResultDir();
        clearStepTimes(); // the second attempt's steps alone are the run's steps
        clearSlotWait(slotAttempt);
        ran = direct();
      }
    } else {
      ran = direct();
    }
    collect(resultDir);
  } finally {
    if (resultDir) rmSync(resultDir, { recursive: true, force: true });
  }
  const slotWait = collectSlotWait(slotAttempt);
  const stepTimes = readStepTimes();
  if (stepTimesDir) {
    try { rmSync(stepTimesDir, { recursive: true, force: true }); } catch { /* telemetry */ }
  }
  const { result, tapped } = ran;
  if (tapped?.stopped) return stoppedGate(tapped.stopped);
  if (result.status === GATE_SLOT_BUSY_EXIT) return slotBusyGate(slotWait);
  emitSlotWaitReading(slotWait, 'ran');
  const cost = tapped ? Object.fromEntries(COST_FIELDS.map(k => [k, tapped[k] ?? null])) : undefined;
  const derivation = task ? { ...task.derivation, hit: answer.answered?.hit === true } : undefined;
  if (answer?.answered?.hit) {
    write(`[gate] ${project.name} reused: derivation hit (method ${derivation.method}, input ${derivation.input})\n`);
    const seconds = Number(clock() - started) / 1e9;
    const executionSeconds = Math.max(0, seconds
      - finiteSeconds(reading.leaseWaitedSeconds)
      - finiteSeconds(reading.waitedSeconds));
    recordCycle(project, executionSeconds, 0, { ...cycleEnv, GATE_RUN_KIND: 'reused' }, cycleDeps);
    liveEvent(env, { kind: 'gate-verdict', project: project.name, verdict: { exit: 0, seconds: 0, reused: true }, derivation, status: 'Answered' });
    const completed = complete(0, 'reused', executionSeconds);
    runReadShadow('reused', 'green');
    runGrade('reused', 'green', 'reuse derivation');
    return completed;
  }
  const crateShadowStarted = clock();
  shadowBudget.begin();
  let crateContext = null;
  if (crateShadowEligible && !crateShadowState.dropped && env.GATE_READING !== 'off' && stepTimes.trim()) {
    try {
      crateContext = crateKeyContext({
        root, project, cargo, metadata: narrowMetadata, commit: gateCommit, childEnv,
        readsOf: memberReads, spawn, budget: shadowBudget,
      });
    } catch (error) {
      dropCrateShadow(project.name, error, crateShadowState, write);
    }
  }
  emitStepReadings(project.name, reading, stepTimes, {
    env, collector: readingCollector, crateContext, crateShadowState, write,
  });
  shadowBudget.end();
  started += clock() - crateShadowStarted;
  // The exit status is the verdict; what the tap recognized only classifies it. For a suite's named
  // half the verdict comes from the attempt's written result, never the status alone (suiteDecision).
  const outcome = suiteNamed
    ? suiteDecision({ exit: result.status ?? 1, signal: result.signal ?? null, result: suiteResult, expected: evidence.suite,
      gateSha: gateCommit, headAfter: revParse('HEAD'), dirtyAfter: dirtyAfterRun(root, gateCommit) })
    : { status: result.status ?? 1, record: true };
  const status = outcome.status;
  const seconds = Number(clock() - started) / 1e9;
  const executionSeconds = Math.max(0, seconds
    - finiteSeconds(reading.leaseWaitedSeconds)
    - finiteSeconds(reading.waitedSeconds));
  let evidenceCid = null;
  // The verdict is recorded against the closure — red too; a known-red closure is information.
  // An interrupted Task has no verdict, so nothing is recorded for it. A suite's named half is
  // recorded only on its own bytes, with HEAD and the view's paths unchanged since the start.
  const suiteCheck = outcome;
  if (!suiteCheck.record) write(`[gate] ${project.name} evidence: not recorded (${suiteCheck.why})\n`);
  // Whether a gate was stopped is the tap's call alone (its `stopped`, read from the leader's own exit
  // against the signals the stop sent it; returned above). Any other end, a signal death included
  // (an OOM kill, the RAM guard's shed), records what it reached, as it always did.
  if (tapped?.outputTruncated) {
    write(`[gate] ${project.name}: its output was cut after the recipe exited (a process outside its tree held it open); `
      + 'the verdict is the recipe\'s own exit\n');
  }
  if (evidence.key && evidence.buildRefBin && !answer?.unanswered && suiteCheck.record) {
    const put = recordEvidence(project, evidence.key, {
      success: status === 0,
      seconds: executionSeconds,
      cost,
      commit: gateCommit,
      buildRefBin: evidence.buildRefBin,
      root,
      jobs: cargoEnvOf(childEnv).CARGO_BUILD_JOBS,
      dirty: evidence.dirty,
      nativeKey: evidence.nativeKey,
      env,
      cargoEnv: cargoEnvOf(childEnv),
      spawn,
      warn,
    });
    const note = put.status === 0
      ? `recorded ${String(put.stdout || '').trim().slice(0, 16)}…`
      : `not recorded (${String(put.stderr || '').trim().split('\n').pop() || `exit ${put.status}`})`;
    if (!put.keptGreen) write(`[gate] ${project.name} evidence: ${note}\n`);
    if (put.status === 0 && !put.keptGreen) {
      evidenceCid = String(put.stdout || '').trim().split('\n').pop() || null;
      emitGateVerdict(project, evidence.key, {
        success: status === 0, seconds: executionSeconds, commit: gateCommit, put, root, env, spawn, warn,
      });
    }
  }
  // Cycle time is observed here because this is the one place every gate runs.
  // The ceiling, the reviewer and its charter are declared in measures.yaml.
  // Untapped output is unavailable, not evidence that no tests ran.
  const signals = { ...(tapped?.signals || {}), signal: result.signal, firstRedSeconds: tapped?.firstRedSeconds ?? undefined, cost };
  recordCycle(project, executionSeconds, status, cycleEnv, cycleDeps, signals);
  liveEvent(env, { kind: 'gate-verdict', project: project.name, verdict: {
    exit: status, seconds: Math.round(executionSeconds), failure_class: classifyFailure(status, signals) ?? null,
    failures: tapped?.failures ?? null, overflow: tapped?.overflow || undefined, evidence: evidenceCid || undefined,
  }, ...(task ? {
    derivation,
    status: answer.unanswered ? 'Unanswered' : 'Answered',
    ...(answer.unanswered ? { unanswered: answer.unanswered } : {}),
  } : {}) });
  const completed = complete(status, 'ran', executionSeconds);
  runReadShadow('ran', status === 0 ? 'green' : 'red');
  runGrade('ran', status === 0 ? 'green' : 'red', `ran ${status === 0 ? 'green' : 'red'}`);
  return completed;
}

// The per-project cargo cap travels as one serialized variable; the jobs count
// is part of what a duration means, so it rides on the observation.
function cargoEnvOf(childEnv) {
  try {
    return JSON.parse(childEnv.GATE_CARGO_ENV || '{}');
  } catch {
    return {};
  }
}

/**
 * Cheapest first, so a red that takes seconds to find is not queued behind an hour of compile.
 * Key: an attested gate is 0 s (a forge read); otherwise this workspace's recorded p90; a gate
 * with no recorded run sorts as two minutes. Stable for equal keys: the walker's order stands.
 */
export function orderByCost(projects, { root = ROOT, rows } = {}) {
  if (projects.length < 2) return projects;
  let ledger = rows;
  if (ledger === undefined) {
    try {
      const union = readCycleUnion(root);
      ledger = union.readableLedgers ? union.rows : [];
    } catch { ledger = []; }
  }
  const key = project => project.run?.kind === 'attested'
    ? 0 : (costOf(ledger, { project, subjectFallback: null }).p90 ?? 120);
  return projects.map((project, index) => ({ project, index, cost: key(project) }))
    .sort((a, b) => a.cost - b.cost || a.index - b.index)
    .map(entry => entry.project);
}

function usage() {
  console.error('usage: gate-runner.mjs (--target <project-or-path> | --changed-file-list | --list) [--print] [--names [--order cost]] [--recipe <name>]');
  console.error('  env: GATE_ON_RED=continue|stop (default from pool-policy.json gate_on_red) · GATE_PREFLIGHT=off · GATE_PREFLIGHT_DONE=1 (pre-push ran it)');
  console.error('  env: GATE_ORACLE=rakia|shadow|path (default rakia; without a brit binary every mode is path-only) · BRIT_BIN · GATE_ROOT (fixture repository root)');
  console.error('  env: GATE_RECIPE_KEY=shadow|off (default shadow): a root-just gate prints whether a justfile edit since the integration base left its recipe closure unchanged; it changes no key, reuse decision or record');
  console.error('  env: GATE_INPUTS=shadow|off (default shadow; no on yet): prints, once per push, the borrowed steps whose own buildProcess alone selects them, and per gate when, since the integration base, its step fingerprints moved under all and are unchanged under sources; it changes no selection, key, reuse decision or record');
  console.error('  env: GATE_READ_SHADOW=off (default on) · GATE_READ_SHADOW_BUDGET_MS (default 15000 once per push with GATE_READ_SHADOW_LEDGER, otherwise per invocation): after recorded timing, compares closure movement under two read-audit fixes; it decides nothing, and enablement requires proof for #[path]-included tests/** support, format! joins with variable suffixes, absent paths, and eprfs running and blocking on the same push');
  console.error('  env: GATE_GRADE=off|shadow (default shadow; there is no on): after a gate is recorded, one line says what evidence this device holds for its closure (its own execution apart, signature-checked independent executions, sibling claims with the signature unchecked, a carrier\'s result apart, what was not counted, any disagreement) and what the PROPOSED graded table would say for the class of the change (yes, no, or unexamined: <what>); it shares the read-shadow budget and decides nothing');
  console.error('  env: GATE_READING=off (default on): suppress collection and the one bounded `epr flow reading append` call; the shadow reading never changes selection, reuse, keys or exit status');
  console.error('  env: GATE_CARRY=off|shadow|on (default shadow, read by gate-carry.mjs under pre-push): carry one long one-build-job cargo gate on this device\'s second cargo seat beside the loop when `berth room cargo` admits it; shadow prints what would be carried and why and changes nothing');
}

const isMain = import.meta.url === `file://${process.argv[1]}` ||
  import.meta.url === `file://${resolve(process.argv[1])}`;
// Normal gates reuse the gate-live id (one per standalone run or push). Focused --recipe runs
// deliberately do not open gate-live, so they get one stable id for this runner invocation.
const invocationReadingRunId = isMain
  ? `gate-runner-${new Date().toISOString().replace(/[-:.]/g, '')}-${process.pid}`
  : null;

if (isMain) {
  process.once('exit', () => appendGateReadings(invocationGateReadings));
  const args = process.argv.slice(2);
  const printOnly = args.includes('--print');
  const namesOnly = args.includes('--names');
  const registry = loadGateRegistry(ROOT);
  let projects;
  let changedFiles = null;

  if (args.includes('--list')) {
    for (const project of registry.values()) {
      const how = project.run.kind === 'attested'
        ? `attested:${project.run.attestation.repo}#${project.run.attestation.check}`
        : `${project.run.kind}:${project.run.recipe}`;
      process.stdout.write(`${project.name}\t${project.dir}\t${how}\n`);
    }
    process.exit(0);
  } else if (args.includes('--changed-file-list')) {
    changedFiles = readFileSync(0, 'utf8').split('\n').map(value => value.trim()).filter(Boolean);
    projects = projectsForChanges(ROOT, changedFiles);
  } else {
    const index = args.indexOf('--target');
    if (index < 0 || !args[index + 1]) {
      usage();
      process.exit(2);
    }
    projects = selectGateProjects(registry, args[index + 1]);
    if (!args.includes('--recipe')) projects = withHalves(registry, projects);
  }

  if (projects.length === 0) {
    if (!namesOnly) process.stdout.write('[gate] no manifest-declared projects selected\n');
    process.exit(0);
  }

  const recipeIndex = args.indexOf('--recipe');
  const recipe = recipeIndex >= 0 ? args[recipeIndex + 1] : null;
  if (recipeIndex >= 0 && (!recipe || !/^[A-Za-z0-9_-]+$/.test(recipe) || projects.length !== 1)) {
    console.error('gate-runner: --recipe takes one recipe name and exactly one --target project');
    process.exit(2);
  }
  const orderIndex = args.indexOf('--order');
  if (orderIndex >= 0 && args[orderIndex + 1] === 'cost') projects = orderByCost(projects);
  if (namesOnly || printOnly || recipe) {
    let failed = 0;
    let refused = false;
    for (const project of projects) {
      const status = runProject(project, printOnly, namesOnly, { recipe });
      if (status === GATE_SLOT_BUSY_EXIT) refused = true;
      failed |= status;
    }
    process.exit(refused ? GATE_SLOT_BUSY_EXIT : (failed ? 1 : 0));
  }

  // A standalone `just gate` opens its own live run so its reds reach the session the way a
  // push's do; under pre-push the hook's run is inherited.
  let ownRun = null;
  if (!process.env.GATE_LIVE_RUN) {
    try {
      const head = spawnSync('git', ['-C', ROOT, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).stdout.trim() || null;
      ownRun = openRun({ root: ROOT, sha: head, target: 'just gate', session: resolveSession() }).path;
      process.env.GATE_LIVE_RUN = ownRun;
      // A signal or an uncaught error still closes the run this process opened, never leaving it open.
      const interrupted = code => () => { closeRun(ownRun, 'interrupted'); process.exit(code); };
      process.once('SIGINT', interrupted(130));
      process.once('SIGTERM', interrupted(143));
      process.once('SIGHUP', interrupted(129));
      process.once('uncaughtException', error => { console.error(error); closeRun(ownRun, 'failed'); process.exit(1); });
      process.stdout.write(`[gate] live failures: node genesis/orchestrator/gate-live.mjs follow --run ${ownRun}\n`);
    } catch { ownRun = null; }
  }
  const policy = onRedPolicy({ root: ROOT });
  let failed = 0;
  let stopping = null;
  let refused = false;
  if (process.env.GATE_PREFLIGHT !== 'off' && process.env.GATE_PREFLIGHT_DONE !== '1') {
    const rows = runPreflight({ root: ROOT, projects, registry, changed: changedFiles, onRow: row => {
      process.stdout.write(`${formatRow(row)}\n`);
      emitLive([row]);
    } });
    if (rows.some(row => row.level === 'REFUSED')) {
      failed = 1;
      if (policy.mode === 'stop') stopping = `preflight red under GATE_ON_RED=stop (${policy.from})`;
    }
  }
  const notRun = [];
  const ordered = orderByCost(projects);
  // Every gate in this process draws from one telemetry budget; a spent budget starts no later
  // shadow subprocess and yields one skip line per remaining eligible gate.
  const readShadowBudget = readShadowPushBudget(process.env, () => process.hrtime.bigint());
  if (failed && process.env.GATE_LIVE_RUN) await outlook(process.env.GATE_LIVE_RUN, ordered.map(p => p.name), { root: ROOT });
  for (const [index, project] of ordered.entries()) {
    if (!stopping && process.env.GATE_LIVE_RUN && stopRequested(process.env.GATE_LIVE_RUN)) {
      stopping = 'stop requested on the live run';
      // The run's own record that it took the stop, which `gate-live.mjs stop` waits for.
      if (ownRun) appendEvent(ownRun, { kind: 'stop-honoured', where: 'between-gates', requests: liveStopIds(ownRun) });
    }
    if (stopping) { notRun.push(project.name); continue; }
    // Unless pre-push already set it, the gates after this one, for the tap's mid-gate outlook.
    if (ownRun || process.env.GATE_PREFLIGHT_DONE !== '1') {
      process.env.GATE_REMAINING = ordered.slice(index + 1).map(p => p.name).join(' ');
    }
    const status = runProject(project, false, false, { readShadowBudget });
    if (status === GATE_SLOT_BUSY_EXIT) {
      failed = 1;
      refused = true;
      stopping = `${project.name} slot busy (gate did not run)`;
      continue;
    }
    failed |= status;
    if (status !== 0 && process.env.GATE_LIVE_RUN) {
      await outlook(process.env.GATE_LIVE_RUN, ordered.slice(index + 1).map(p => p.name), { root: ROOT });
    }
    if (status !== 0 && policy.mode === 'stop') stopping = `${project.name} red under GATE_ON_RED=stop (${policy.from})`;
  }
  // A gate that did not run is not a pass: a stopped run never exits 0.
  if (notRun.length) {
    process.stdout.write(`[gate] stopped: ${stopping}; not run: ${notRun.join(' ')}\n`);
    failed = 1;
  } else if (process.env.GATE_LIVE_RUN && stopRequested(process.env.GATE_LIVE_RUN)) {
    // A stop that came after the last gate started still refuses: every verdict reached stands.
    if (ownRun) appendEvent(ownRun, { kind: 'stop-honoured', where: 'after-gates', requests: liveStopIds(ownRun) });
    process.stdout.write('[gate] stopped: a stop was requested on the live run after the last gate started; the run is refused\n');
    failed = 1;
  }
  if (ownRun) {
    closeRun(ownRun, refused ? 'interrupted' : (failed ? 'failed' : 'passed'));
    for (const line of formatHarvest(harvest({ root: ROOT, last: 10 }))) process.stdout.write(`${line}\n`);
  }
  // Let Node drain stdout/stderr before ending. The recipe's inherited stdout is unbuffered, while
  // the runner's own NOT RUN / gate heading can otherwise be lost when capture uses a pipe.
  process.exit(refused ? GATE_SLOT_BUSY_EXIT : (failed ? 1 : 0));
}
