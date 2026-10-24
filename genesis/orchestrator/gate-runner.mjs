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
import { recordCycle } from './gate-cycle.mjs';
import { gateEvidence, recordEvidence } from './gate-evidence.mjs';
import { rakiaAffected, resolveRakiaBin } from './gate-oracle.mjs';
import { runAttested } from './gate-attest.mjs';
import { rawCid } from './scripts/brit-attest.mjs';

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

  if (mode !== 'path') {
    const ask = opts.rakia || (() => rakiaAffected(root, files, { rakiaBin: resolveRakiaBin(env) }));
    const stale = ask();
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

  return chosen.map(project => {
    const registered = registry.get(project.name);
    if (!registered) throw new Error(`Detected unregistered gate project: ${project.name}`);
    return { ...registered, reasons: project.reasons };
  });
}

// The rakia-validated manifest schema accepts `run.cargo.env` since rakia 2b2cedb
// (2026-09-23); a project's cap declares on its own manifest (elohim-storage does).
// genesis/agentic/pool-policy.json's `cargo_env_overrides` remains for projects that
// have not moved theirs yet. Read once per call; a missing/malformed file is not
// fatal to the gate.
function loadPoolPolicy(root) {
  try {
    return JSON.parse(readFileSync(resolve(root, 'genesis/agentic/pool-policy.json'), 'utf8'));
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

// These notes are local evidence, not a grant or a release head. Only opted-in gates use them.
// All failures (including an old epr without the ledger slots) mean run the gate again.
export function receiptKey(project, { root = ROOT, env = process.env, run = spawnSync } = {}) {
  const cargo = cargoEnvOf(gateChildEnv(project, env, root));
  if (cargo.EPR_GATE_RECEIPT !== '1' || !cargo.RUSTUP_TOOLCHAIN) return null;
  const git = args => {
    const result = run('git', args, { cwd: root, env, encoding: 'utf8', timeout: 10000 });
    if (result.status !== 0) throw new Error('receipt git read failed');
    return result.stdout;
  };
  try {
    // Include untracked files. Never certify an in-progress tree or a different checked-out tree.
    if (git(['status', '--porcelain', '--untracked-files=all', '--', project.dir]).trim()) return null;
    const commit = env.GATE_COMMIT || 'HEAD';
    const tree = git(['rev-parse', `${commit}:${project.dir}`]).trim();
    if (tree !== git(['rev-parse', `HEAD:${project.dir}`]).trim()) return null;
    const manifest = `${project.dir}/build-manifest.json`;
    const bytes = git(['show', `${commit}:${manifest}`]);
    if (bytes !== readFileSync(resolve(root, manifest), 'utf8')) return null;
    return { tree, toolchain: cargo.RUSTUP_TOOLCHAIN, manifest: rawCid(Buffer.from(bytes)) };
  } catch { return null; }
}

export function matchingReceipt(key, ledger) {
  if (!key || !Array.isArray(ledger?.entries)) return null;
  for (const entry of ledger.entries.toReversed()) {
    if (entry.kind !== 'observation' || entry.via || !/^baf[a-z2-7]+$/.test(entry.cid)) continue;
    const slots = entry.extra;
    if (!Array.isArray(slots) || !slots.includes('measure:gate-receipt@1') || !slots.includes('value:1')) continue;
    if (Object.entries(key).every(([name, value]) => slots.filter(s => s.startsWith(`env:${name}=`)).length === 1
      && slots.includes(`env:${name}=${value}`))) return entry.cid;
  }
  return null;
}

export function findReceipt(project, key, { root = ROOT, env = process.env, run = spawnSync } = {}) {
  if (!key) return null;
  try {
    const result = run(env.EPR_BIN || 'epr', ['flow', 'ledger', `${project.dir}/build-manifest.json`, '--json', '--root', root], {
      cwd: root, env, encoding: 'utf8', timeout: 10000, maxBuffer: 4 * 1024 * 1024,
    });
    return result.status === 0 ? matchingReceipt(key, JSON.parse(result.stdout)) : null;
  } catch { return null; }
}

export function writeReceipt(project, before, { root = ROOT, env = process.env, run = spawnSync } = {}) {
  const after = receiptKey(project, { root, env, run });
  if (!before || JSON.stringify(before) !== JSON.stringify(after)) return null;
  const args = ['flow', 'note', '--kind', 'observation', '--measure', 'gate-receipt@1',
    '--subject', `${project.dir}/build-manifest.json`, '--value', '1', '--unit', 'passes', '--json', '--root', root];
  for (const [key, value] of Object.entries(before)) args.push('--env', `${key}=${value}`);
  try {
    const result = run(env.EPR_BIN || 'epr', args, { cwd: root, env, encoding: 'utf8', timeout: 10000 });
    return result.status === 0 ? JSON.parse(result.stdout).record_cid : null;
  } catch { return null; }
}

function runProject(project, printOnly, namesOnly) {
  const cargo = project.run.cargo || {};
  const args = [
    ROOT,
    project.name,
    project.dir,
    project.run.kind,
    project.run.recipe,
    cargo.workspace || '',
    worktreeTargetDir(cargo.targetDir || ''),
    cargo.profile || 'dev',
    Object.hasOwn(cargo, 'rustflags') ? cargo.rustflags : '__inherit__',
  ];

  if (namesOnly) {
    process.stdout.write(`${project.name}\n`);
    return 0;
  }
  if (printOnly) {
    // resolvedCargoEnv is the same manifest∪pool-policy merge gateChildEnv would send
    // to run-local-gate.sh — a dry-run rail so `--print` proves the cap without
    // claiming the cargo berth (no spawnSync).
    const resolvedEnv = gateChildEnv(project, {}).GATE_CARGO_ENV;
    const resolvedCargoEnv = resolvedEnv ? JSON.parse(resolvedEnv) : {};
    process.stdout.write(`${JSON.stringify({ name: project.name, dir: project.dir, run: project.run, resolvedCargoEnv, reasons: project.reasons || [] })}\n`);
    return 0;
  }

  if (project.run.kind === 'attested') {
    // No local recipe: the pinned commit's own upstream attestation is the gate.
    // Never reaches run-local-gate.sh (which keeps refusing unknown kinds).
    process.stdout.write(`\n[gate] ${project.name} (${project.dir}) — attested, no local recipe\n`);
    return runAttested(project, {
      root: ROOT,
      env: process.env,
      log: line => process.stdout.write(`${line}\n`),
      runEpr: eprArgs => spawnSync(process.env.EPR_BIN || 'epr', eprArgs, { cwd: ROOT, stdio: 'ignore', timeout: 30000 }).status,
    });
  }

  // Explicit crate gates carry the same style floor as changed-project selection.
  if (project.run.cargo && project.name !== 'rust-style') {
    const styleArgs = [resolve(ROOT, 'genesis/orchestrator/rust-style.sh'), '--scope', project.dir];
    if (process.env.GATE_COMMIT) styleArgs.push('--head', process.env.GATE_COMMIT);
    const style = spawnSync('bash', styleArgs, {
      cwd: ROOT, stdio: 'inherit', env: gateChildEnv(project, process.env),
    });
    if (style.status !== 0) return style.status || 2;
  }

  const receipt = receiptKey(project);
  if (process.env.GATE_RUN_KIND === 'pre-push') {
    const accepted = findReceipt(project, receipt);
    if (accepted) {
      process.stdout.write(`[gate] ${project.name} attested: receipt ${accepted}\n`);
      return 0;
    }
  }
  process.stdout.write(`\n[gate] ${project.name} (${project.dir})\n`);
  const childEnv = gateChildEnv(project, process.env);
  // Evidence by input closure (gate-evidence.mjs): has THIS closure — the step's declared
  // input bytes at the pushed commit, the recipe, the resolved cargo env — already been
  // gated, by whom, and when? REPORT-ONLY in this slice: the line is printed and the gate
  // runs regardless. GATE_COMMIT is the commit being pushed (pre-push exports it); a human
  // running `just gate` is asking about HEAD. Fail-open: an exception here is one line.
  const gateCommit = (process.env.GATE_COMMIT || '').trim() || 'HEAD';
  let evidence = { line: 'unavailable (evidence lookup threw)' };
  try {
    evidence = gateEvidence(project, { env: process.env, root: ROOT, commit: gateCommit, cargoEnv: cargoEnvOf(childEnv) });
  } catch (error) {
    evidence = { line: `unavailable (${String(error?.message || error).split('\n')[0]})` };
  }
  process.stdout.write(`[gate] ${project.name} evidence: ${evidence.line}\n`);
  const started = process.hrtime.bigint();
  const result = spawnSync('bash', [resolve(ROOT, 'genesis/orchestrator/run-local-gate.sh'), ...args], {
    cwd: ROOT,
    stdio: 'inherit',
    env: childEnv,
  });
  const status = result.status ?? 1;
  const seconds = Number(process.hrtime.bigint() - started) / 1e9;
  if (status === 0 && cargoEnvOf(childEnv).EPR_GATE_RECEIPT === '1') {
    const cid = writeReceipt(project, receipt);
    process.stdout.write(`[gate] ${project.name} receipt: ${cid || 'not written (dirty, changed, or evaluator unavailable)'}\n`);
  }
  // The verdict is recorded against the closure — red too; a known-red closure is information.
  if (evidence.key && evidence.buildRefBin) {
    const put = recordEvidence(project, evidence.key, {
      success: status === 0,
      seconds,
      commit: gateCommit,
      buildRefBin: evidence.buildRefBin,
      root: ROOT,
      jobs: cargoEnvOf(childEnv).CARGO_BUILD_JOBS,
      dirty: evidence.dirty,
    });
    const note = put.status === 0
      ? `recorded ${String(put.stdout || '').trim().slice(0, 16)}…`
      : `not recorded (${String(put.stderr || '').trim().split('\n').pop() || `exit ${put.status}`})`;
    process.stdout.write(`[gate] ${project.name} evidence: ${note}\n`);
  }
  // Cycle time is observed here because this is the one place every gate runs.
  // The ceiling, the reviewer and its charter are declared in measures.yaml.
  recordCycle(project, seconds, status, { ...process.env, ...cargoEnvOf(childEnv) }, {
    root: ROOT,
    measuresPath: resolve(ROOT, '.claude/epr-meta/measures.yaml'),
    ledgerPath: resolve(ROOT, '.claude/data/architecture-findings.jsonl'),
    runEpr: eprArgs => spawnSync(process.env.EPR_BIN || 'epr', eprArgs, { cwd: ROOT, stdio: 'ignore', timeout: 30000 }).status,
    print: line => process.stdout.write(`${line}\n`),
    now: () => new Date().toISOString().replace(/\.\d+Z$/, 'Z'),
  });
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

function usage() {
  console.error('usage: gate-runner.mjs (--target <project-or-path> | --changed-file-list | --list) [--print] [--names]');
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

  if (args.includes('--list')) {
    for (const project of registry.values()) {
      const how = project.run.kind === 'attested'
        ? `attested:${project.run.attestation.repo}#${project.run.attestation.check}`
        : `${project.run.kind}:${project.run.recipe}`;
      process.stdout.write(`${project.name}\t${project.dir}\t${how}\n`);
    }
    process.exit(0);
  } else if (args.includes('--changed-file-list')) {
    const files = readFileSync(0, 'utf8').split('\n').map(value => value.trim()).filter(Boolean);
    projects = projectsForChanges(ROOT, files);
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

  let failed = 0;
  for (const project of projects) failed |= runProject(project, printOnly, namesOnly);
  process.exit(failed ? 1 : 0);
}
