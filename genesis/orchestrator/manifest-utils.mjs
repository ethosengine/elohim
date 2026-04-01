#!/usr/bin/env node
// Shared utilities for build manifest discovery, loading, and resolution.
// Used by both validate-manifests.mjs and graph-walker.mjs.

import { readFileSync } from 'fs';
import { resolve } from 'path';
import { execSync } from 'child_process';

/**
 * Discover all build-manifest.json files under rootDir.
 * Returns relative paths (e.g., './app/elohim-app/build-manifest.json').
 */
export function discoverManifests(rootDir) {
  const output = execSync(
    "find . -name 'build-manifest.json' -not -path '*/node_modules/*' -not -path '*/.superpowers/*'",
    { cwd: rootDir, encoding: 'utf8' }
  );
  return output.trim().split('\n').filter(Boolean);
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
