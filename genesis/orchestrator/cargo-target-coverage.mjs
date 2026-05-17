#!/usr/bin/env node
// Cargo target coverage check.
//
// For every Cargo.toml whose path is itself covered by at least one
// build-manifest source glob, verify that each declared `[[bin]]`,
// `[[bench]]`, and `[[example]]` target's source file is ALSO covered
// by some manifest glob.
//
// Why this exists:
//   Adding a new `[[bench]]` to a Cargo.toml is a one-line change that
//   silently changes the build surface — a new file under `benches/`
//   becomes part of the crate, but if `benches/**` isn't in any
//   manifest source list, the orchestrator's quality gate won't trip
//   on changes to that file. The dep-cache placeholder layer in the
//   crate's Dockerfile will also blow up at manifest-parse time in CI.
//   Local pre-push gates use real files on disk and don't catch
//   either failure mode. We hit this twice in one shift on the
//   2026-05-17 graph-native landing.
//
// Library usage: import { findUncoveredTargets } from './cargo-target-coverage.mjs'
// CLI usage:     node cargo-target-coverage.mjs

import { readFileSync, readdirSync } from 'node:fs';
import { dirname, resolve, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import picomatch from 'picomatch';
import { loadManifests } from './manifest-utils.mjs';

const TARGET_KINDS = ['bin', 'bench', 'example'];
const SKIP_DIRS = new Set(['node_modules', '.git', 'target', 'dist', 'build', '.superpowers', '.cargo-target-pool']);

/**
 * Extract `[[bin]]`, `[[bench]]`, and `[[example]]` targets from a Cargo.toml.
 * Returns [{ kind, name, path }] where `path` may be null if not explicit.
 *
 * Hand-rolled extractor — Cargo array-of-table sections are well-formed
 * enough for a regex-based pass; we deliberately avoid pulling in a TOML
 * dep for one narrow use.
 */
export function extractCargoTargets(cargoTomlText) {
  const targets = [];
  // Match array-of-table headers: [[bin]] / [[bench]] / [[example]]
  // Allow trailing comments after `]]`.
  const headerRe = /^\s*\[\[(bin|bench|example)\]\]\s*(?:#.*)?$/gm;
  let match;
  while ((match = headerRe.exec(cargoTomlText)) !== null) {
    const kind = match[1];
    const bodyStart = match.index + match[0].length;
    // Body ends at the next section header (any `[...]` at line start).
    const nextSection = /^\s*\[/gm;
    nextSection.lastIndex = bodyStart;
    const next = nextSection.exec(cargoTomlText);
    const bodyEnd = next ? next.index : cargoTomlText.length;
    const body = cargoTomlText.slice(bodyStart, bodyEnd);
    const nameMatch = /^\s*name\s*=\s*"([^"]+)"/m.exec(body);
    const pathMatch = /^\s*path\s*=\s*"([^"]+)"/m.exec(body);
    if (!nameMatch) continue; // malformed — cargo would reject; skip
    targets.push({
      kind,
      name: nameMatch[1],
      path: pathMatch ? pathMatch[1] : null,
    });
  }
  return targets;
}

/**
 * Candidate source paths for a target, relative to repo root.
 * Returns every conventional location cargo would accept, so a manifest
 * covering ANY of them counts as coverage.
 */
export function candidateTargetPaths(target, crateDir) {
  if (target.path) {
    return [posixJoin(crateDir, target.path)];
  }
  switch (target.kind) {
    case 'bin':
      return [
        posixJoin(crateDir, `src/bin/${target.name}.rs`),
        posixJoin(crateDir, `src/bin/${target.name}/main.rs`),
        // sole-bin convention (cargo allows `[[bin]] name = "<pkg>"` to bind to src/main.rs)
        posixJoin(crateDir, 'src/main.rs'),
      ];
    case 'bench':
      return [
        posixJoin(crateDir, `benches/${target.name}.rs`),
        posixJoin(crateDir, `benches/${target.name}/main.rs`),
      ];
    case 'example':
      return [
        posixJoin(crateDir, `examples/${target.name}.rs`),
        posixJoin(crateDir, `examples/${target.name}/main.rs`),
      ];
    default:
      return [];
  }
}

function posixJoin(...parts) {
  return parts
    .map((p, i) => (i === 0 ? p.replace(/\/+$/, '') : p.replace(/^\/+|\/+$/g, '')))
    .filter(Boolean)
    .join('/');
}

/**
 * Discover Cargo.toml files under root, skipping vendored/build trees.
 */
export function findCargoTomls(rootDir) {
  const found = [];
  walk(rootDir, '.', found);
  return found.sort();
}

