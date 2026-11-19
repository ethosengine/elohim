#!/usr/bin/env node
// Seconds-cheap checks for the selected gate set, run before the first gate. Each one is a
// careless miss that has already cost a long gate run its own wall-clock: an uninitialised
// submodule, an unbuilt workspace dist, a Rust read the gate key cannot name, an evidence
// record the dirty tree will throw away. A line per check: `ok`, `REFUSED` (a red: the
// selected gates will fail or the push cannot pass) or `WARN` (the run will pass but pays
// for something it need not). Each REFUSED and WARN names its cure.
//
// Reds are emitted to the open live run (GATE_LIVE_RUN) the moment they are known, so the
// pushing session hears them in seconds. Whether the push then stops is not decided here:
// gate-runner and pre-push read GATE_ON_RED (see onRedPolicy). Exit 1 on any REFUSED.
//
// The fresh-worktree checklist this replaces lived in agent memory and was re-learned on
// 2026-10-15, -17, -19, -22, -27, -31 and 11-01. Habit `careless-reds-cost-seconds`.

import { accessSync, constants, existsSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadGateRegistry } from './pipeline-registry.mjs';
import { SUITE_READ_LISTS, UNAUDITED_RECIPE_PROJECTS, globCovers, readSet, suiteReadSet } from './gate-input-audit.mjs';
import { declaredPinStores } from './suite-inventory.mjs';
import { dirtyPaths } from './gate-evidence.mjs';
import { appendEvent } from './gate-live.mjs';
import { applicableTargets, checkCBaseline, hostTripleOf, readDeclaredTargets } from './c-baseline.mjs';
// gate-runner imports this module too; the cycle is safe because neither calls the other while loading.
import { gateChildEnv } from './gate-runner.mjs';
// Loaded so that a module that fails to load still gives a row, not a stack trace (fleetRollRows).
let fleetGuard = null;
let fleetGuardError = null;
try { fleetGuard = await import('./fleet-roll-guard.mjs'); } catch (error) { fleetGuardError = error; }

const ROOT = process.env.GATE_ROOT
  ? resolve(process.env.GATE_ROOT)
  : resolve(dirname(fileURLToPath(import.meta.url)), '../..');

/** Submodules whose manifests declare attested gate projects; without them the registry is short. */
export const ATTESTED_SUBMODULES = ['elohim/brit', 'elohim/rakia', 'sophia'];

/**
 * Projects whose Rust read set must be fully named (gate-input-audit.test.mjs `REAL … read set`).
 * The orchestrator gate runs that test, so any of these, or the orchestrator itself, selects it.
 * Membership is an explicit read-set qualification, not an implication of dependency on the
 * Rust-style checker. The checker consumes published EPRFS crates and does not read their source tree.
 */
export const NAMED_READ_SET_PROJECTS = ['elohim-storage', 'doorway', 'steward-node', 'elohim-epr'];
// Every path-scoped gate whose key one unnamed read would widen to the whole tree. eprfs keys on
// named paths since 6cba5805a but is audited separately from the list above (whose REAL test also
// asserts the runtime services' shared-crate reads). Without it here, one new test-file read
// (epr-cli/tests/session_envelope.rs, 2026-11-10) put eprfs back on whole-tree keying unseen, and a
// dirty ledger then cost a 47-minute re-run.
// eprfs-live-corpus scans the same sources and keys on the reads its `gate-reads(eprfs-live-corpus)`
// lines name, so one unnamed read there would widen it the same way.
export const PREFLIGHT_READ_SET_PROJECTS = [...NAMED_READ_SET_PROJECTS, 'eprfs', 'eprfs-live-corpus'];

const ELEMENTS_DIR = 'app/elohim-elements';
const STORAGE_CLIENT_DIST = 'elohim/sdk/storage-client-ts/dist/index.js';
const SOPHIA_UMD = [
  'app/elohim-app/src/assets/sophia-plugin/sophia-element.umd.js',
  'sophia/packages/sophia-element/dist/sophia-element.umd.js',
];

/** `continue` (default): every gate still runs and reports; `stop`: end at the first red. */
export function onRedPolicy({ env = process.env, root = ROOT } = {}) {
  const fromEnv = (env.GATE_ON_RED || '').trim();
  if (fromEnv === 'stop' || fromEnv === 'continue') return { mode: fromEnv, from: 'GATE_ON_RED' };
  try {
    const policy = JSON.parse(readFileSync(join(root, 'genesis/agentic/pool-policy.json'), 'utf8'));
    const declared = policy.gate_on_red?.mode;
    if (declared === 'stop' || declared === 'continue') return { mode: declared, from: 'pool-policy.json gate_on_red' };
  } catch { /* An unreadable policy keeps the default. */ }
  return { mode: 'continue', from: 'default' };
}

