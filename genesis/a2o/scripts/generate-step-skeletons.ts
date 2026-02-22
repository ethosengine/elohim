#!/usr/bin/env tsx
/**
 * Step Skeleton Generator
 *
 * Runs cucumber-js --dry-run against a tag filter (or all genesis features),
 * parses the snippet output to extract undefined step patterns, deduplicates
 * them, and writes skeleton step definition files to steps/generated/.
 *
 * Usage:
 *   npx tsx scripts/generate-step-skeletons.ts                    # all genesis
 *   npx tsx scripts/generate-step-skeletons.ts --tag @epic:value_scanner
 *   npx tsx scripts/generate-step-skeletons.ts --tag @governance_layer:community
 *   npx tsx scripts/generate-step-skeletons.ts --limit 50         # max 50 steps
 */

import { execSync } from 'node:child_process';
import { mkdir, writeFile, readFile, readdir, access } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const E2E_ROOT = join(__dirname, '..');
const GENERATED_DIR = join(E2E_ROOT, 'steps', 'generated');
const GENESIS_PATHS = '../docs/content/elohim-protocol/**/*.feature';

interface ParsedSnippet {
  keyword: string; // Given, When, Then
  pattern: string; // the step text or regex
  code: string; // full snippet code
}

function parseArgs(): { tag?: string; limit: number } {
  const args = process.argv.slice(2);
  let tag: string | undefined;
  let limit = 200;

  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--tag' && args[i + 1]) {
      tag = args[++i];
    } else if (args[i] === '--limit' && args[i + 1]) {
      limit = Number.parseInt(args[++i], 10);
    }
  }

  return { tag, limit };
}

function runDryRun(tag?: string): string {
  const tagFilter = tag ? `--tags '${tag}'` : '';
  const cmd = ['npx cucumber-js --dry-run', `'${GENESIS_PATHS}'`, tagFilter, '--format snippets']
    .filter(Boolean)
    .join(' ');

  try {
    // eslint-disable-next-line sonarjs/os-command -- intentional CLI invocation of cucumber-js
    return execSync(cmd, {
      cwd: E2E_ROOT,
      encoding: 'utf-8',
      timeout: 120_000,
      // dry-run exits 1 when there are undefined steps; capture stdout
      stdio: ['pipe', 'pipe', 'pipe'],
    });
  } catch (err: unknown) {
    // cucumber-js exits non-zero when steps are undefined; that's expected
    const execErr = err as { stdout?: string; stderr?: string };
    if (execErr.stdout) return execErr.stdout;
    throw err;
  }
}

function parseSnippets(output: string): ParsedSnippet[] {
  const snippets: ParsedSnippet[] = [];
  // Cucumber snippet format for async-await:
  //   Given('pattern', async function () {
  //     // Write code here that turns the phrase above into concrete actions
  //     return 'pending';
  //   });
  const snippetRegex =
    /(Given|When|Then)\('([^']+)',\s*async\s+function\s*\([^)]*\)\s*\{[^}]*\}\);/g;

  let match: RegExpExecArray | null;
  while ((match = snippetRegex.exec(output)) !== null) {
    snippets.push({
      keyword: match[1],
      pattern: match[2],
      code: match[0],
    });
  }

  return snippets;
}

