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

import { accessSync, constants, existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadGateRegistry } from './pipeline-registry.mjs';
import { globCovers, readSet } from './gate-input-audit.mjs';
import { dirtyPaths } from './gate-evidence.mjs';
import { appendEvent } from './gate-live.mjs';

const ROOT = process.env.GATE_ROOT
  ? resolve(process.env.GATE_ROOT)
  : resolve(dirname(fileURLToPath(import.meta.url)), '../..');

/** Submodules whose manifests declare attested gate projects; without them the registry is short. */
export const ATTESTED_SUBMODULES = ['elohim/brit', 'elohim/rakia', 'sophia'];

/**
 * Projects whose Rust read set must be fully named (gate-input-audit.test.mjs `REAL … read set`).
 * The orchestrator gate runs that test, so any of these, or the orchestrator itself, selects it.
 */
export const NAMED_READ_SET_PROJECTS = ['elohim-storage', 'doorway', 'steward-node', 'elohim-epr'];

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
    return row('submodules', 'REFUSED', `not checked out: ${missing.join(' ')} — their attested gate projects vanish and the orchestrator registry tests fail`,
      `git submodule update --init ${missing.join(' ')}`);
  }
  const moved = String(r.stdout).split('\n').filter(line => line.startsWith('+')).map(line => line.trim().split(/\s+/)[1]);
  if (moved.length) {
    return row('submodules', 'WARN', `checked-out commit differs from the pin: ${moved.join(' ')} — gates test bytes the push does not carry`,
      `git submodule update ${moved.join(' ')}   (or commit the pin you mean)`);
  }
  return row('submodules', 'ok');
}

export function checkNodeModules({ root, projects }) {
  const needing = projects.filter(usesNode);
  if (!needing.length) return null;
  if (existsSync(join(root, 'node_modules/.modules.yaml'))) return row('node_modules', 'ok');
  return row('node_modules', 'REFUSED', `no installed workspace for ${needing.map(p => p.name).join(' ')}`,
    'pnpm install --frozen-lockfile --prefer-offline');
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
  return row('element-dists', 'REFUSED', `unbuilt element package(s) a selected gate imports: ${missing.join(' ')}`,
    `pnpm ${missing.map(name => `--filter ${name}`).join(' ')} run build`);
}

export function checkStorageClientDist({ root, projects }) {
  const a2o = projects.some(p => p.dir.startsWith('genesis/a2o'));
  if (!a2o && !projects.some(underApp)) return null;
  if (existsSync(join(root, STORAGE_CLIENT_DIST))) return row('storage-client-dist', 'ok');
  // genesis-a2o builds nothing first (10-19: type-aware lint read error types; 11-01: a lane of 0
  // scenarios); app prebuilds build it, though lint can run before that.
  return row('storage-client-dist', a2o ? 'REFUSED' : 'WARN', `${STORAGE_CLIENT_DIST} absent — type-aware lint and the a2o lane read it`,
    'pnpm --filter @elohim/storage-client... run build');
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

export function checkSophiaUmd({ root, projects }) {
  if (!projects.some(p => p.name === 'elohim-app')) return null;
  if (SOPHIA_UMD.some(path => existsSync(join(root, path)))) return row('sophia-umd', 'ok');
  // check-sophia.sh can still fetch it from Nexus or a GitHub release, so this predicts, never refuses.
  return row('sophia-umd', 'WARN', 'no sophia-element UMD in the app assets or the sophia submodule — elohim-app prebuild must fetch one from the network or fail',
    'pnpm -C sophia install && pnpm -C sophia build && pnpm -C sophia --filter @ethosengine/sophia-element build:umd');
}

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
  if (!names.has('orchestrator') && !NAMED_READ_SET_PROJECTS.some(n => names.has(n)) && !names.has('epr-storage')) return null;
  const relevant = changed === null ? null : changed.filter(f => f.endsWith('.rs') || f.endsWith('build-manifest.json'));
  const problems = [];
  let audited = 0;
  for (const name of NAMED_READ_SET_PROJECTS) {
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
 * A reuse-enabled gate that runs on a dirty closure records nothing, so the next push pays it
 * again in full (87 min for eprfs on 2026-10-27). Said before the gate starts, not after.
 */
export function checkEvidenceRecordable({ root, projects, env = process.env, reuse, read = readSet, dirty = dirtyPaths }) {
  const enabled = new Set(reuse ?? []);
  const rows = [];
  const commit = (env.GATE_COMMIT || '').trim() || 'HEAD';
  for (const project of projects.filter(p => enabled.has(p.name))) {
    let paths;
    try {
      // Projects outside the named set key on the whole tree (pool-policy.json gate_reuse).
      const result = NAMED_READ_SET_PROJECTS.includes(project.name)
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

export function runPreflight({ root = ROOT, projects, env = process.env, spawn = spawnSync, registry = loadGateRegistry(root), changed = changedFromEnv(env), onRow = () => {} }) {
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
    ...timed(() => checkNodeModules({ root, projects })),
    ...timed(() => checkElementDists({ root, projects })),
    ...timed(() => checkStorageClientDist({ root, projects })),
    ...timed(() => checkSophiaUmd({ root, projects })),
    ...timed(() => checkArk({ projects, env })),
    ...timed(() => checkCargoDeny({ root, projects, spawn })),
    ...timed(() => checkReadSets({ root, projects, registry, changed, read })),
    ...timed(() => checkEvidenceRecordable({ root, projects, env, reuse, read })),
  ];
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

/** Emit each REFUSED to the live run as a failure the session hears at its next tool call. */
export function emitLive(rows, env = process.env, append = appendEvent) {
  if (!env.GATE_LIVE_RUN) return;
  for (const r of rows.filter(r => r.level === 'REFUSED')) {
    append(env.GATE_LIVE_RUN, {
      kind: 'failure', project: 'preflight',
      failure: { class: r.check, excerpt: [`${r.why}`, r.cure ? `cure: ${r.cure}` : ''].filter(Boolean) },
    });
  }
}

function main(argv) {
  if (argv.includes('--on-red')) {
    process.stdout.write(`${onRedPolicy().mode}\n`);
    return 0;
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