const row = (check, level, why = '', cure = '') => ({ check, level, why, cure });

// A fresh worktree (a harness-made `.claude/worktrees/agent-<id>` among them, backlog row 25) has
// neither submodules nor node_modules; one command cures both, then the dists and this preflight.
export const PREP_COMMAND = 'just dev prep';
const viaPrep = (root, byHand) => `${PREP_COMMAND}, run in ${root} (or by hand: ${byHand})`;

// Prep executes argv, never shell-parses a human cure. Both views have one source.
export const WORKSPACE_INSTALL = ['pnpm', 'install', '--frozen-lockfile', '--prefer-offline'];
const commandRow = (check, level, why, commands) => ({
  ...row(check, level, why, commands.map(command => command.join(' ')).join(' && ')), commands,
});

function usesNode(project) {
  return !project.run?.cargo && project.run?.kind !== 'attested';
}

function underApp(project) {
  return project.dir === 'app' || project.dir.startsWith('app/') || project.dir.startsWith('doorway/doorway-app');
}

export function checkSubmodules({ root, spawn = spawnSync }) {
  const r = spawn('git', ['-C', root, 'submodule', 'status', '--', ...ATTESTED_SUBMODULES], { encoding: 'utf8' });
  if (r.status !== 0) return row('submodules', 'WARN', 'git submodule status failed', 'git submodule status');
  const missing = String(r.stdout).split('\n').filter(line => line.startsWith('-')).map(line => line.trim().split(/\s+/)[1]);
  if (missing.length) {
    return { ...row('submodules', 'REFUSED', `not checked out: ${missing.join(' ')} — their attested gate projects vanish and the orchestrator registry tests fail`,
      viaPrep(root, `git submodule update --init ${missing.join(' ')}`)), missing };
  }
  const moved = String(r.stdout).split('\n').filter(line => line.startsWith('+')).map(line => line.trim().split(/\s+/)[1]);
  if (moved.length) {
    return row('submodules', 'WARN', `checked-out commit differs from the pin: ${moved.join(' ')} — gates test bytes the push does not carry`,
      `git submodule update ${moved.join(' ')}   (or commit the pin you mean)`);
  }
  return row('submodules', 'ok');
}

/**
 * The orchestrator gate's named half builds its view from the submodule pins its read list declares
 * (suite-reads.json), read from an object store here. Missing one refuses that gate minutes in, so
 * name it now. Found on a second device, 2026-11-19: holochain-conductor's pin was in no store while
 * the attested-three check said `ok`. No row unless the orchestrator project is selected.
 */
export function checkSubmodulePins({ root, projects, env, pins = declaredPinStores }) {
  if (!projects.some(project => project.name === 'orchestrator')) return null;
  let declared;
  try { declared = pins(env ? { root, env } : { root }); }
  catch (error) { return row('submodule-pins', 'WARN', `could not read the declared submodule pins: ${String(error?.message || error).split('\n')[0]}`, 'git submodule status'); }
  const missing = declared.filter(pin => !pin.store);
  if (!missing.length) return row('submodule-pins', 'ok');
  return row('submodule-pins', 'REFUSED',
    `pinned commit in no object store here: ${missing.map(pin => `${pin.path}@${pin.oid.slice(0, 12)}`).join(' ')} — the orchestrator named half builds its view from them and refuses`,
    `git submodule update --init ${missing.map(pin => pin.path).join(' ')}`);
}

/**
 * A temp root inside a git repository nests every fixture repository a test creates there, so
 * `git` in a test resolves the OUTER repository (the Codex sandbox's transient `/tmp/.git`
 * redded eprfs trust tests deep in a run; row 74). Refused in the first second instead.
 */
export function checkTmpdir({ projects, env = process.env, exists = existsSync }) {
  if (!projects.length) return null;
  const tmp = resolve(env.TMPDIR || tmpdir());
  for (let dir = tmp; ; dir = dirname(dir)) {
    if (exists(join(dir, '.git'))) {
      return row('tmpdir', 'REFUSED',
        `TMPDIR ${tmp} is inside the git repository at ${dir} — every fixture repository a test creates there resolves that outer repository`,
        `export TMPDIR=<a directory outside any repository>, or remove a stray ${join(dir, '.git')}`);
    }
    if (dirname(dir) === dir) return row('tmpdir', 'ok');
  }
}