function walk(rootDir, relDir, out) {
  let entries;
  try {
    entries = readdirSync(resolve(rootDir, relDir), { withFileTypes: true });
  } catch {
    return;
  }
  for (const entry of entries) {
    if (entry.name.startsWith('.') && entry.name !== '.cargo') continue;
    if (SKIP_DIRS.has(entry.name)) continue;
    const relPath = relDir === '.' ? entry.name : `${relDir}/${entry.name}`;
    if (entry.isDirectory()) {
      walk(rootDir, relPath, out);
    } else if (entry.name === 'Cargo.toml') {
      out.push(relPath);
    }
  }
}

/**
 * Returns an array of glob matchers built from every manifest's
 * `inputs.sources` list (across all steps).
 */
export function buildManifestMatchers(manifests) {
  const matchers = [];
  for (const { content } of manifests) {
    for (const step of Object.values(content.steps || {})) {
      for (const glob of step.inputs?.sources || []) {
        matchers.push({ glob, matcher: picomatch(glob) });
      }
    }
  }
  return matchers;
}

function isCovered(path, matchers) {
  return matchers.some(({ matcher }) => matcher(path));
}

/**
 * Core check. Returns:
 *   {
 *     checked:  [{ cargoFile, targets: [{kind,name,path,candidates}] }],  // tracked crates examined
 *     skipped:  [{ cargoFile, reason }],                                  // not tracked by any manifest
 *     gaps:     [{ cargoFile, target, candidates }],                      // FAILS — declared but uncovered
 *   }
 *
 * A Cargo.toml is "tracked" iff its own path is matched by at least one
 * manifest source glob. Vendored / forked / out-of-tree crates that
 * nobody covers are skipped silently.
 */
export function findUncoveredTargets(rootDir, manifests) {
  const matchers = buildManifestMatchers(manifests);
  const cargoFiles = findCargoTomls(rootDir);
  const checked = [];
  const skipped = [];
  const gaps = [];

  for (const cargoFile of cargoFiles) {
    if (!isCovered(cargoFile, matchers)) {
      skipped.push({ cargoFile, reason: 'no manifest covers Cargo.toml itself' });
      continue;
    }
    let text;
    try {
      text = readFileSync(resolve(rootDir, cargoFile), 'utf8');
    } catch (err) {
      skipped.push({ cargoFile, reason: `unreadable: ${err.message}` });
      continue;
    }
    const targets = extractCargoTargets(text);
    if (targets.length === 0) continue;

    const crateDir = dirname(cargoFile);
    const targetsRecord = [];
    for (const target of targets) {
      const candidates = candidateTargetPaths(target, crateDir);
      const covered = candidates.some(p => isCovered(p, matchers));
      targetsRecord.push({ ...target, candidates, covered });
      if (!covered) {
        gaps.push({ cargoFile, target, candidates });
      }
    }
    checked.push({ cargoFile, targets: targetsRecord });
  }

  return { checked, skipped, gaps };
}

// ── CLI ──────────────────────────────────────────────────────────

const entry = process.argv[1];
const isMain = entry !== undefined && (
  import.meta.url === `file://${entry}` ||
  import.meta.url === `file://${resolve(entry)}`
);

if (isMain) {
  const __dirname = dirname(fileURLToPath(import.meta.url));
  const ROOT = resolve(__dirname, '../..');
  const manifests = loadManifests(ROOT);
  const { checked, skipped, gaps } = findUncoveredTargets(ROOT, manifests);

  console.log(`Checked ${checked.length} tracked crate(s); skipped ${skipped.length} (no manifest covers Cargo.toml).\n`);

  if (process.argv.includes('--verbose') || process.argv.includes('-v')) {
    for (const { cargoFile, targets } of checked) {
      console.log(`  ${cargoFile}`);
      for (const t of targets) {
        const mark = t.covered ? '✓' : '✗';
        console.log(`    ${mark} [[${t.kind}]] ${t.name}${t.path ? ` (path = "${t.path}")` : ''}`);
      }
    }
    console.log();
  }

  if (gaps.length === 0) {
    console.log('  ✓ All declared cargo targets are covered by at least one build-manifest source glob.');
    process.exit(0);
  }

  console.error(`  ✗ ${gaps.length} cargo target(s) declared in a tracked crate but not covered by any manifest source glob:\n`);
  for (const gap of gaps) {
    console.error(`    ${gap.cargoFile}: [[${gap.target.kind}]] '${gap.target.name}'`);
    console.error(`      looked for: ${gap.candidates.join(', ')}`);
    console.error(`      fix: add a glob covering one of those paths to a step in the manifest that already covers ${gap.cargoFile}`);
    console.error('');
  }
  process.exit(1);
}
