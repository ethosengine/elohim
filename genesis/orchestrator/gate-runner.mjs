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
import { rakiaAffected, resolveRakiaBin } from './gate-oracle.mjs';
import { runAttested } from './gate-attest.mjs';

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
// manifest left undeclared. This lets pool-policy carry today's cap (schema-pinned
// out of the manifest) without silently overriding a project that later declares
// its own value once the rakia schema is widened.
export function gateChildEnv(project, baseEnv, root = ROOT) {
  const policy = loadPoolPolicy(root);
  const override = (policy.cargo_env_overrides || {})[project.name] || {};
  const declared = { ...override, ...((project.run.cargo || {}).env || {}) };
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

  process.stdout.write(`\n[gate] ${project.name} (${project.dir})\n`);
  const childEnv = gateChildEnv(project, process.env);
  const started = process.hrtime.bigint();
  const result = spawnSync('bash', [resolve(ROOT, 'genesis/orchestrator/run-local-gate.sh'), ...args], {
    cwd: ROOT,
    stdio: 'inherit',
    env: childEnv,
  });
  const status = result.status ?? 1;
  const seconds = Number(process.hrtime.bigint() - started) / 1e9;
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
  console.error('  env: GATE_ORACLE=shadow|rakia|path (default shadow) · RAKIA_BIN · GATE_ROOT (fixture repository root)');
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