export function checkNodeModules({ root, projects }) {
  const needing = projects.filter(usesNode);
  if (!needing.length) return null;
  if (existsSync(join(root, 'node_modules/.modules.yaml'))) return row('node_modules', 'ok');
  const red = commandRow('node_modules', 'REFUSED', `no installed workspace for ${needing.map(p => p.name).join(' ')}`,
    [WORKSPACE_INSTALL]);
  return { ...red, cure: viaPrep(root, red.cure) };
}

function readJson(path) {
  try { return JSON.parse(readFileSync(path, 'utf8')); } catch { return null; }
}

/** Element packages a project depends on, unless its own prebuild builds them (doorway-app). */
export function elementDepsOf(root, project) {
  const manifest = readJson(join(root, project.dir, 'package.json'));
  if (!manifest) return [];
  if (/\belements\b/.test(String(manifest.scripts?.prebuild || ''))) return [];
  let names = [];
  try { names = readdirSync(join(root, ELEMENTS_DIR)); } catch { return []; }
  const declared = { ...(manifest.dependencies || {}), ...(manifest.devDependencies || {}) };
  return names.filter(name => Object.hasOwn(declared, name)).sort();
}

/**
 * Every JS `exports[*].import` under dist/ of each element package a selected project depends on
 * (elohim-app on 11-01: `Could not resolve "elohim-core/register"`; storybook on 10-31:
 * `elohim-qahal/register`). An element no selected gate imports is never required.
 */
export function checkElementDists({ root, projects }) {
  const needed = [...new Set(projects.flatMap(project => elementDepsOf(root, project)))].sort();
  if (!needed.length) return null;
  const missing = [];
  for (const name of needed) {
    const manifest = readJson(join(root, ELEMENTS_DIR, name, 'package.json'));
    for (const target of Object.values(manifest?.exports || {})) {
      const file = typeof target === 'string' ? target : target?.import;
      // JS module exports only: an unresolvable import is what failed the app and storybook builds.
      // A declared asset (elohim-core's ./dist/styles.css) may have no build step at all, and
      // refusing on it would block a push whose gates pass.
      if (typeof file !== 'string' || !/^\.\/dist\/.+\.m?js$/.test(file)) continue;
      if (!existsSync(join(root, ELEMENTS_DIR, name, file))) { missing.push(name); break; }
    }
  }
  if (!missing.length) return row('element-dists', 'ok');
  return commandRow('element-dists', 'REFUSED', `unbuilt element package(s) a selected gate imports: ${missing.join(' ')}`,
    [['pnpm', ...missing.flatMap(name => ['--filter', name]), 'run', 'build']]);
}

export function checkStorageClientDist({ root, projects }) {
  const a2o = projects.some(p => p.dir.startsWith('genesis/a2o'));
  if (!a2o && !projects.some(underApp)) return null;
  if (existsSync(join(root, STORAGE_CLIENT_DIST))) return row('storage-client-dist', 'ok');
  // genesis-a2o builds nothing first (10-19: type-aware lint read error types; 11-01: a lane of 0
  // scenarios); app prebuilds build it, though lint can run before that.
  return commandRow('storage-client-dist', a2o ? 'REFUSED' : 'WARN', `${STORAGE_CLIENT_DIST} absent — type-aware lint and the a2o lane read it`,
    [['pnpm', '--filter', '@elohim/storage-client...', 'run', 'build']]);
}

/** The elohim dev pool slot the agentic-shift harness falls back to (`just gate elohim-ark` builds it). */
export const ARK_DEV_SLOT = '/projects/.cargo-target-pool/family/dev/elohim/dev/debug/ark';

function isExecutable(path) {
  try { accessSync(path, constants.X_OK); return statSync(path).isFile(); } catch { return false; }
}

/**
 * agentic-shift's harness runs `ark`: $ARK_BIN, else `ark` on PATH, else the elohim dev slot.
 * With none, its tests fail in setUp (26 of them, 2026-10-31; landing queue S4-3).
 */
export function checkArk({ projects, env = process.env, executable = isExecutable }) {
  if (!projects.some(p => p.name === 'agentic-shift')) return null;
  const onPath = (env.PATH || '').split(':').filter(Boolean).some(dir => executable(join(dir, 'ark')));
  if ((env.ARK_BIN && executable(env.ARK_BIN)) || onPath || executable(ARK_DEV_SLOT)) return row('ark', 'ok');
  return row('ark', 'REFUSED', 'no ark binary ($ARK_BIN, ark on PATH, or the elohim dev slot) — the agentic-shift harness fails in setUp',
    'just gate elohim-ark');
}

