#!/usr/bin/env node
// One local gate path for humans and pre-push. Detection and execution both
// come from build-manifest.json; no project-name command switch lives here.

import { readFileSync } from 'fs';
import { spawnSync } from 'child_process';
import { resolve, dirname, basename } from 'path';
import { fileURLToPath } from 'url';
import { loadManifests } from './manifest-utils.mjs';
import { loadGateRegistry } from './pipeline-registry.mjs';
import { projectsFromStale, walkGraph } from './graph-walker.mjs';
import { filterChanged } from './ci-ignore.mjs';
import { COST_FIELDS, classifyFailure, recordCycle, subjectFor } from './gate-cycle.mjs';
import { CI_BRANCH_PATTERN, costOf, deferDecision, formatExpectation, readCycleUnion } from './gate-cost.mjs';
import {
  ageOf, emitGateVerdict, gateEvidence, recordEvidence, reusableHit, selfAgentId,
  siblingGovernanceChange, siblingVerdicts,
} from './gate-evidence.mjs';
import { rakiaAffected, resolveRakiaBin } from './gate-oracle.mjs';
import { CLAIMED_EXIT_CODE, runAttested } from './gate-attest.mjs';
import { affectedPackages, loadMetadata } from './cargo-affected.mjs';
import { readSet } from './gate-input-audit.mjs';
import { appendEvent, closeRun, formatHarvest, harvest, openRun, outlook, resolveSession, stopRequested } from './gate-live.mjs';
import { emitLive, formatRow, onRedPolicy, runPreflight } from './gate-preflight.mjs';
import { prepareTask, readTaskStatus, taskAnswer, taskEnabled } from './gate-task.mjs';

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

export function projectsForChanges(root, changedFiles, opts = {}) {
  const env = opts.env || process.env;
  const mode = opts.oracle || oracleMode(env);
  // Diagnostics go to STDERR: the pre-push hook parses this command's stdout as
  // project names (`--names`), so a line on stdout becomes a bogus gate target.
  const log = opts.log || (line => process.stderr.write(`${line}\n`));
  const manifests = loadManifests(root);
  const registry = loadGateRegistry(root);
  const files = filterChanged(changedFiles);

  const byPath = walkGraph(manifests, files).projects;
  let chosen = byPath;
  let oracleAvailable = false;

  if (mode !== 'path') {
    const ask = opts.rakia || (() => rakiaAffected(root, files, { rakiaBin: resolveRakiaBin(env), spawn: opts.spawn || spawnSync }));
    const stale = ask();
    oracleAvailable = stale !== null;
    if (stale === null) {
      // Said in both modes: in shadow mode an absent binary would otherwise mean the
      // flip evidence silently never arrives.
      log('[gate] rakia unavailable — path-only selection');
    } else {
      const byOracle = projectsFromStale(manifests, stale, files);
      if (mode === 'rakia') {
        chosen = byOracle;
      } else {
        const diff = oracleDiffLine(byPath, byOracle);
        if (diff) log(diff);
      }
    }
  }
  // Keep the array contract; reuse callers also need to know whether selection fell back.
  opts.onOracle?.(oracleAvailable);

  return chosen.map(project => {
    const registered = registry.get(project.name);
    if (!registered) throw new Error(`Detected unregistered gate project: ${project.name}`);
    return { ...registered, reasons: project.reasons };
  });
}

