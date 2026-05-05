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

import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { basename, dirname, extname, join, resolve } from 'node:path';
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

export function gherkinToMarkdown(featureContent, fileName) {
  // Wrap the raw .feature in a fenced code block with `gherkin` language.
  // Storybook's Markdown block handles syntax highlighting via prismjs.
  const titleMatch = featureContent.match(/^Feature:\s*(.+)$/m);
  const heading = titleMatch ? `# ${titleMatch[1].trim()}` : `# ${fileName}`;
  return `${heading}\n\n_Source: \`${fileName}\`_\n\n\`\`\`gherkin\n${featureContent.trimEnd()}\n\`\`\`\n`;
}

export function expandGlob(pattern, baseDir) {
  // Minimal glob: supports `<segments>/*.<ext>` only — no `**`, no character classes.
  // Add complexity only if a future mapping requires it.
  const parts = pattern.split('/');
  const fileGlob = parts.pop();
  const dir = join(baseDir, parts.join('/'));
  if (!existsSync(dir) || !statSync(dir).isDirectory()) return [];
  const ext = fileGlob.replace(/^\*/, '');
  return readdirSync(dir)
    .filter(name => name.endsWith(ext) && !name.startsWith('.'))
    .map(name => join(dir, name));
}

function toTitleCase(slug) {
  return slug
    .replace(/[-_]/g, ' ')
    .replace(/\b\w/g, c => c.toUpperCase());
}

function relativeFromTo(fromDir, toDir, slug) {
  // Both are relative to the same root (graphos/src). Compute the relative
  // path between them, then append the file.
  const fromParts = fromDir.split('/').filter(Boolean);
  // Go up from fromParts to graphos/src root, then descend into imported/<toDir>/<slug>.md
  const upDirs = fromParts.length;
  const ups = '../'.repeat(upDirs);
  const toParts = toDir.split('/').filter(Boolean);
  return `${ups}imported/${toParts.join('/')}/${slug}.md`;
}

export function runSyncWithGlobs(mappings, genesisDir, outDir, wrappersBase) {
  for (const m of mappings) {
    if (!m.fromGlob) continue;
    const matches = expandGlob(m.fromGlob, genesisDir);
    for (const sourcePath of matches) {
      const fileName = basename(sourcePath);
      const slug = fileName.replace(/\.feature$/, '');
      const niceName = toTitleCase(slug);
      const featureContent = readFileSync(sourcePath, 'utf-8');
      const md = gherkinToMarkdown(featureContent, fileName);

      // 1. Write the imported markdown content
      const mdDest = join(outDir, m.toDir, `${slug}.md`);
      mkdirSync(dirname(mdDest), { recursive: true });
      writeFileSync(mdDest, md);

      // 2. Write the generated MDX wrapper.
      // Wrapper lives under wrappersBase/<sectionPath>/<slug>.mdx
      // toDir example: 'domains/identity/stories/' → 'domains/identity/__docs__/_generated/'
      const sectionPath = m.toDir.replace(/\/[^/]+\/?$/, '/__docs__/_generated/');
      const mdxDest = join(wrappersBase, sectionPath, `${slug}.mdx`);
      mkdirSync(dirname(mdxDest), { recursive: true });
      const title = m.titleFn(niceName);
      const importRelPath = relativeFromTo(sectionPath, m.toDir, slug);
      const mdxContent = `import { Meta, Markdown } from '@storybook/addon-docs/blocks';
import content from '${importRelPath}?raw';

<Meta title="${title}" />

<Markdown>{content}</Markdown>
`;
      writeFileSync(mdxDest, mdxContent);
    }
  }
}

// CLI entrypoint
const WRAPPERS_BASE = resolve(__dirname, '..', 'projects', 'graphos', 'src');

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
  runSyncWithGlobs(MAPPINGS, GENESIS_DIR, OUT_DIR, WRAPPERS_BASE);
  // Silent on success per spec.
}
