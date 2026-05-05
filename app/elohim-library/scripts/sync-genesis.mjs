#!/usr/bin/env node
// MDX rendering: <Markdown> block (from @storybook/addon-docs/blocks) — confirmed in Task 1
//
// sync-genesis.mjs — single source of truth for genesis-to-graphos mapping.
//
// Copies mapped genesis files into projects/graphos/src/imported/ so that
// MDX wrappers can import them as ?raw at build time. Validates every
// mapping resolves; fails loudly otherwise.
//
// Glob mappings and gherkin transforms land in later tasks.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Repo root: app/elohim-library/scripts/sync-genesis.mjs → ../../../
const REPO_ROOT = resolve(__dirname, '..', '..', '..');
const GENESIS_DIR = join(REPO_ROOT, 'genesis');
const OUT_DIR = resolve(__dirname, '..', 'projects', 'graphos', 'src', 'imported');

export const MAPPINGS = [
  // I. Narrative Flow / Why
  { from: 'docs/content/elohim-protocol/manifesto.md',
    to: 'narrative/why/manifesto.md',
    title: 'I. Why / Manifesto' },
  { from: 'docs/content/elohim-protocol/constitution.md',
    to: 'narrative/why/constitution.md',
    title: 'I. Why / Constitution' },
  { from: 'docs/content/elohim-protocol/global-orchestra.md',
    to: 'narrative/why/vision.md',
    title: 'I. Why / Vision' },
  // I. Narrative Flow / What
  { from: 'graphos/elohim-protocol-design-spec.md',
    to: 'narrative/what/brand.md',
    title: 'I. What / Brand' },
  // I. Narrative Flow / How
  { from: 'docs/content/elohim-protocol/protocol-specification.md',
    to: 'narrative/how/protocol-specification.md',
    title: 'I. How / Protocol Specification' },
  { from: 'docs/content/elohim-protocol/governance-layers-architecture.md',
    to: 'narrative/how/governance-layers.md',
    title: 'I. How / Governance Layers' },
  { from: 'docs/content/elohim-protocol/epr-developer-guide.md',
    to: 'narrative/how/epr-developer-guide.md',
    title: 'I. How / EPR Developer Guide' },
  { from: 'docs/content/elohim-protocol/hardware-spec.md',
    to: 'narrative/how/hardware-spec.md',
    title: 'I. How / Hardware Spec' },
  // II. Foundations
  { from: 'graphos/vocabulary.md',
    to: 'foundations/vocabulary-register.md',
    title: 'II. Foundations / Vocabulary Register' },
  // III. Domains — single-file Reference Design (where genesis content exists)
  { from: 'docs/content/elohim-protocol/lamad.md',
    to: 'domains/learning/reference.md',
    title: 'III. Domains / Learning (Lamad) / Reference Design' },
];

export function validateMappings(mappings, genesisDir) {
  const missing = [];
  for (const m of mappings) {
    if (m.from) {
      const full = join(genesisDir, m.from);
      if (!existsSync(full)) {
        missing.push({ mapping: m, error: `Source file missing: ${full}` });
      }
    }
    // glob mappings handled in a later task
  }
  return missing;
}

export function runSync(mappings, genesisDir, outDir) {
  for (const m of mappings) {
    if (m.from && m.to) {
      const src = join(genesisDir, m.from);
      const dst = join(outDir, m.to);
      mkdirSync(dirname(dst), { recursive: true });
      const content = readFileSync(src, 'utf-8');
      writeFileSync(dst, content);
    }
  }
}

// CLI entrypoint
if (import.meta.url === `file://${process.argv[1]}`) {
  const missing = validateMappings(MAPPINGS, GENESIS_DIR);
  if (missing.length > 0) {
    console.error('sync-genesis: missing source files:');
    for (const m of missing) {
      console.error(`  - ${m.error} (would render as: ${m.mapping.title})`);
    }
    process.exit(1);
  }
  runSync(MAPPINGS, GENESIS_DIR, OUT_DIR);
  // Silent on success per spec.
}
