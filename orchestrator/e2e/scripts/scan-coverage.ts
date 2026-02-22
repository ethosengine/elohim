#!/usr/bin/env tsx
/**
 * BDD Coverage Gap Scanner
 *
 * Compares conceptual scenarios (genesis feature files) against executable
 * scenarios (Cypress + Cucumber-JS features). Produces a gap report that
 * drives sprint planning and scenario generation.
 *
 * Usage:
 *   npx tsx scripts/scan-coverage.ts              # local dev (read from disk)
 *   npx tsx scripts/scan-coverage.ts --ci         # CI mode (non-interactive)
 *   npx tsx scripts/scan-coverage.ts --from-app https://alpha.elohim.host
 *   npx tsx scripts/scan-coverage.ts --suggest 10 # top 10 actionable stubs
 */

import { execSync } from 'node:child_process';
import { access, mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { parseExecutableFeatures } from './parsers/executable-features.js';
import { parseConceptualFeatures } from './parsers/gherkin-tags.js';

import type {
  ConceptualScenario,
  CoverageGapReport,
  DomainGap,
  ExecutableScenario,
  FeatureReadiness,
  GovernanceGap,
  PrioritizedGap,
} from './types.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '../../..');
const E2E_ROOT = join(__dirname, '..');
const REPORT_DIR = join(__dirname, '..', 'reports');
const REPORT_PATH = join(REPORT_DIR, 'coverage-gap-report.json');

// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------

interface CliArgs {
  ci: boolean;
  fromApp?: string;
  suggest?: number;
}

function parseCliArgs(): CliArgs {
  const args = process.argv.slice(2);
  const result: CliArgs = { ci: false };

  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--ci') result.ci = true;
    else if (args[i] === '--from-app' && args[i + 1]) result.fromApp = args[++i];
    else if (args[i] === '--suggest' && args[i + 1])
      result.suggest = Number.parseInt(args[++i], 10);
  }

  return result;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function getGitCommit(): string {
  try {
    // eslint-disable-next-line sonarjs/no-os-command-from-path -- git is a standard system tool
    return execSync('git rev-parse --short HEAD', { cwd: REPO_ROOT, encoding: 'utf-8' }).trim();
  } catch {
    return 'unknown';
  }
}

function unique<T>(arr: T[]): T[] {
  return [...new Set(arr)];
}

// ---------------------------------------------------------------------------
// --from-app: fetch gherkin ContentNodes from a running doorway
// ---------------------------------------------------------------------------

async function fetchConceptualFromApp(appUrl: string): Promise<ConceptualScenario[]> {
  // Resolve doorway URL from app URL
  const doorwayUrl = appUrl
    .replace('://alpha.', '://doorway-alpha.')
    .replace('://staging.', '://doorway-staging.')
    .replace('://elohim.host', '://doorway.elohim.host');

  console.log(`  Fetching gherkin content from ${doorwayUrl}...`);

  const { request } = await import('undici');
  const { statusCode, body } = await request(`${doorwayUrl}/api/v1/cache/content?tags=gherkin`, {
    method: 'GET',
    headers: { 'content-type': 'application/json' },
  });

  const text = await body.text();
  if (statusCode < 200 || statusCode >= 300) {
    throw new Error(`Failed to fetch from ${doorwayUrl}: ${statusCode} ${text.slice(0, 200)}`);
  }

  const nodes = JSON.parse(text) as {
    id: string;
    title: string;
    content: string;
    tags: string[];
  }[];

  console.log(`  Fetched ${nodes.length} gherkin ContentNode(s) from app`);

  const scenarios: ConceptualScenario[] = [];
  for (const node of nodes) {
    const lines = (node.content ?? '').split('\n');
    const epic = extractTag(lines, 'epic');
    const userType = extractTag(lines, 'user_type');
    const governanceLayer = extractTag(lines, 'governance_layer');

    const names = extractScenarioNames(node.content ?? '');
    for (const name of names) {
      scenarios.push({
        name,
        featureFile: `app:${node.id}`,
        epic,
        userType,
        governanceLayer,
      });
    }
  }

  return scenarios;
}

function extractTag(lines: string[], key: string): string {
  for (const line of lines) {
    const match = new RegExp(`^@${key}:(.+)$`).exec(line);
    if (match) return match[1].trim();
  }
  return 'unknown';
}

function extractScenarioNames(content: string): string[] {
  const re = /^\s*Scenario(?:\s+Outline)?:\s*(.+)$/; // eslint-disable-line sonarjs/slow-regex
  const names: string[] = [];
  for (const line of content.split('\n')) {
    const match = re.exec(line);
    if (match) names.push(match[1].trim());
  }
  return names;
}

// ---------------------------------------------------------------------------
// Step definition readiness scoring
// ---------------------------------------------------------------------------

async function loadStepPatterns(): Promise<string[]> {
  const patterns: string[] = [];
  const stepsDir = join(E2E_ROOT, 'steps');

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
        const fileContent = await readFile(full, 'utf-8');
        const re = /(?:Given|When|Then)\('([^']+)'/g;
        let m: RegExpExecArray | null;
        while ((m = re.exec(fileContent)) !== null) {
          patterns.push(m[1]);
        }
      }
    }
  }

  await walk(stepsDir);
  return patterns;
}

