#!/usr/bin/env node
// Atomic, incremental replant for elohim-agent packages.
//
// WHY THIS EXISTS
// ---------------
// `package-projections.mjs project --write-runtime` rewrites EVERY package's
// runtime file. On a tree where some packages are un-planted (Claude-authored
// source) or ambiently modified by a concurrent session, that clobbers work
// nobody asked it to touch. `--only <name>` scopes the write, but a plant is
// still three separate steps (fixtures, runtime, verify) with no rollback: if
// verify goes red after the writes land, the tree is left half-planted.
//
// This wrapper makes one plant ATOMIC and a batch INCREMENTAL:
//   * atomic     — snapshot every file a plant could touch, run the plant, and
//                  restore byte-exactly if the plant makes verify worse.
//   * incremental— one package at a time, stop on the first failure. A partial
//                  run leaves N good plants and zero half-plants.
//   * no clobber — always `--only <name>`; never a bare tree-wide write.
//
// BASELINE-AWARE VERIFY
// ---------------------
// The tree can already be red for reasons unrelated to the plant (e.g. a stale
// root CLAUDE.md projection). We capture the FAIL set once, before touching
// anything, and only roll back on failures that are NEW relative to it.
//
// USAGE
//   node elohim/sdk/domains/elohim-agent/scripts/replant.mjs <name> [<name>...]
//   node elohim/sdk/domains/elohim-agent/scripts/replant.mjs --dry-run <name>
//   node elohim/sdk/domains/elohim-agent/scripts/replant.mjs --compose <name>
//
// FLAGS
//   --dry-run   Report what would change; restore everything afterward.
//   --compose   After all plants succeed, record composition provenance via
//               `eprfs-agent compose-graph`. Skipped with a warning if the
//               binary is absent — a missing provenance record is not a reason
//               to fail plants that already verified green.
//   --keep-going  Continue to the next name after a failure instead of stopping.
//
// EXIT CODES
//   0  every requested plant verified green (or dry-run completed)
//   1  a plant introduced new verify failures and was rolled back
//   2  bad usage / unreadable package / projector could not run

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';

const REPO_ROOT = resolve(import.meta.dirname, '../../../../..');
const PROJECTOR = resolve(import.meta.dirname, 'package-projections.mjs');
const PROJECTION_DIR = resolve(REPO_ROOT, '.epr-meta/elohim/projections');
const PACKAGE_DIR = resolve(REPO_ROOT, '.epr-meta/elohim/packages');

const args = process.argv.slice(2);
const DRY_RUN = args.includes('--dry-run');
const COMPOSE = args.includes('--compose');
const KEEP_GOING = args.includes('--keep-going');
const targets = args.filter((a) => !a.startsWith('--'));

if (targets.length === 0) {
  console.error('usage: replant.mjs [--dry-run] [--compose] [--keep-going] <name> [<name>...]');
  process.exit(2);
}

// ---------------------------------------------------------------- snapshotting

// Walk every file under a directory, returning repo-relative paths.
function walkFiles(dir) {
  if (!existsSync(dir)) return [];
  const out = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) out.push(...walkFiles(full));
    else if (entry.isFile()) out.push(full);
  }
  return out;
}

// The set of files a plant can touch: the whole projection-fixture tree (small
// — a few hundred KB of markdown) plus the runtime paths the packages declare.
// Snapshotting the tree wholesale rather than predicting per-kind fixture paths
// means rollback is exact for every package kind, including ones this script
// has never seen.
function filesAtRisk(packageNames) {
  const paths = new Set(walkFiles(PROJECTION_DIR));
  for (const name of packageNames) {
    for (const runtimePath of runtimePathsForPackage(name)) {
      paths.add(resolve(REPO_ROOT, runtimePath));
    }
  }
  return [...paths];
}

function snapshot(paths) {
  const snap = new Map();
  for (const p of paths) snap.set(p, existsSync(p) ? readFileSync(p) : null);
  return snap;
}