/** Workspace libraries a gate imports from a built dist/ (nothing builds them before lint). */
export const WORKSPACE_DISTS = [
  { name: '@elohim/epr', dist: 'elohim/sdk/epr-ts/dist/index.js' },
];

function dependsOn(root, project, name) {
  const pkg = readJson(join(root, project.dir, 'package.json'));
  return ['dependencies', 'devDependencies', 'peerDependencies', 'optionalDependencies']
    .some(map => pkg?.[map] && Object.hasOwn(pkg[map], name));
}

/**
 * A selected gate's own package.json names a workspace library whose dist/ is absent: its lint
 * and tests cannot resolve the import (elohim-app, 2026-11-02: `Failed to resolve import
 * "@elohim/epr"`, and type-aware lint misreading every call into it).
 */
export function checkWorkspaceDists({ root, projects }) {
  const missing = WORKSPACE_DISTS.filter(lib =>
    projects.some(project => dependsOn(root, project, lib.name)) && !existsSync(join(root, lib.dist)));
  const needed = WORKSPACE_DISTS.some(lib => projects.some(project => dependsOn(root, project, lib.name)));
  if (!needed) return null;
  if (!missing.length) return row('workspace-dists', 'ok');
  return commandRow('workspace-dists', 'REFUSED', `unbuilt workspace librar${missing.length > 1 ? 'ies' : 'y'} a selected gate imports: ${missing.map(lib => lib.name).join(' ')}`,
    [['pnpm', ...missing.flatMap(lib => ['--filter', lib.name]), 'run', 'build']]);
}

export function checkSophiaUmd({ root, projects }) {
  if (!projects.some(p => p.name === 'elohim-app')) return null;
  if (SOPHIA_UMD.some(path => existsSync(join(root, path)))) return row('sophia-umd', 'ok');
  // check-sophia.sh can still fetch it from Nexus or a GitHub release, so this predicts, never refuses.
  return commandRow('sophia-umd', 'WARN', 'no sophia-element UMD in the app assets or the sophia submodule — elohim-app prebuild must fetch one from the network or fail',
    [['pnpm', '-C', 'sophia', 'install'], ['pnpm', '-C', 'sophia', 'build'],
      ['pnpm', '-C', 'sophia', '--filter', '@ethosengine/sophia-element', 'build:umd']]);
}

export const DIST_CHECKS = [checkElementDists, checkStorageClientDist, checkWorkspaceDists, checkSophiaUmd];

function sourcesOf(root, project) {
  const manifest = JSON.parse(readFileSync(join(root, project.manifestPath), 'utf8'));
  return [...new Set(project.steps.flatMap(step => manifest.steps[step]?.inputs?.sources || []))];
}

/** One readSet per project per preflight; storage alone costs ~7 s. */
function memoRead(read) {
  const cache = new Map();
  return options => {
    if (!cache.has(options.project.name)) cache.set(options.project.name, read(options));
    return cache.get(options.project.name);
  };
}

/**
 * The orchestrator gate's REAL read-set tests, run in seconds instead of at its end. A new
 * unnamed read can only arrive with a changed .rs file or manifest inside a project's sources,
 * so with a known change list only those projects are audited; without one, all are.
 */
export function checkReadSets({ root, projects, registry, changed = null, read = readSet }) {
  const names = new Set(projects.map(p => p.name));
  if (!names.has('orchestrator') && !PREFLIGHT_READ_SET_PROJECTS.some(n => names.has(n)) && !names.has('epr-storage')) return null;
  const relevant = changed === null ? null : changed.filter(f => f.endsWith('.rs') || f.endsWith('build-manifest.json'));
  const problems = [];
  let audited = 0;
  for (const name of PREFLIGHT_READ_SET_PROJECTS) {
    const project = registry.get(name);
    if (!project) continue;
    const sources = sourcesOf(root, project);
    if (relevant !== null && !relevant.some(f => f === project.manifestPath || sources.some(glob => globCovers(glob, f)))) continue;
    audited += 1;
    const result = read({ root, project, sources });
    for (const u of result.unresolved) problems.push(`${name}: ${u.file}:${u.line} ${u.expr}`);
    if (result.paths.includes('.')) problems.push(`${name}: a repository-wide read`);
  }
  if (!problems.length) return row('read-sets', 'ok', audited ? '' : 'no Rust or manifest change reaches a named read set');
  return row('read-sets', 'REFUSED', `${problems.length} read(s) the gate key cannot name — ${problems.slice(0, 3).join('; ')}`,
    'declare each with `// gate-reads: <repo-relative path>` beside the read; check: node genesis/orchestrator/gate-input-audit.mjs <project>');
}