function stepMatchesPattern(stepText: string, pattern: string): boolean {
  // Convert cucumber pattern placeholders to regex
  // {string} -> matches quoted string, {int} -> matches number, {word} -> matches word, {float} -> matches float
  const regexStr = pattern
    .replace(/\{string\}/g, '"[^"]*"')
    .replace(/\{int\}/g, String.raw`\d+`)
    .replace(/\{word\}/g, String.raw`\w+`)
    .replace(/\{float\}/g, String.raw`[\d.]+`);

  try {
    return new RegExp(`^${regexStr}$`).test(stepText);
  } catch {
    return stepText === pattern;
  }
}

// eslint-disable-next-line sonarjs/slow-regex
const STEP_LINE_RE = /^\s*(?:Given|When|Then|And|But)\s+(.+)$/;

function extractStepLines(content: string): string[] {
  const lines: string[] = [];
  for (const line of content.split('\n')) {
    const match = STEP_LINE_RE.exec(line);
    if (match) lines.push(match[1].trim());
  }
  return lines;
}

async function scoreFeatureFile(
  featureFile: string,
  stepPatterns: string[]
): Promise<FeatureReadiness | null> {
  if (featureFile.startsWith('app:')) return null;

  const fullPath = join(REPO_ROOT, featureFile);
  let content: string;
  try {
    content = await readFile(fullPath, 'utf-8');
  } catch {
    return null;
  }

  const stepLines = extractStepLines(content);
  const totalSteps = stepLines.length;
  const matchedSteps = stepLines.filter(step =>
    stepPatterns.some(p => stepMatchesPattern(step, p))
  ).length;

  return {
    featureFile,
    totalSteps,
    matchedSteps,
    readinessPercent: totalSteps > 0 ? +((matchedSteps / totalSteps) * 100).toFixed(1) : 0,
  };
}

async function buildFeatureReadiness(
  conceptual: ConceptualScenario[]
): Promise<FeatureReadiness[]> {
  const stepPatterns = await loadStepPatterns();

  // Group scenarios by feature file
  const byFile = new Map<string, ConceptualScenario[]>();
  for (const s of conceptual) {
    const list = byFile.get(s.featureFile) ?? [];
    list.push(s);
    byFile.set(s.featureFile, list);
  }

  const readiness: FeatureReadiness[] = [];
  for (const [featureFile] of byFile) {
    const score = await scoreFeatureFile(featureFile, stepPatterns);
    if (score) readiness.push(score);
  }

  readiness.sort((a, b) => b.readinessPercent - a.readinessPercent);
  return readiness;
}

// ---------------------------------------------------------------------------
// Gap builders (unchanged logic)
// ---------------------------------------------------------------------------