// Restore a snapshot byte-exactly. Files that did not exist at snapshot time
// are removed; files that did are rewritten only when their bytes differ, so a
// clean rollback leaves mtimes alone.
function restore(snap) {
  const restored = [];
  for (const [p, bytes] of snap) {
    const now = existsSync(p) ? readFileSync(p) : null;
    if (bytes === null) {
      if (now !== null) {
        rmSync(p);
        restored.push(p);
      }
      continue;
    }
    if (now === null || !now.equals(bytes)) {
      mkdirSync(dirname(p), { recursive: true });
      writeFileSync(p, bytes);
      restored.push(p);
    }
  }
  return restored;
}

function changedSince(snap) {
  const changed = [];
  for (const [p, bytes] of snap) {
    const now = existsSync(p) ? readFileSync(p) : null;
    if (bytes === null && now !== null) changed.push({ path: p, kind: 'created' });
    else if (bytes !== null && now === null) changed.push({ path: p, kind: 'deleted' });
    else if (bytes !== null && now !== null && !now.equals(bytes)) changed.push({ path: p, kind: 'modified' });
  }
  return changed;
}

// ------------------------------------------------------------------- packages

// Find a package JSON by bare id across every kind directory.
function packageFileFor(name) {
  for (const file of walkFiles(PACKAGE_DIR)) {
    if (!file.endsWith('.json')) continue;
    try {
      const pkg = JSON.parse(readFileSync(file, 'utf8'));
      if (pkg?.metadata?.id === name || pkg?.metadata?.name === name) return file;
    } catch {
      // A malformed package is the projector's problem to report, not ours; we
      // are only searching for a name match.
    }
  }
  return null;
}

function runtimePathsForPackage(name) {
  const file = packageFileFor(name);
  if (!file) return [];
  const pkg = JSON.parse(readFileSync(file, 'utf8'));
  return Object.values(pkg.projections ?? {})
    .map((p) => p?.path)
    .filter((p) => typeof p === 'string');
}

// ---------------------------------------------------------------- projector