/** Moved from after ALL CLEAR, keeping its scope: any non-empty selection, whenever installed. */
export function checkCargoDeny({ root, projects, spawn = spawnSync }) {
  if (!projects.length) return null;
  const which = spawn('bash', ['-c', 'command -v cargo-deny'], { encoding: 'utf8' });
  if (which.status !== 0) {
    return row('cargo-deny-bans', 'WARN', 'cargo-deny is not installed, so the deny.toml boundary is not checked on this device (CI still checks it)',
      'cargo install --locked cargo-deny');
  }
  const r = spawn('cargo', ['deny', '--manifest-path', 'elohim/Cargo.toml', 'check', 'bans'], { cwd: root, encoding: 'utf8' });
  if (r.status === 0) return row('cargo-deny-bans', 'ok');
  const tail = String(r.stderr || r.stdout).trim().split('\n').filter(Boolean).slice(-2).join(' | ');
  return row('cargo-deny-bans', 'REFUSED', `deny.toml boundary violation — ${tail}`, 'see deny.toml; cargo deny --manifest-path elohim/Cargo.toml check bans');
}

/**
 * The cargo workspace root above `dir` (repo-relative, inclusive): the nearest ancestor whose Cargo.toml declares
 * `[workspace]` or that owns a Cargo.lock (an `exclude`d crate such as elohim-storage is its own root, though a
 * parent workspace sits above it); else the nearest Cargo.toml.
 */
function cargoWorkspaceOf(root, dir) {
  let nearest = null;
  for (let d = dir; d && d !== '.' && d !== '/'; d = dirname(d)) {
    const manifest = join(root, d, 'Cargo.toml');
    if (!existsSync(manifest)) continue;
    nearest ??= d;
    if (existsSync(join(root, d, 'Cargo.lock')) || /^\s*\[workspace[\].]/m.test(readFileSync(manifest, 'utf8'))) return d;
  }
  return nearest;
}

const LOCK_STALE = /lock file .*needs to be updated|--locked was passed/i;

/**
 * A manifest edit that outran its Cargo.lock passes the local gate (its cargo rewrites the lock)
 * and fails the first `--locked` CI build; on 2026-11-07 it surfaced only as a dirty tree after a
 * 65-minute eprfs gate. `cargo metadata --locked --offline` resolves without writing, in seconds.
 * Only the lock-update refusal is a red; a crate missing from the offline cache is a WARN.
 */
export function checkCargoLocks({ root, projects, spawn = spawnSync, env = process.env }) {
  const workspaces = new Set();
  for (const p of projects) {
    if (!p.run?.cargo || !p.dir || p.dir.startsWith('elohim/holochain/dna')) continue;
    const ws = cargoWorkspaceOf(root, p.dir);
    if (ws && !ws.startsWith('elohim/holochain/dna')) workspaces.add(ws);
  }
  if (!workspaces.size) return null;
  // The pin lives only in pool-policy; with no readable pin, cargo resolves its own default.
  let toolchain;
  try { toolchain = JSON.parse(readFileSync(join(root, 'genesis/agentic/pool-policy.json'), 'utf8')).cargo_env_overrides?.['*']?.RUSTUP_TOOLCHAIN; }
  catch { /* an unreadable policy pins nothing */ }
  const rows = [];
  for (const ws of [...workspaces].sort()) {
    const manifest = join(root, ws, 'Cargo.toml');
    const r = spawn('cargo', ['metadata', '--locked', '--offline', '--format-version', '1', '--manifest-path', manifest],
      { cwd: root, encoding: 'utf8', stdio: ['ignore', 'ignore', 'pipe'], env: { ...env, RUSTFLAGS: '', ...(toolchain ? { RUSTUP_TOOLCHAIN: toolchain } : {}) } });
    const name = `cargo-lock:${ws}`;
    if (r.status === 0) { rows.push(row(name, 'ok')); continue; }
    const err = String(r.stderr || r.error?.message || '').trim();
    const first = err.split('\n').filter(Boolean).slice(0, 2).join(' | ');
    if (LOCK_STALE.test(err)) {
      rows.push(row(name, 'REFUSED', `${ws}/Cargo.lock lags its manifests — a --locked CI build refuses it, and this gate would silently rewrite it`,
        `cargo update --workspace --offline --manifest-path ${ws}/Cargo.toml, then commit the lockfile with the manifest change`));
    } else {
      rows.push(row(name, 'WARN', `could not check ${ws}/Cargo.lock offline (not a lock mismatch) — ${first || 'cargo unavailable'}`,
        `cargo metadata --locked --format-version 1 --manifest-path ${ws}/Cargo.toml > /dev/null`));
    }
  }
  return rows;
}