function buildDomainGaps(
  conceptual: ConceptualScenario[],
  executable: ExecutableScenario[]
): DomainGap[] {
  const epicMap = new Map<string, ConceptualScenario[]>();
  for (const s of conceptual) {
    const list = epicMap.get(s.epic) ?? [];
    list.push(s);
    epicMap.set(s.epic, list);
  }

  const executableByTag = new Map<string, ExecutableScenario[]>();
  for (const s of executable) {
    for (const tag of s.tags) {
      const epicMatch = /^@epic:(.+)$/.exec(tag);
      const key = epicMatch ? epicMatch[1] : tag.replace(/^@/, '');
      const list = executableByTag.get(key) ?? [];
      list.push(s);
      executableByTag.set(key, list);
    }
  }

  const gaps: DomainGap[] = [];
  for (const [epic, scenarios] of epicMap) {
    const execScenarios = executableByTag.get(epic) ?? [];
    const conceptualCount = scenarios.length;
    const executableCount = execScenarios.length;

    gaps.push({
      epic,
      conceptualCount,
      executableCount,
      coveragePercent:
        conceptualCount > 0 ? +((executableCount / conceptualCount) * 100).toFixed(2) : 0,
      userTypes: unique(scenarios.map(s => s.userType)),
      governanceLayers: unique(scenarios.map(s => s.governanceLayer)),
      sampleScenarios: scenarios.slice(0, 5).map(s => s.name),
    });
  }

  gaps.sort((a, b) => b.conceptualCount - a.conceptualCount);
  return gaps;
}

function buildGovernanceGaps(
  conceptual: ConceptualScenario[],
  executable: ExecutableScenario[]
): GovernanceGap[] {
  const layerMap = new Map<string, ConceptualScenario[]>();
  for (const s of conceptual) {
    const list = layerMap.get(s.governanceLayer) ?? [];
    list.push(s);
    layerMap.set(s.governanceLayer, list);
  }

  const executableLayers = new Set<string>();
  for (const s of executable) {
    for (const tag of s.tags) {
      const layerMatch = /^@governance_layer:(.+)$/.exec(tag);
      if (layerMatch) executableLayers.add(layerMatch[1]);
    }
  }

  const gaps: GovernanceGap[] = [];
  for (const [layer, scenarios] of layerMap) {
    const execCount = executableLayers.has(layer)
      ? executable.filter(s => s.tags.some(t => t === `@governance_layer:${layer}`)).length
      : 0;

    gaps.push({
      layer,
      conceptualCount: scenarios.length,
      executableCount: execCount,
      epics: unique(scenarios.map(s => s.epic)),
    });
  }

  gaps.sort((a, b) => b.conceptualCount - a.conceptualCount);
  return gaps;
}