/** Reuse needs the dependency oracle for changes outside the attested closure too. */
export function reuseSelector(project, gateCommit, { root = ROOT, env = process.env, spawn = spawnSync } = {}) {
  return commit => {
    let diff;
    try {
      if (!commit) return { ok: false, message: 'attested commit unreadable' };
      diff = spawn('git', ['diff', '--name-only', commit, gateCommit], { cwd: root, encoding: 'utf8' });
      if (diff.status !== 0) return { ok: false, message: 'attested commit unreadable' };
    } catch {
      return { ok: false, message: 'attested commit unreadable' };
    }
    const files = diff.stdout.split('\n').filter(Boolean);
    if (files.length === 0) return { ok: true };
    try {
      let oracleAvailable = false;
      const projects = projectsForChanges(root, files, {
        env, spawn, oracle: 'rakia', onOracle: available => { oracleAvailable = available; },
      });
      if (!oracleAvailable) return { ok: false, message: 'rakia oracle unavailable; reuse needs it' };
      if (projects.some(p => p.name === project.name)) {
        return { ok: false, message: `changed since attested commit and selects this gate: ${files[0]}` };
      }
      return { ok: true };
    } catch {
      return { ok: false, message: 'rakia oracle unavailable; reuse needs it' };
    }
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
export function gateChildEnv(project, baseEnv, root = ROOT) {
  const policy = loadPoolPolicy(root);
  const overrides = policy.cargo_env_overrides || {};
  // "*" is the all-cargo-projects default (the local gate's RUSTUP_TOOLCHAIN pin); only a
  // project that goes through the cargo path (declares run.cargo) receives it.
  const shared = project.run.cargo ? overrides['*'] || {} : {};
  const declared = { ...shared, ...(overrides[project.name] || {}), ...((project.run.cargo || {}).env || {}) };
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

/**
 * Run a gate command through gate-tap.mjs, which forwards its bytes unchanged, keeps its
 * exit status, appends failure lines to the live run (GATE_LIVE_RUN) as they print, and
 * returns the classifyFailure signals on fd 3. `tap: false` (or GATE_TAP=off) runs the
 * command directly, as an injected spawn in a fixture does.
 */
function gateSpawn(project, command, argv, options, { spawn, env, tap }) {
  if (!tap || env.GATE_TAP === 'off') return { result: spawn(command, argv, options), tapped: null };
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

export function runProject(project, printOnly, namesOnly, {
  root = ROOT, env = process.env, spawn = spawnSync,
  write = text => process.stdout.write(text), warn = text => process.stderr.write(text),
  readsOf, recipe = null, tap = spawn === spawnSync,
} = {}) {
  const cargo = project.run.cargo || {};
  const targetDir = worktreeTargetDir(cargo.targetDir || '', root);
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

  if (recipe) {
    if (project.run.kind === 'attested') {
      process.stderr.write(`gate ${project.name}: an attested project has no local recipe to run\n`);
      return 2;
    }
    write(`\n[gate] ${project.name} (${project.dir}) — recipe ${recipe}\n`);
    return spawn('bash', [resolve(root, 'genesis/orchestrator/run-local-gate.sh'), ...args], {
      cwd: root,
      stdio: 'inherit',
      env: gateChildEnv(project, env, root),
    }).status ?? 1;
  }

  if (project.run.kind === 'attested') {
    // No local recipe: the pinned commit's own upstream attestation is the gate.
    // Never reaches run-local-gate.sh (which keeps refusing unknown kinds).
    write(`\n[gate] ${project.name} (${project.dir}) — attested, no local recipe\n`);
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
      return 0;
    }
    liveEvent(env, { kind: 'gate-verdict', project: project.name, verdict: { exit: attestedStatus, seconds: 0 } });
    return attestedStatus;
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
  const childEnv = gateChildEnv(project, env, root);
  const started = process.hrtime.bigint();
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
    const seconds = Number(process.hrtime.bigint() - started) / 1e9;
    recordCycle(project, seconds, 0, { ...cycleEnv, GATE_RUN_KIND: 'deferred' }, cycleDeps);
    liveEvent(env, { kind: 'gate-verdict', project: project.name, verdict: { exit: 0, seconds: 0, deferred: true } });
    return 0;
  }

  // Explicit crate gates carry the same style floor as changed-project selection.
  liveEvent(env, { kind: 'gate-start', project: project.name });
  if (project.run.cargo && project.name !== 'rust-style') {
    const styleArgs = [resolve(root, 'genesis/orchestrator/rust-style.sh'), '--scope', project.dir];
    if (env.GATE_COMMIT) styleArgs.push('--head', env.GATE_COMMIT);
    const { result: style } = gateSpawn(project, 'bash', styleArgs, {
      cwd: root, stdio: 'inherit', env: gateChildEnv(project, env, root),
    }, { spawn, env, tap });
    if (style.status !== 0) {
      liveEvent(env, { kind: 'gate-verdict', project: project.name, step: 'rust-style', verdict: { exit: style.status || 2, seconds: 0 } });
      return style.status || 2;
    }
  }

  // A selection belongs to this project and this push, never to the caller's environment.
  delete childEnv.GATE_CARGO_PACKAGES;
  const narrowProjects = loadPoolPolicy(root).gate_narrow?.projects;
  if (Array.isArray(narrowProjects) && narrowProjects.includes(project.name)) {
    const mode = ['off', 'shadow', 'on'].includes(env.GATE_NARROW) ? env.GATE_NARROW : 'shadow';
    let selection = { packages: null, why: 'no push base' };
    let metadata = null;
    let files = null;
    if (env.GATE_NARROW_BASE === 'ok' && env.GATE_CHANGED_FILES) {
      try { files = readFileSync(env.GATE_CHANGED_FILES, 'utf8').split('\n').filter(Boolean); }
      catch { /* An unreadable push range must keep the full workspace. */ }
    }
    if (files && mode === 'off') selection.why = 'narrowing disabled';
    else if (files) {
      try {
        const workspaceDir = cargo.workspace || project.dir;
        const metadataEnv = { ...childEnv, ...cargoEnvOf(childEnv), RUSTC_WRAPPER: '' };
        delete metadataEnv.CARGO_TARGET_DIR;
        if (Object.hasOwn(cargo, 'rustflags')) metadataEnv.RUSTFLAGS = cargo.rustflags;
        metadata = loadMetadata({ root, workspaceDir,
          spawn: (command, argv, options) => spawn(command, argv, { ...options, env: metadataEnv }),
        });
        selection = affectedPackages({ root, workspaceDir, files, metadata,
          readsOf: readsOf || (dir => readSet({ root, sources: [`${dir}/**`] }).paths),
        });
      } catch (error) {
        selection = { packages: null, why: `narrowing unavailable (${String(error?.message || error).split('\n')[0]})` };
      }
    }
    const { packages, why } = selection;
    write(`[gate] ${project.name} narrow (${mode}): ${packages === null
      ? `full workspace — ${why}`
      : `${packages.join(' ')} (${packages.length} of ${metadata.workspace_members.length} members)`}\n`);
    if (mode === 'on' && packages !== null) childEnv.GATE_CARGO_PACKAGES = packages.join(' ');
  }
  // Evidence by input closure (gate-evidence.mjs): has THIS closure — the step's declared
  // input bytes at the pushed commit, the recipe, the resolved cargo env — already been
  // gated, by whom, and when? Reuse requires an audited project and either this workspace's
  // signer or an admitted sibling verdict under D5's stricter fences.
  // GATE_COMMIT is the commit being pushed (pre-push exports it); a human
  // running `just gate` is asking about HEAD. Fail-open: an exception here is one line.
  const gateCommit = (env.GATE_COMMIT || '').trim() || 'HEAD';
  let evidence = { line: 'unavailable (evidence lookup threw)' };
  try {
    evidence = gateEvidence(project, { env: childEnv, root, commit: gateCommit, cargoEnv: cargoEnvOf(childEnv), spawn, warn });
  } catch (error) {
    evidence = { line: `unavailable (${String(error?.message || error).split('\n')[0]})` };
  }
  write(`[gate] ${project.name} evidence: ${evidence.line}\n`);
  const enabled = loadPoolPolicy(root).gate_reuse?.projects;
  // A Task (gate-task.mjs) carries the same reuse rules into ark's derivation look-up, which
  // reads brit itself; the gate then runs through ark only on a miss.
  let task = null;
  // The signer is asked of the binary gateEvidence resolved from childEnv, never one from process.env.
  const signerOptions = { env: childEnv, buildRefBin: evidence.buildRefBin };
  if (taskEnabled(project, env)) {
    const selfId = selfAgentId(root, spawn, signerOptions);
    task = prepareTask(project, {
      root, env, childEnv, args, evidence, spawn, device: selfId,
      reuse: {
        mode: env.GATE_EVIDENCE || null,
        enabled: Array.isArray(enabled) && enabled.includes(project.name),
        selfId: env.GATE_EVIDENCE === 'reuse' ? selfId : null,
        dirty: evidence.dirty ?? null,
        commit: gateCommit,
      },
    });
    if (task.why) {
      write(`gate-task: ark unavailable (${task.why}) — ran direct\n`);
      task = null;
    } else if (task.dropped.length > 0) {
      write(`gate-task: not passed to the Task child (braces are ark template syntax): ${task.dropped.join(' ')}\n`);
    }
  }
  const reuseContext = {
    mode: env.GATE_EVIDENCE,
    enabled: Array.isArray(enabled) && enabled.includes(project.name),
    selfId: env.GATE_EVIDENCE === 'reuse' ? selfAgentId(root, spawn, signerOptions) : null,
    selectorCheck: reuseSelector(project, gateCommit, { root, env: childEnv, spawn }),
  };
  let decision = task ? { reuse: false, why: 'decided by the Task' }
    : reusableHit(evidence, reuseContext);
  // Preserve the local brit path exactly. Only a local miss asks the device channel, and the
  // native read has already removed self, revoked and unadmitted devices.
  if (!task && !decision.reuse && env.GATE_EVIDENCE === 'reuse'
      && reuseContext.enabled && reuseContext.selfId && evidence.key) {
    const sibling = siblingVerdicts(project, evidence.key, { root, env, spawn });
    if (sibling.candidates.length > 0) {
      const governance = siblingGovernanceChange({ root, spawn });
      decision = governance.changed
        ? { reuse: false, why: governance.why }
        : reusableHit(evidence, {
          ...reuseContext,
          candidates: sibling.candidates,
          // Admission is established by `epr peer verdicts`; this callback keeps the decision
          // seam explicit and lets alternate candidate sources fail closed.
          admitCandidate: candidate => candidate.source === 'sibling',
        });
    }
  }
  if (decision.reuse) {
    const row = decision.row;
    const sibling = decision.independence === 'sibling';
    const signer = sibling
      ? `signer sibling ${shortDid(row.device)}, independence: sibling`
      : 'signer this workspace';
    write(`[gate] ${project.name} reused: green ${ageOf(sibling ? row.at : row.attestation.builtAt)} at ${String(row.commit).slice(0, 9)} (${signer})\n`);
    const seconds = Number(process.hrtime.bigint() - started) / 1e9;
    recordCycle(project, seconds, 0, {
      ...cycleEnv, GATE_RUN_KIND: sibling ? 'sibling-reused' : 'reused',
    }, cycleDeps);
    liveEvent(env, { kind: 'gate-verdict', project: project.name, verdict: { exit: 0, seconds: 0, reused: true } });
    return 0;
  }
  if (env.GATE_EVIDENCE === 'reuse' && !task) write(`[gate] ${project.name} not reused: ${decision.why}\n`);
  const direct = () => gateSpawn(project, 'bash', [resolve(root, 'genesis/orchestrator/run-local-gate.sh'), ...args], {
    cwd: root,
    stdio: 'inherit',
    env: childEnv,
  }, { spawn, env, tap });
  let ran;
  let answer = null;
  if (task) {
    ran = gateSpawn(project, task.bin, task.argv, { cwd: root, stdio: 'inherit', env: childEnv }, { spawn, env, tap });
    answer = taskAnswer(readTaskStatus(task.status), ran.result.status);
    if (answer.fallback) {
      write(`gate-task: ${answer.fallback} — ran direct\n`);
      task = null;
      answer = null;
      ran = direct();
    }
  } else {
    ran = direct();
  }
  const { result, tapped } = ran;
  const cost = tapped ? Object.fromEntries(COST_FIELDS.map(k => [k, tapped[k] ?? null])) : undefined;
  const derivation = task ? { ...task.derivation, hit: answer.answered?.hit === true } : undefined;
  if (answer?.answered?.hit) {
    write(`[gate] ${project.name} reused: derivation hit (method ${derivation.method}, input ${derivation.input})\n`);
    const seconds = Number(process.hrtime.bigint() - started) / 1e9;
    recordCycle(project, seconds, 0, { ...cycleEnv, GATE_RUN_KIND: 'reused' }, cycleDeps);
    liveEvent(env, { kind: 'gate-verdict', project: project.name, verdict: { exit: 0, seconds: 0, reused: true }, derivation, status: 'Answered' });
    return 0;
  }
  // The exit status is the verdict; what the tap recognized only classifies it.
  const status = result.status ?? 1;
  const seconds = Number(process.hrtime.bigint() - started) / 1e9;
  let evidenceCid = null;
  // The verdict is recorded against the closure — red too; a known-red closure is information.
  // An interrupted Task has no verdict, so nothing is recorded for it.
  if (evidence.key && evidence.buildRefBin && !answer?.unanswered) {
    const put = recordEvidence(project, evidence.key, {
      success: status === 0,
      seconds,
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
    write(`[gate] ${project.name} evidence: ${note}\n`);
    if (put.status === 0) {
      evidenceCid = String(put.stdout || '').trim().split('\n').pop() || null;
      emitGateVerdict(project, evidence.key, {
        success: status === 0, seconds, commit: gateCommit, put, root, env, spawn, warn,
      });
    }
  }
  // Cycle time is observed here because this is the one place every gate runs.
  // The ceiling, the reviewer and its charter are declared in measures.yaml.
  // Untapped output is unavailable, not evidence that no tests ran.
  const signals = { ...(tapped?.signals || {}), signal: result.signal, firstRedSeconds: tapped?.firstRedSeconds ?? undefined, cost };
  recordCycle(project, seconds, status, cycleEnv, cycleDeps, signals);
  liveEvent(env, { kind: 'gate-verdict', project: project.name, verdict: {
    exit: status, seconds: Math.round(seconds), failure_class: classifyFailure(status, signals) ?? null,
    failures: tapped?.failures ?? null, overflow: tapped?.overflow || undefined, evidence: evidenceCid || undefined,
  }, ...(task ? {
    derivation,
    status: answer.unanswered ? 'Unanswered' : 'Answered',
    ...(answer.unanswered ? { unanswered: answer.unanswered } : {}),
  } : {}) });
  return status;
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
  console.error('  env: GATE_ORACLE=rakia|shadow|path (default rakia; without a rakia binary every mode is path-only) · RAKIA_BIN · GATE_ROOT (fixture repository root)');
}

const isMain = import.meta.url === `file://${process.argv[1]}` ||
  import.meta.url === `file://${resolve(process.argv[1])}`;

if (isMain) {
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
    for (const project of projects) failed |= runProject(project, printOnly, namesOnly, { recipe });
    process.exit(failed ? 1 : 0);
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
  if (failed && process.env.GATE_LIVE_RUN) await outlook(process.env.GATE_LIVE_RUN, ordered.map(p => p.name), { root: ROOT });
  for (const [index, project] of ordered.entries()) {
    if (!stopping && process.env.GATE_LIVE_RUN && stopRequested(process.env.GATE_LIVE_RUN)) stopping = 'stop requested on the live run';
    if (stopping) { notRun.push(project.name); continue; }
    // Unless pre-push already set it, the gates after this one, for the tap's mid-gate outlook.
    if (ownRun || process.env.GATE_PREFLIGHT_DONE !== '1') {
      process.env.GATE_REMAINING = ordered.slice(index + 1).map(p => p.name).join(' ');
    }
    const status = runProject(project, false, false, {});
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
  }
  if (ownRun) {
    closeRun(ownRun, failed ? 'failed' : 'passed');
    for (const line of formatHarvest(harvest({ root: ROOT, last: 10 }))) process.stdout.write(`${line}\n`);
  }
  process.exit(failed ? 1 : 0);
}