/**
 * Row 29.2: a project on a declared workspace (pool-policy.json declared_targets) builds its C at
 * the declared level, or what it builds must not be offered to a sibling (a drawn epr whose ring C
 * carried AVX2 died with SIGILL on a v2 device). Never REFUSED: the device's own gate runs and
 * passes as before; the WARN withholds the producer's offer (`withholds.offer` names the projects).
 * The env judged is each project's build env (gateBuildEnv), never the caller's alone: a C-family
 * key the gate itself sets (manifest run.cargo.env, cargo_env_overrides) reaches the C it builds.
 */
export function checkCBaselineOffer({ root, projects, env = process.env, spawn = spawnSync, record = readDeclaredTargets(root), hostTriple = hostTripleOf(), check = checkCBaseline, buildEnv = gateBuildEnv }) {
  const workspaces = [...new Set(projects.map(p => p.run?.cargo?.workspace).filter(Boolean))];
  const targets = applicableTargets(record, { workspaces, hostTriple });
  if (!targets.length) return null;
  const covered = new Set(targets.flatMap(t => t.workspaces));
  const offered = projects.filter(p => covered.has(p.run?.cargo?.workspace));
  // Each project is judged in the env its own gate builds with; projects whose build env is
  // identical share one check (one probe).
  const byEnv = new Map();
  const findings = [];
  const withheld = [];
  for (const project of offered) {
    const workspace = project.run.cargo.workspace;
    const built = buildEnv(project, env, root);
    const key = JSON.stringify([workspace, Object.entries(built).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))]);
    if (!byEnv.has(key)) byEnv.set(key, check({ root, env: built, record, spawn, workspaces: [workspace], hostTriple }));
    const own = byEnv.get(key);
    if (!own.length) continue;
    withheld.push(project.name);
    for (const f of own) {
      const seen = findings.find(g => g.why === f.why && g.check === f.check && g.triple === f.triple);
      if (seen) seen.projects.push(project.name);
      else findings.push({ ...f, projects: [project.name] });
    }
  }
  const declared = targets.map(t => `${t.triple} at ${t.cflags}`).join(', ');
  if (!findings.length) return row('c-baseline', 'ok', `C for ${declared}`);
  const named = f => (offered.length > 1 ? `${f.why} (${f.projects.join(' ')})` : f.why);
  return {
    ...row('c-baseline', 'WARN',
      `${findings.length} C-baseline finding(s) for ${declared} — ${findings.slice(0, 3).map(named).join('; ')}${findings.length > 3 ? `; +${findings.length - 3} more` : ''} — the gate still runs, but this device's offer of ${withheld.join(' ')} is withheld`,
      [...new Set(findings.map(f => f.cure))].join(' ; ')),
    withholds: { offer: withheld },
    findings,
  };
}

/**
 * The environment a project's cargo build runs under: gate-runner's gateChildEnv over `env`
 * (the one merge of manifest run.cargo.env, pool-policy cargo_env_overrides and declared_targets),
 * with its GATE_CARGO_ENV map exported over the rest and the variable itself gone, as
 * run-local-gate.sh does before `just`. gateChildEnv's conflict line is the gate's to print.
 */
export function gateBuildEnv(project, env, root) {
  const { GATE_CARGO_ENV: map, ...child } = gateChildEnv(project, env, root, { warn: () => {} });
  let declared = {};
  try { declared = JSON.parse(map || '{}') || {}; } catch { /* gateChildEnv writes JSON; nothing to overlay otherwise */ }
  return { ...child, ...Object.fromEntries(Object.entries(declared).map(([key, value]) => [key, String(value)])) };
}

/**
 * A reuse-enabled gate that runs on a dirty closure records nothing, so the next push pays it
 * again in full (87 min for eprfs on 2026-10-27). Said before the gate starts, not after.
 */