function buildPrioritizedGaps(
  domainGaps: DomainGap[],
  governanceGaps: GovernanceGap[]
): PrioritizedGap[] {
  const candidates: {
    epic: string;
    layer: string;
    density: number;
  }[] = [];

  for (const dg of domainGaps) {
    if (dg.executableCount > 0) continue;
    for (const layer of dg.governanceLayers) {
      const gg = governanceGaps.find(g => g.layer === layer);
      if (gg?.executableCount === 0) {
        candidates.push({
          epic: dg.epic,
          layer,
          density: dg.conceptualCount,
        });
      }
    }
  }

  const seen = new Set<string>();
  const deduped = candidates.filter(c => {
    const key = `${c.epic}:${c.layer}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
  deduped.sort((a, b) => b.density - a.density);

  return deduped.slice(0, 20).map((c, i) => ({
    rank: i + 1,
    domain: c.epic,
    governanceLayer: c.layer,
    conceptualDensity: c.density,
    suggestedFeatureFile: `orchestrator/e2e/features/${c.epic}/${c.layer}.feature`,
    rationale: `${c.epic} has ${c.density} conceptual scenarios at the ${c.layer} layer with zero executable tests. High density = high value for first coverage.`,
  }));
}

// ---------------------------------------------------------------------------
// --suggest: output actionable stubs
// ---------------------------------------------------------------------------

function printSuggestions(
  prioritizedGaps: PrioritizedGap[],
  readiness: FeatureReadiness[],
  count: number
): void {
  console.log(`\n--- Top ${count} Suggested Next Steps ---`);

  // Prioritize features with highest readiness (easiest to make executable)
  const readyFeatures = readiness.filter(r => r.readinessPercent > 0).slice(0, count);

  if (readyFeatures.length > 0) {
    console.log('\nNearest to executable (have some step definitions):');
    for (const f of readyFeatures) {
      console.log(`  ${f.featureFile}`);
      console.log(
        `    ${f.matchedSteps}/${f.totalSteps} steps matched (${f.readinessPercent}% ready)`
      );
    }
  }

  console.log('\nHighest-impact gaps (zero coverage, most scenarios):');
  for (const gap of prioritizedGaps.slice(0, count)) {
    console.log(`  ${gap.rank}. ${gap.domain} / ${gap.governanceLayer}`);
    console.log(`     ${gap.conceptualDensity} scenarios, suggested: ${gap.suggestedFeatureFile}`);
  }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
  const { ci: isCI, fromApp, suggest } = parseCliArgs();

  let conceptual: ConceptualScenario[];

  if (fromApp) {
    console.log(`Fetching conceptual scenarios from app: ${fromApp}...`);
    conceptual = await fetchConceptualFromApp(fromApp);
  } else {
    console.log('Scanning conceptual scenarios (genesis)...');
    conceptual = await parseConceptualFeatures(REPO_ROOT);
  }
  console.log(`  Found ${conceptual.length} conceptual scenarios`);

  console.log('Scanning executable scenarios (cypress + cucumber-js)...');
  const executable = await parseExecutableFeatures(REPO_ROOT);
  console.log(`  Found ${executable.length} executable scenarios`);

  const domainGaps = buildDomainGaps(conceptual, executable);
  const governanceGaps = buildGovernanceGaps(conceptual, executable);
  const prioritizedGaps = buildPrioritizedGaps(domainGaps, governanceGaps);

  // Feature readiness scoring
  console.log('Computing feature readiness scores...');
  const featureReadiness = await buildFeatureReadiness(conceptual);
  const stepPatterns = await loadStepPatterns();
  const readyCount = featureReadiness.filter(r => r.readinessPercent > 0).length;
  console.log(
    `  ${stepPatterns.length} step patterns defined, ${readyCount} features have partial readiness`
  );

  const epicsWithZeroTests = domainGaps.filter(d => d.executableCount === 0).map(d => d.epic);
  const governanceLayersWithZeroTests = governanceGaps
    .filter(g => g.executableCount === 0)
    .map(g => g.layer);

  const coveragePercent =
    conceptual.length > 0 ? +((executable.length / conceptual.length) * 100).toFixed(2) : 0;

  const report: CoverageGapReport = {
    generatedAt: new Date().toISOString(),
    gitCommit: getGitCommit(),
    summary: {
      totalConceptualScenarios: conceptual.length,
      totalExecutableScenarios: executable.length,
      coveragePercent,
      epicsWithZeroTests,
      governanceLayersWithZeroTests,
      totalDefinedStepPatterns: stepPatterns.length,
    },
    domainGaps,
    governanceGaps,
    prioritizedGaps,
    featureReadiness: featureReadiness.slice(0, 50), // top 50 by readiness
  };

  await mkdir(REPORT_DIR, { recursive: true });
  await writeFile(REPORT_PATH, JSON.stringify(report, null, 2) + '\n');
  console.log(`\nReport written to ${REPORT_PATH}`);

  // Print summary
  console.log('\n--- Coverage Gap Summary ---');
  console.log(`Conceptual scenarios: ${report.summary.totalConceptualScenarios}`);
  console.log(`Executable scenarios: ${report.summary.totalExecutableScenarios}`);
  console.log(`Coverage: ${report.summary.coveragePercent}%`);
  console.log(`Step patterns defined: ${stepPatterns.length}`);
  console.log(`Epics with zero tests: ${report.summary.epicsWithZeroTests.join(', ')}`);
  console.log(
    `Top priority gap: ${prioritizedGaps[0]?.domain ?? 'none'} (${prioritizedGaps[0]?.governanceLayer ?? 'n/a'})`
  );

  if (suggest) {
    printSuggestions(prioritizedGaps, featureReadiness, suggest);
  }

  if (isCI && report.summary.coveragePercent < 1) {
    console.log('\n[CI] Coverage below 1% threshold (advisory only, not failing build)');
  }
}

main().catch(err => {
  console.error('Scanner failed:', err);
  process.exit(1);
});
