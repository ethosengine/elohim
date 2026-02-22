#!/usr/bin/env tsx
/**
 * BDD Coverage Gap Scanner
 *
 * Compares conceptual scenarios (genesis feature files) against executable
 * scenarios (Cypress + Cucumber-JS features). Produces a gap report that
 * drives sprint planning and scenario generation.
 *
 * Usage:
 *   npx tsx scripts/scan-coverage.ts          # local dev
 *   npx tsx scripts/scan-coverage.ts --ci     # CI mode (non-interactive)
 */

import { execSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseConceptualFeatures } from './parsers/gherkin-tags.js';
import { parseExecutableFeatures } from './parsers/executable-features.js';
import type {
  ConceptualScenario,
  CoverageGapReport,
  DomainGap,
  ExecutableScenario,
  GovernanceGap,
  PrioritizedGap,
} from './types.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '../../..');
const REPORT_DIR = join(__dirname, '..', 'reports');
const REPORT_PATH = join(REPORT_DIR, 'coverage-gap-report.json');

function getGitCommit(): string {
  try {
    return execSync('git rev-parse --short HEAD', { cwd: REPO_ROOT, encoding: 'utf-8' }).trim();
  } catch {
    return 'unknown';
  }
}

function unique<T>(arr: T[]): T[] {
  return [...new Set(arr)];
}

function buildDomainGaps(
  conceptual: ConceptualScenario[],
  executable: ExecutableScenario[],
): DomainGap[] {
  const epicMap = new Map<string, ConceptualScenario[]>();
  for (const s of conceptual) {
    const list = epicMap.get(s.epic) ?? [];
    list.push(s);
    epicMap.set(s.epic, list);
  }

  // Build a set of executable tags/epics for matching.
  // Executable features may tag with @epic:value or just @domain-name.
  const executableByTag = new Map<string, ExecutableScenario[]>();
  for (const s of executable) {
    for (const tag of s.tags) {
      const epicMatch = tag.match(/^@epic:(.+)$/);
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
      coveragePercent: conceptualCount > 0 ? +((executableCount / conceptualCount) * 100).toFixed(2) : 0,
      userTypes: unique(scenarios.map((s) => s.userType)),
      governanceLayers: unique(scenarios.map((s) => s.governanceLayer)),
      sampleScenarios: scenarios.slice(0, 5).map((s) => s.name),
    });
  }

  // Sort by conceptual count descending (biggest gaps first)
  gaps.sort((a, b) => b.conceptualCount - a.conceptualCount);
  return gaps;
}

function buildGovernanceGaps(
  conceptual: ConceptualScenario[],
  executable: ExecutableScenario[],
): GovernanceGap[] {
  const layerMap = new Map<string, ConceptualScenario[]>();
  for (const s of conceptual) {
    const list = layerMap.get(s.governanceLayer) ?? [];
    list.push(s);
    layerMap.set(s.governanceLayer, list);
  }

  // Check which governance layers have any executable coverage
  const executableLayers = new Set<string>();
  for (const s of executable) {
    for (const tag of s.tags) {
      const layerMatch = tag.match(/^@governance_layer:(.+)$/);
      if (layerMatch) executableLayers.add(layerMatch[1]);
    }
  }

  const gaps: GovernanceGap[] = [];
  for (const [layer, scenarios] of layerMap) {
    const execCount = executableLayers.has(layer)
      ? executable.filter((s) => s.tags.some((t) => t === `@governance_layer:${layer}`)).length
      : 0;

    gaps.push({
      layer,
      conceptualCount: scenarios.length,
      executableCount: execCount,
      epics: unique(scenarios.map((s) => s.epic)),
    });
  }

  gaps.sort((a, b) => b.conceptualCount - a.conceptualCount);
  return gaps;
}

function buildPrioritizedGaps(domainGaps: DomainGap[], governanceGaps: GovernanceGap[]): PrioritizedGap[] {
  // Cross-product: for each epic x governance layer with zero tests, create a
  // prioritized gap entry. Rank by conceptual density (more scenarios = higher priority).
  const candidates: Array<{
    epic: string;
    layer: string;
    density: number;
  }> = [];

  for (const dg of domainGaps) {
    if (dg.executableCount > 0) continue; // already has some coverage
    for (const layer of dg.governanceLayers) {
      const gg = governanceGaps.find((g) => g.layer === layer);
      if (gg && gg.executableCount === 0) {
        candidates.push({
          epic: dg.epic,
          layer,
          density: dg.conceptualCount,
        });
      }
    }
  }

  // Deduplicate and sort by density
  const seen = new Set<string>();
  const deduped = candidates.filter((c) => {
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

async function main() {
  const isCI = process.argv.includes('--ci');

  console.log('Scanning conceptual scenarios (genesis)...');
  const conceptual = await parseConceptualFeatures(REPO_ROOT);
  console.log(`  Found ${conceptual.length} conceptual scenarios`);

  console.log('Scanning executable scenarios (cypress + cucumber-js)...');
  const executable = await parseExecutableFeatures(REPO_ROOT);
  console.log(`  Found ${executable.length} executable scenarios`);

  const domainGaps = buildDomainGaps(conceptual, executable);
  const governanceGaps = buildGovernanceGaps(conceptual, executable);
  const prioritizedGaps = buildPrioritizedGaps(domainGaps, governanceGaps);

  const epicsWithZeroTests = domainGaps.filter((d) => d.executableCount === 0).map((d) => d.epic);
  const governanceLayersWithZeroTests = governanceGaps
    .filter((g) => g.executableCount === 0)
    .map((g) => g.layer);

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
    },
    domainGaps,
    governanceGaps,
    prioritizedGaps,
  };

  await mkdir(REPORT_DIR, { recursive: true });
  await writeFile(REPORT_PATH, JSON.stringify(report, null, 2) + '\n');
  console.log(`\nReport written to ${REPORT_PATH}`);

  // Print summary
  console.log('\n--- Coverage Gap Summary ---');
  console.log(`Conceptual scenarios: ${report.summary.totalConceptualScenarios}`);
  console.log(`Executable scenarios: ${report.summary.totalExecutableScenarios}`);
  console.log(`Coverage: ${report.summary.coveragePercent}%`);
  console.log(`Epics with zero tests: ${report.summary.epicsWithZeroTests.join(', ')}`);
  console.log(`Top priority gap: ${prioritizedGaps[0]?.domain ?? 'none'} (${prioritizedGaps[0]?.governanceLayer ?? 'n/a'})`);

  if (isCI && report.summary.coveragePercent < 1) {
    console.log('\n[CI] Coverage below 1% threshold (advisory only, not failing build)');
  }
}

main().catch((err) => {
  console.error('Scanner failed:', err);
  process.exit(1);
});