function listedReads(root, list) {
  try { return suiteReadSet(readFileSync(join(root, list), 'utf8'), list); } catch { return null; }
}

export function checkEvidenceRecordable({ root, projects, env = process.env, reuse, read = readSet, dirty = dirtyPaths }) {
  const enabled = new Set(reuse ?? []);
  const rows = [];
  const commit = (env.GATE_COMMIT || '').trim() || 'HEAD';
  for (const project of projects.filter(p => enabled.has(p.name))) {
    let paths;
    try {
      // Mirror the closure key (gate-evidence.mjs readSetAt): every cargo project is keyed on its
      // named reads when it has no unresolved one; a non-cargo gate, and a project whose recipe
      // runs tests the audit cannot read (UNAUDITED_RECIPE_PROJECTS), keys on the whole tree. A
      // suite list (SUITE_READ_LISTS) keys on its reads; an unreadable list keys on the tree, as
      // readSetAt does.
      const list = SUITE_READ_LISTS[project.name];
      const result = list ? listedReads(root, list)
        : (NAMED_READ_SET_PROJECTS.includes(project.name) || project.run?.cargo)
          && !UNAUDITED_RECIPE_PROJECTS.includes(project.name)
          ? read({ root, project, sources: sourcesOf(root, project) }) : null;
      const reads = result && result.unresolved.length === 0 && !result.paths.includes('.')
        ? { scope: 'paths', oids: result.paths.map(path => ({ path })) }
        : { scope: 'tree' };
      paths = dirty(project, { root, commit, reads });
    } catch { paths = null; }
    if (paths === null || paths.length === 0) continue;
    rows.push(row(`evidence:${project.name}`, 'WARN',
      `${paths.length} dirty path(s) in its closure (${paths.slice(0, 3).join(' ')}) — this run records no evidence and the next push pays it again`,
      'commit them, or move hook-written ledger lines out (append to the main tree copy, then git checkout -- <file>)'));
  }
  return rows;
}

export function runPreflight({ root = ROOT, projects, env = process.env, spawn = spawnSync, registry = loadGateRegistry(root), changed = changedFromEnv(env), hostTriple = hostTripleOf(), onRow = () => {}, fleet = {} }) {
  const read = memoRead(readSet);
  let reuse = [];
  try { reuse = JSON.parse(readFileSync(join(root, 'genesis/agentic/pool-policy.json'), 'utf8')).gate_reuse?.projects || []; }
  catch { /* No reuse list: nothing to warn about. */ }
  const timed = (fn) => {
    const started = process.hrtime.bigint();
    let out;
    try { out = fn(); } catch (error) { out = row(fn.name || 'preflight', 'WARN', `check threw: ${String(error?.message || error).split('\n')[0]}`); }
    const ms = Number(process.hrtime.bigint() - started) / 1e6;
    const rows = [].concat(out ?? []).map(r => ({ ...r, ms }));
    for (const r of rows) onRow(r);
    return rows;
  };
  return [
    ...timed(() => checkSubmodules({ root, spawn })),
    ...timed(() => checkSubmodulePins({ root, projects })),
    ...timed(() => checkTmpdir({ projects, env })),
    ...timed(() => checkNodeModules({ root, projects })),
    ...DIST_CHECKS.flatMap(check => timed(() => check({ root, projects }))),
    ...timed(() => checkArk({ projects, env })),
    ...timed(() => checkCargoDeny({ root, projects, spawn })),
    ...timed(() => checkCargoLocks({ root, projects, spawn, env })),
    ...timed(() => checkCBaselineOffer({ root, projects, env, spawn, hostTriple })),
    ...timed(() => checkReadSets({ root, projects, registry, changed, read })),
    ...timed(() => checkEvidenceRecordable({ root, projects, env, reuse, read })),
    // pre-push runs this one alone before its early passes (`--fleet-roll`); it is skipped here only
    // when that run's mark file holds exactly this push's ref list.
    ...(fleetGuard?.fleetRollMarkHonoured(env) ? [] : timed(() => fleetRollRows(env, changed, fleet))),
  ];
}

/**
 * The fleet-roll rows, from fleet-roll-guard.mjs, or when that module cannot load, one row saying
 * so: red on a push that names dev or main, in every mode (the policy in force is read from git
 * objects by the module that failed), a WARN otherwise.
 */
