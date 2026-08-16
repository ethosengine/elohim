#!/usr/bin/env node
// Shared utilities for build manifest discovery, loading, and resolution.
// Used by both validate-manifests.mjs and graph-walker.mjs.

import { readFileSync, readdirSync } from 'fs';
import { resolve, relative, sep } from 'path';

/**
 * Discover all build-manifest.json files under rootDir.
 * Returns relative paths (e.g., './app/elohim-app/build-manifest.json').
 */
export function discoverManifests(rootDir) {
  const root = resolve(rootDir);
  const found = [];
  const skipped = new Set(['.claude', '.git', '.superpowers', 'node_modules']);

  function visit(dir) {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      if (entry.isDirectory()) {
        if (!skipped.has(entry.name)) visit(resolve(dir, entry.name));
      } else if (entry.isFile() && entry.name === 'build-manifest.json') {
        found.push(`.${sep}${relative(root, resolve(dir, entry.name))}`);
      }
    }
  }

  visit(root);
  return found.sort();
}

/**
 * Discover and parse all build-manifest.json files.
 * Returns [{ path, content }].
 */
export function loadManifests(rootDir) {
  const paths = discoverManifests(rootDir);
  return paths.map(relPath => {
    const absPath = resolve(rootDir, relPath);
    const content = JSON.parse(readFileSync(absPath, 'utf8'));
    return { path: relPath, content };
  });
}

/**
 * Normalize a dependency reference to qualified form.
 * 'build-angular' + 'elohim' -> 'elohim:build-angular'
 * 'elohim-sophia:build-sophia-umd' -> 'elohim-sophia:build-sophia-umd' (unchanged)
 */
export function resolveStep(dep, currentPipeline) {
  return dep.includes(':') ? dep : `${currentPipeline}:${dep}`;
}
