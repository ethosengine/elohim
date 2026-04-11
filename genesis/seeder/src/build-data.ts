/**
 * Humans / Presences JSON Generator
 *
 * Reads markdown sources with YAML frontmatter from genesis/data/humans/ and
 * genesis/data/presences/ and emits derived JSON projections (humans.json,
 * presences.json) checked into the repo. A pre-push hook verifies that the
 * derived JSON is fresh relative to its markdown sources.
 *
 * This module is intentionally a simple transform — validation lives in
 * validate-humans.ts and validate-presences.ts.
 */

import { readFile, readdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { parse as parseYaml } from 'yaml';
import type { HumanFrontmatter, RelationshipEntry } from './validate-humans.js';
import type { PresenceFrontmatter } from './validate-presences.js';

// =============================================================================
// Types
// =============================================================================

export interface HumansJson {
  version: string;
  description: string;
  humans: HumanFrontmatter[];
  relationships: RelationshipEntry[];
}

export interface PresencesJson {
  version: string;
  description: string;
  presences: PresenceFrontmatter[];
}

// =============================================================================
// Helpers
// =============================================================================

const FRONTMATTER_RE = /^---\r?\n([\s\S]*?)\r?\n---/;
const SKIP_FILES = new Set(['relationships.md', 'README.md']);

function extractFrontmatter(source: string): unknown {
  const m = FRONTMATTER_RE.exec(source);
  if (!m) return null;
  try {
    return parseYaml(m[1]);
  } catch {
    return null;
  }
}

async function listMarkdownFiles(dir: string): Promise<string[]> {
  const entries = await readdir(dir);
  return entries.filter(f => f.endsWith('.md') && !SKIP_FILES.has(f)).sort();
}

// =============================================================================
// Builders
// =============================================================================

export async function buildHumansJson(dir: string): Promise<HumansJson> {
  const files = await listMarkdownFiles(dir);
  const humans: HumanFrontmatter[] = [];

  for (const file of files) {
    const source = await readFile(join(dir, file), 'utf-8');
    const fm = extractFrontmatter(source);
    if (fm == null || typeof fm !== 'object') continue;
    humans.push(fm as HumanFrontmatter);
  }

  humans.sort((a, b) => (a.id ?? '').localeCompare(b.id ?? ''));

  // Relationships
  const relationships: RelationshipEntry[] = [];
  let entries: string[] = [];
  try {
    entries = await readdir(dir);
  } catch {
    // unreachable: listMarkdownFiles already succeeded
  }
  if (entries.includes('relationships.md')) {
    const relSource = await readFile(join(dir, 'relationships.md'), 'utf-8');
    const relFm = extractFrontmatter(relSource);
    if (relFm != null && typeof relFm === 'object') {
      const arr = (relFm as Record<string, unknown>).relationships;
      if (Array.isArray(arr)) {
        for (const rel of arr) {
          relationships.push(rel as RelationshipEntry);
        }
      }
    }
  }

  return {
    version: '2.0.0',
    description:
      'Generated from genesis/data/humans/*.md. DO NOT EDIT BY HAND. Regenerate with: pnpm --filter genesis-seeder run build:data',
    humans,
    relationships,
  };
}

export async function buildPresencesJson(dir: string): Promise<PresencesJson> {
  const files = await listMarkdownFiles(dir);
  const presences: PresenceFrontmatter[] = [];

  for (const file of files) {
    const source = await readFile(join(dir, file), 'utf-8');
    const fm = extractFrontmatter(source);
    if (fm == null || typeof fm !== 'object') continue;
    presences.push(fm as PresenceFrontmatter);
  }

  presences.sort((a, b) => (a.id ?? '').localeCompare(b.id ?? ''));

  return {
    version: '1.0.0',
    description:
      'Generated from genesis/data/presences/*.md. DO NOT EDIT BY HAND. Regenerate with: pnpm --filter genesis-seeder run build:data',
    presences,
  };
}

// =============================================================================
// CLI
// =============================================================================

async function writeJson(path: string, data: unknown): Promise<void> {
  await writeFile(path, JSON.stringify(data, null, 2) + '\n', 'utf-8');
}

async function runCli(target: string): Promise<void> {
  const humansDir = new URL('../../data/humans/', import.meta.url).pathname;
  const presencesDir = new URL('../../data/presences/', import.meta.url)
    .pathname;
  const humansOut = join(humansDir, 'humans.json');
  const presencesOut = join(presencesDir, 'presences.json');

  if (target === 'humans' || target === 'all') {
    const data = await buildHumansJson(humansDir);
    await writeJson(humansOut, data);
    console.log(`Wrote ${data.humans.length} humans to ${humansOut}`);
  }
  if (target === 'presences' || target === 'all') {
    const data = await buildPresencesJson(presencesDir);
    await writeJson(presencesOut, data);
    console.log(`Wrote ${data.presences.length} presences to ${presencesOut}`);
  }
}

const isCli =
  process.argv[1]?.endsWith('build-data.ts') ||
  process.argv[1]?.endsWith('build-data.js');

if (isCli) {
  const target = process.argv[2] ?? 'all';
  if (!['humans', 'presences', 'all'].includes(target)) {
    console.error(`Unknown target "${target}". Use: humans | presences | all`);
    process.exit(2);
  }
  runCli(target).catch(err => {
    console.error('build-data failed:', err);
    process.exit(1);
  });
}
