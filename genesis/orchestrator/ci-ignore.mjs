#!/usr/bin/env node
// .ci-ignore — gitignore-style patterns for files that NEVER trigger
// source pipelines. Single source of truth: the repo-root `.ci-ignore`.
//
// This module is the shared JS port of that file. Consumers:
//   - orchestrator-strategy.mjs  (analyzePipelineRequirements)
//   - graph-walker.mjs CLI        (.husky/pre-push pipes $CHANGED through it)
//   - .husky/pre-push             (direct CLI filter for the fallback path)
//
// The Jenkinsfile mirrors this logic in Groovy (loadCiIgnore / isCiIgnored)
// because Groovy can't import .mjs; drift between the two is caught by
// orchestrator-strategy.test.mjs which loads BOTH and compares parses.
//
// Library usage:
//   import { matchesCiIgnore, CI_IGNORE_PATTERNS, filterChanged } from './ci-ignore.mjs';
//
// CLI usage (reads newline-separated paths from stdin, emits filtered list):
//   echo "file1\nfile2\n.claude/x" | node genesis/orchestrator/ci-ignore.mjs

import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const CI_IGNORE_PATH = resolve(REPO_ROOT, '.ci-ignore');

/**
 * Parse .ci-ignore text into a list of pattern objects.
 *
 * Pattern shapes (gitignore-flavored):
 *   - trailing '/'       → subtree prefix match
 *   - contains '/'       → exact path match (anchored)
 *   - no '/'             → basename match anywhere
 *
 * Returns array of `{ kind: 'prefix'|'exact'|'basename', value: string }`.
 */
export function parseCiIgnore(text) {
  return text
    .split('\n')
    .map(line => line.replace(/\s*#.*$/, '').trim())
    .filter(Boolean)
    .map(value => {
      if (value.endsWith('/')) return { kind: 'prefix', value };
      if (value.includes('/')) return { kind: 'exact', value };
      return { kind: 'basename', value };
    });
}

/**
 * Check a single file path against parsed .ci-ignore patterns.
 * Mirrors the same logic in Jenkinsfile's isCiIgnored helper.
 */
export function matchesCiIgnore(file, patterns) {
  for (const p of patterns) {
    if (p.kind === 'prefix') {
      if (file.startsWith(p.value)) return true;
    } else if (p.kind === 'exact') {
      if (file === p.value) return true;
    } else {
      const slash = file.lastIndexOf('/');
      const basename = slash >= 0 ? file.slice(slash + 1) : file;
      if (basename === p.value) return true;
    }
  }
  return false;
}

export const CI_IGNORE_PATTERNS = parseCiIgnore(readFileSync(CI_IGNORE_PATH, 'utf8'));

/**
 * Filter a list of changed file paths, dropping any that match .ci-ignore.
 * Convenience wrapper around matchesCiIgnore for the common case.
 */
export function filterChanged(files, patterns = CI_IGNORE_PATTERNS) {
  return files.filter(f => !matchesCiIgnore(f, patterns));
}

// ── CLI mode ─────────────────────────────────────────────────────
// Reads newline-separated paths from stdin, emits surviving paths to stdout.
// Used by .husky/pre-push to filter $CHANGED before the manifest/fallback
// detection paths run, so .ci-ignore is honored uniformly.

const isMain = import.meta.url === `file://${process.argv[1]}` ||
               import.meta.url === `file://${resolve(process.argv[1])}`;

if (isMain) {
  // fd 0 (stdin) — robust to pipes from execFileSync; '/dev/stdin' can
  // ENXIO when stdin is a rewired child-process pipe.
  const input = readFileSync(0, 'utf8');
  const files = input.split('\n').map(f => f.trim()).filter(Boolean);
  const survivors = filterChanged(files);
  if (survivors.length > 0) {
    process.stdout.write(survivors.join('\n') + '\n');
  }
}