export function fleetRollRows(env, changed, options = {}, guard = fleetGuard, loadError = fleetGuardError) {
  if (guard) return guard.checkFleetRoll({ env, changed, ...options });
  const refs = `${env.GATE_PUSH_TARGETS ?? ''}\n${env.GATE_PUSH_REFS ?? ''}`;
  const deploys = /(^|\s)refs\/heads\/(dev|main)(\s|$)/m.test(refs);
  return row('fleet-roll', deploys ? 'REFUSED' : 'WARN',
    `the fleet-roll check could not load: ${String(loadError?.message || loadError).split('\n')[0]}`,
    'node genesis/orchestrator/fleet-roll-guard.mjs names the load error; fix it and push again');
}

/** pre-push exports the push range's file list; `just gate` has none and audits everything. */
export function changedFromEnv(env = process.env) {
  if (!env.GATE_CHANGED_FILES) return null;
  try { return readFileSync(env.GATE_CHANGED_FILES, 'utf8').split('\n').filter(Boolean); }
  catch { return null; }
}

export function formatRow(r) {
  if (r.level === 'ok') return `[preflight] ok ${r.check}${r.why ? ` (${r.why})` : ''}`;
  return `[preflight] ${r.level} ${r.check}: ${r.why}${r.cure ? `\n[preflight]   cure: ${r.cure}` : ''}`;
}

/**
 * Emit each REFUSED to the live run as a failure the session hears at its next tool call, and
 * each WARN that withholds the producer's offer (c-baseline) as a `notice` event: a non-red fact
 * the session hears the same way (gate-live-inject, the follower's NOTICE line) while the gate
 * stays green, and that nothing counts as a red. Other WARNs stay local.
 */
export function emitLive(rows, env = process.env, append = appendEvent) {
  if (!env.GATE_LIVE_RUN) return;
  for (const r of rows) {
    if (r.level === 'REFUSED') {
      append(env.GATE_LIVE_RUN, {
        kind: 'failure', project: 'preflight',
        failure: { class: r.check, excerpt: [r.why, r.cure ? `cure: ${r.cure}` : ''].filter(Boolean) },
      });
    } else if (r.level === 'WARN' && r.withholds) {
      const offer = (r.withholds.offer || []).join(' ') || 'this device';
      const text = `${r.check}: offer of ${offer} withheld, the gate still runs: ${r.why}${r.cure ? ` — cure: ${r.cure}` : ''}`;
      append(env.GATE_LIVE_RUN, {
        kind: 'notice', project: 'preflight',
        notice: { class: 'offer-withheld', text: text.replace(/\s*\n\s*/g, ' ') },
      });
    }
  }
}

function main(argv) {
  if (argv.includes('--on-red')) {
    process.stdout.write(`${onRedPolicy().mode}\n`);
    return 0;
  }
  if (argv.includes('--fleet-roll')) {
    // Project-independent, so pre-push runs it before the early passes that skip the preflight.
    // Exit 3 when red, so the hook tells a refusal from a node that could not start (any other code).
    const rows = [].concat(fleetRollRows(process.env, changedFromEnv(process.env)) ?? []);
    for (const r of rows) process.stdout.write(`${formatRow(r)}\n`);
    emitLive(rows);
    // The mark the later preflight checks: this exact ref list was judged in this hook run.
    if (process.env.GATE_FLEET_ROLL_MARK && process.env.GATE_PUSH_REFS) {
      try { writeFileSync(process.env.GATE_FLEET_ROLL_MARK, process.env.GATE_PUSH_REFS); } catch { /* no mark: the later preflight runs the check again */ }
    }
    return rows.some(r => r.level === 'REFUSED') ? 3 : 0;
  }
  const index = argv.indexOf('--projects');
  const names = index >= 0 ? argv.slice(index + 1).filter(a => !a.startsWith('--')) : [];
  const registry = loadGateRegistry(ROOT);
  const projects = names.map(name => registry.get(name)).filter(Boolean);
  const started = Date.now();
  const rows = runPreflight({ root: ROOT, projects, registry, onRow: r => {
    process.stdout.write(`${formatRow(r)}\n`);
    emitLive([r]);
  } });
  const refused = rows.filter(r => r.level === 'REFUSED').length;
  const warned = rows.filter(r => r.level === 'WARN').length;
  process.stdout.write(`[preflight] ${refused ? `${refused} REFUSED` : 'clear'}${warned ? `, ${warned} WARN` : ''} in ${((Date.now() - started) / 1000).toFixed(1)}s\n`);
  return refused ? 1 : 0;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = main(process.argv.slice(2));
}