function runProjector(subcommand, extraArgs) {
  try {
    const stdout = execFileSync('node', [PROJECTOR, subcommand, ...extraArgs], {
      cwd: REPO_ROOT,
      encoding: 'utf8',
      // The projector walks every package and validates each one; on a cold
      // filesystem the full verify pass is the slow leg. Ten minutes is well
      // clear of an observed run while still bounding a hang.
      timeout: 600_000,
      maxBuffer: 64 * 1024 * 1024,
      // Capture the child's stderr instead of letting it reach the terminal.
      // The projector prints every FAIL line it sees, including pre-existing
      // ones; echoing those would contradict this tool's own baseline-aware
      // verdict and read as a failed plant when the plant was clean.
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    return { ok: true, stdout };
  } catch (error) {
    // A non-zero projector exit is a normal, expected outcome (verify red), so
    // return it as data rather than throwing.
    return { ok: false, stdout: `${error.stdout ?? ''}${error.stderr ?? ''}`, error };
  }
}

function failureSet(stdout) {
  return new Set(
    stdout
      .split('\n')
      .filter((line) => line.startsWith('FAIL:'))
      .map((line) => line.trim()),
  );
}

function verifyFailures() {
  const { stdout } = runProjector('verify', []);
  return failureSet(stdout);
}

// ------------------------------------------------------------------ main flow

const rel = (p) => relative(REPO_ROOT, p).split('\\').join('/');

// Confirm every requested name resolves before writing anything — a typo
// should cost nothing.
const missing = targets.filter((name) => !packageFileFor(name));
if (missing.length > 0) {
  console.error(`no package found for: ${missing.join(', ')}`);
  console.error(`(searched ${rel(PACKAGE_DIR)} by metadata.id and metadata.name)`);
  process.exit(2);
}

const results = [];

for (const name of targets) {
  // Re-capture the baseline immediately before EACH plant rather than once for
  // the batch. This tree is shared: a concurrent session can edit an unrelated
  // package between two of our plants, which makes that package's projections
  // stale. Against a batch-wide baseline that staleness reads as a failure our
  // plant introduced, and we would roll back a perfectly good plant.
  const baseline = verifyFailures();
  console.log(`${name}: baseline ${baseline.size} pre-existing failure(s) (excluded from rollback)`);

  const atRisk = filesAtRisk([name]);
  const snap = snapshot(atRisk);

  const fixtures = runProjector('project', ['--write-fixtures', '--only', name]);
  const runtime = fixtures.ok ? runProjector('project', ['--write-runtime', '--only', name]) : fixtures;

  if (!runtime.ok) {
    const restored = restore(snap);
    console.log(`✗ ${name}: projector failed; rolled back ${restored.length} file(s)`);
    console.log(runtime.stdout.split('\n').filter((l) => l.startsWith('FAIL:')).slice(0, 5).join('\n'));
    results.push({ name, status: 'projector-failed' });
    if (!KEEP_GOING) break;
    continue;
  }

  const after = verifyFailures();
  const changed = changedSince(snap);

  // Two distinct failure classes, and conflating them hides real breakage:
  //
  //   introduced — any failure absent from the baseline. Caused by this plant.
  //   unresolved — any failure still naming the TARGET after the plant.
  //
  // The baseline must never excuse a failure about the target itself. The
  // package edit always lands BEFORE the plant, so a failure that edit caused
  // (e.g. metadata.description changed without the matching
  // projections.*.frontmatter.description) is already in the baseline and would
  // be waved through as "pre-existing" — the plant would report clean while
  // leaving its own target red. Baseline exclusion applies to OTHER packages
  // only; that is its entire purpose.
  const introduced = [...after].filter((f) => !baseline.has(f));
  const unresolved = [...after].filter((f) => f.includes(name) && !introduced.includes(f));

  if (unresolved.length > 0) {
    const restored = restore(snap);
    console.log(`✗ ${name}: ${unresolved.length} failure(s) about this target survived the plant; rolled back ${restored.length} file(s)`);
    for (const f of unresolved) console.log(`    ${f}`);
    console.log('    The package itself is inconsistent — projecting it cannot fix that. Fix the package, then replant.');
    results.push({ name, status: 'unresolved', unresolved });
    if (!KEEP_GOING) break;
    continue;
  }

  if (introduced.length > 0) {
    const restored = restore(snap);
    console.log(`✗ ${name}: introduced ${introduced.length} new verify failure(s); rolled back ${restored.length} file(s)`);
    for (const f of introduced) console.log(`    ${f}`);
    results.push({ name, status: 'rolled-back', introduced });
    if (!KEEP_GOING) break;
    continue;
  }

  if (DRY_RUN) {
    const restored = restore(snap);
    console.log(`· ${name}: would update ${changed.length} file(s) (dry-run, restored ${restored.length})`);
    for (const c of changed) console.log(`    ${c.kind}: ${rel(c.path)}`);
    results.push({ name, status: 'dry-run', changed: changed.length });
    continue;
  }

  console.log(`✓ ${name}: planted, verify clean vs baseline — ${changed.length} file(s) updated`);
  for (const c of changed) console.log(`    ${c.kind}: ${rel(c.path)}`);
  results.push({ name, status: 'planted', changed: changed.length });
}

// Composition provenance is recorded once, after the batch, and only if every
// plant held. It is a record of what happened, not a gate on it.
const allGreen = results.every((r) => r.status === 'planted' || r.status === 'dry-run');
if (COMPOSE && allGreen && !DRY_RUN) {
  console.log('\nrecording composition provenance…');
  try {
    execFileSync(
      'eprfs-agent',
      [
        'compose-graph',
        '.epr-meta/elohim/packages',
        '--projections-root',
        '.epr-meta/elohim/projections',
      ],
      { cwd: REPO_ROOT, encoding: 'utf8', timeout: 600_000, stdio: 'inherit' },
    );
  } catch (error) {
    console.warn(`warning: compose-graph did not run (${error.code ?? error.message}).`);
    console.warn('Plants above are still green; re-run compose-graph once eprfs-agent is built.');
  }
}

console.log('\n─── replant summary ───');
for (const r of results) console.log(`  ${r.status.padEnd(16)} ${r.name}`);
const skipped = targets.length - results.length;
if (skipped > 0) console.log(`  ${'not-attempted'.padEnd(16)} ${targets.slice(results.length).join(', ')}`);

const failed = results.filter((r) => r.status === 'rolled-back' || r.status === 'projector-failed');
process.exit(failed.length > 0 ? 1 : 0);