function deduplicateSnippets(snippets: ParsedSnippet[]): ParsedSnippet[] {
  const seen = new Set<string>();
  return snippets.filter(s => {
    // Deduplicate by normalized pattern (ignore parameter placeholders)
    const key = `${s.keyword}:${s.pattern}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

/**
 * Group snippets into logical categories based on common genesis vocabulary.
 */
function categorizeSnippet(s: ParsedSnippet): string {
  const p = s.pattern.toLowerCase();

  // Protocol infrastructure
  if (p.includes('elohim protocol') || p.includes('operational')) return 'protocol';
  // User registration / identity
  if (p.includes('registered') || p.includes('user') || p.includes('identity')) return 'identity';
  // Governance context
  if (p.includes('governance') || p.includes('context is active') || p.includes('leadership'))
    return 'governance';
  // Care economy / tokens
  if (p.includes('care') || p.includes('token') || p.includes('contribution')) return 'economy';
  // Community / social
  if (p.includes('community') || p.includes('cooperative') || p.includes('mutual'))
    return 'community';
  // Agents / elohim AI
  if (p.includes('elohim') && (p.includes('coordinate') || p.includes('agent'))) return 'agents';
  // Content / learning
  if (p.includes('content') || p.includes('path') || p.includes('learn')) return 'learning';

  return 'general';
}

function generateStepFile(category: string, snippets: ParsedSnippet[]): string {
  const imports = new Set<string>();
  for (const s of snippets) {
    imports.add(s.keyword);
  }

  const lines = [
    `/**`,
    ` * Auto-generated step skeletons for: ${category}`,
    ` *`,
    ` * These are placeholder implementations parsed from genesis feature files.`,
    ` * Each step throws PendingException until implemented.`,
    ` * Replace the body with real assertions as scenarios become executable.`,
    ` */`,
    ``,
    `import { ${[...imports].sort((a, b) => a.localeCompare(b)).join(', ')} } from '@cucumber/cucumber';`,
    `import { E2EWorld } from '../../src/framework/world.js';`,
    ``,
  ];

  for (const s of snippets) {
    const escaped = s.pattern.replace(/'/g, String.raw`\'`);
    lines.push(
      `${s.keyword}('${escaped}', async function (this: E2EWorld) {`,
      `  return 'pending';`,
      `});`,
      ``
    );
  }

  return lines.join('\n');
}

async function loadExistingPatterns(): Promise<Set<string>> {
  const patterns = new Set<string>();
  const stepsDir = join(E2E_ROOT, 'steps');

  // Read all .ts files in steps/ (recursively) and extract registered patterns
  async function walk(dir: string): Promise<void> {
    try {
      await access(dir);
    } catch {
      return;
    }
    const entries = await readdir(dir, { withFileTypes: true });
    for (const entry of entries) {
      const full = join(dir, entry.name);
      if (entry.isDirectory()) {
        await walk(full);
      } else if (entry.name.endsWith('.ts')) {
        const content = await readFile(full, 'utf-8');
        // Match Given('...'), When('...'), Then('...')
        const re = /(?:Given|When|Then)\('([^']+)'/g;
        let m: RegExpExecArray | null;
        while ((m = re.exec(content)) !== null) {
          patterns.add(m[1]);
        }
      }
    }
  }

  await walk(stepsDir);
  return patterns;
}

async function main() {
  const { tag, limit } = parseArgs();

  const filterLabel = tag ? ` with filter: ${tag}` : ' (all genesis)';
  console.log(`Running dry-run${filterLabel}...`);
  const output = runDryRun(tag);

  console.log('Parsing undefined step snippets...');
  const raw = parseSnippets(output);
  console.log(`  Found ${raw.length} snippet(s) in output`);

  const deduped = deduplicateSnippets(raw);
  console.log(`  ${deduped.length} unique pattern(s) after deduplication`);

  // Filter out patterns that already have step definitions
  const existing = await loadExistingPatterns();
  const newOnly = deduped.filter(s => !existing.has(s.pattern));
  console.log(`  ${newOnly.length} truly new pattern(s) (${existing.size} already defined)`);

  const limited = newOnly.slice(0, limit);

  if (limited.length === 0) {
    console.log('\nNo new step skeletons to generate.');
    return;
  }

  // Group by category
  const groups = new Map<string, ParsedSnippet[]>();
  for (const s of limited) {
    const cat = categorizeSnippet(s);
    const list = groups.get(cat) ?? [];
    list.push(s);
    groups.set(cat, list);
  }

  // Write files
  await mkdir(GENERATED_DIR, { recursive: true });

  let totalWritten = 0;
  for (const [category, snippets] of groups) {
    const filename = `${category}.steps.ts`;
    const filepath = join(GENERATED_DIR, filename);
    const content = generateStepFile(category, snippets);
    await writeFile(filepath, content);
    totalWritten += snippets.length;
    console.log(`  Wrote ${snippets.length} skeleton(s) to steps/generated/${filename}`);
  }

  console.log(`\nGenerated ${totalWritten} step skeleton(s) in ${groups.size} file(s)`);
  console.log(`Location: steps/generated/`);
}

main().catch(err => {
  console.error('Skeleton generator failed:', err);
  process.exit(1);
});
