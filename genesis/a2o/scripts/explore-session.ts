#!/usr/bin/env tsx
/**
 * Exploration Session Runner
 *
 * Reads a genesis feature file (or selects one by tag), presents each
 * Given/When/Then step for interactive verification, and records session
 * metadata as a JSON report.
 *
 * Designed to be used with Claude Code + Claude-in-Chrome for AI-driven
 * exploratory testing of aspirational scenarios.
 *
 * Usage:
 *   npx tsx scripts/explore-session.ts <feature-file>
 *   npx tsx scripts/explore-session.ts --tag @epic:value_scanner --pick
 *   npx tsx scripts/explore-session.ts --from-report reports/coverage-gap-report.json
 *
 * Output: reports/exploration-sessions/<timestamp>.json
 */

import { readFile, writeFile, mkdir, readdir } from 'node:fs/promises';
import { dirname, join, resolve, basename } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '../../..');
const SESSIONS_DIR = join(__dirname, '..', 'reports', 'exploration-sessions');

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface StepObservation {
  step: string;
  keyword: string;
  status: 'pass' | 'fail' | 'partial' | 'blocked' | 'skipped';
  observation: string;
  screenshotPath?: string;
  qualitative?: string; // for aspirational steps
}

export interface ExplorationSession {
  id: string;
  timestamp: string;
  scenario: string;
  featureFile: string;
  targetUrl: string;
  tags: string[];
  steps: StepObservation[];
  verdict: 'pass' | 'fail' | 'partial' | 'blocked';
  feedback: string;
  duration_ms: number;
  gifPath?: string;
}

// ---------------------------------------------------------------------------
// Feature file parsing
// ---------------------------------------------------------------------------

interface ParsedScenario {
  name: string;
  tags: string[];
  steps: { keyword: string; text: string }[];
}

// eslint-disable-next-line sonarjs/duplicates-in-character-class
const TAG_RE = /@[\w:_-]+/g;
const SCENARIO_RE = /^Scenario(?:\s+Outline)?:/;
// eslint-disable-next-line sonarjs/slow-regex
const STEP_RE = /^(Given|When|Then|And|But)\s+(.+)$/;

function extractFileTags(lines: string[]): { tags: string[]; startIndex: number } {
  const tags: string[] = [];
  let i = 0;
  while (i < lines.length && !lines[i].trimStart().startsWith('Feature:')) {
    const tagMatch = lines[i].match(TAG_RE);
    if (tagMatch) tags.push(...tagMatch);
    i++;
  }
  return { tags, startIndex: i };
}

function buildScenario(
  trimmed: string,
  prevLine: string | undefined,
  fileTags: string[],
  backgroundSteps: { keyword: string; text: string }[]
): ParsedScenario {
  const name = trimmed.replace(/^Scenario(?:\s+Outline)?:\s*/, '');
  const scenarioTags: string[] = [];
  if (prevLine) {
    const prevMatch = prevLine.match(TAG_RE);
    if (prevMatch) scenarioTags.push(...prevMatch);
  }
  return {
    name,
    tags: [...fileTags, ...scenarioTags],
    steps: [...backgroundSteps],
  };
}

function parseFeatureFile(content: string): ParsedScenario[] {
  const lines = content.split('\n');
  const scenarios: ParsedScenario[] = [];
  const { tags: fileTags, startIndex } = extractFileTags(lines);

  let currentScenario: ParsedScenario | null = null;
  let inBackground = false;
  const backgroundSteps: { keyword: string; text: string }[] = [];

  for (let i = startIndex; i < lines.length; i++) {
    const trimmed = lines[i].trimStart();

    if (trimmed.startsWith('Background:')) {
      inBackground = true;
      continue;
    }

    if (SCENARIO_RE.exec(trimmed)) {
      inBackground = false;
      if (currentScenario) scenarios.push(currentScenario);
      currentScenario = buildScenario(trimmed, lines[i - 1], fileTags, backgroundSteps);
      continue;
    }

    const stepMatch = STEP_RE.exec(trimmed);
    if (stepMatch) {
      const step = { keyword: stepMatch[1], text: stepMatch[2] };
      if (inBackground) backgroundSteps.push(step);
      else if (currentScenario) currentScenario.steps.push(step);
    }
  }

  if (currentScenario) scenarios.push(currentScenario);
  return scenarios;
}

// ---------------------------------------------------------------------------
// Gap-based scenario selection
// ---------------------------------------------------------------------------

async function selectFromGapReport(reportPath: string): Promise<string | null> {
  const report = JSON.parse(await readFile(reportPath, 'utf-8'));
  const gaps = report.prioritizedGaps as {
    domain: string;
    governanceLayer: string;
    suggestedFeatureFile: string;
  }[];

  if (gaps.length === 0) return null;

  // Find the first gap that has a corresponding genesis feature file
  const genesisDir = join(REPO_ROOT, 'genesis/docs/content/elohim-protocol');

  for (const gap of gaps) {
    const searchDir = join(genesisDir, gap.domain);
    try {
      const features = await findFeatureFiles(searchDir);
      if (features.length > 0) {
        // Pick one at random for variety
        return features[Math.floor(Math.random() * features.length)]; // eslint-disable-line sonarjs/pseudo-random
      }
    } catch {
      continue;
    }
  }

  return null;
}

async function findFeatureFiles(dir: string): Promise<string[]> {
  const files: string[] = [];
  try {
    const entries = await readdir(dir, { withFileTypes: true });
    for (const entry of entries) {
      const full = join(dir, entry.name);
      if (entry.isDirectory()) {
        files.push(...(await findFeatureFiles(full)));
      } else if (entry.name.endsWith('.feature')) {
        files.push(full);
      }
    }
  } catch {
    // directory doesn't exist
  }
  return files;
}

// ---------------------------------------------------------------------------
// Session creation
// ---------------------------------------------------------------------------

function createEmptySession(
  scenario: ParsedScenario,
  featureFile: string,
  targetUrl: string
): ExplorationSession {
  const id = `explore-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`; // eslint-disable-line sonarjs/pseudo-random

  return {
    id,
    timestamp: new Date().toISOString(),
    scenario: scenario.name,
    featureFile,
    targetUrl,
    tags: scenario.tags,
    steps: scenario.steps.map(s => ({
      step: `${s.keyword} ${s.text}`,
      keyword: s.keyword,
      status: 'skipped',
      observation: '',
    })),
    verdict: 'blocked',
    feedback: '',
    duration_ms: 0,
  };
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

interface CliArgs {
  featureFile?: string;
  tag?: string;
  fromReport?: string;
  targetUrl: string;
  pick: boolean;
}

function parseArgs(): CliArgs {
  const args = process.argv.slice(2);
  const result: CliArgs = {
    targetUrl: process.env['E2E_TARGET_URL'] ?? 'https://alpha.elohim.host',
    pick: false,
  };

  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--tag' && args[i + 1]) result.tag = args[++i];
    else if (args[i] === '--from-report' && args[i + 1]) result.fromReport = args[++i];
    else if (args[i] === '--target' && args[i + 1]) result.targetUrl = args[++i];
    else if (args[i] === '--pick') result.pick = true;
    else if (!args[i].startsWith('-')) result.featureFile = args[i];
  }

  return result;
}

async function main() {
  const cliArgs = parseArgs();

  let featurePath: string;

  if (cliArgs.featureFile) {
    featurePath = resolve(cliArgs.featureFile);
  } else if (cliArgs.fromReport) {
    const selected = await selectFromGapReport(resolve(cliArgs.fromReport));
    if (!selected) {
      console.error('No suitable feature file found from gap report');
      process.exit(1);
    }
    featurePath = selected;
  } else {
    console.error('Usage: explore-session.ts <feature-file> | --from-report <path> | --tag <tag>');
    process.exit(1);
  }

  console.log(`\nExploration Session`);
  console.log(`Feature: ${featurePath}`);
  console.log(`Target: ${cliArgs.targetUrl}`);
  console.log('---');

  const content = await readFile(featurePath, 'utf-8');
  const scenarios = parseFeatureFile(content);

  if (scenarios.length === 0) {
    console.error('No scenarios found in feature file');
    process.exit(1);
  }

  console.log(`Found ${scenarios.length} scenario(s):\n`);
  for (let i = 0; i < scenarios.length; i++) {
    const s = scenarios[i];
    console.log(`  ${i + 1}. ${s.name} (${s.steps.length} steps)`);
  }

  // Create session templates for all scenarios
  await mkdir(SESSIONS_DIR, { recursive: true });
  const sessions: ExplorationSession[] = [];

  for (const scenario of scenarios) {
    const session = createEmptySession(scenario, basename(featurePath), cliArgs.targetUrl);
    sessions.push(session);

    console.log(`\n--- Scenario: ${scenario.name} ---`);
    for (const step of scenario.steps) {
      console.log(`  ${step.keyword} ${step.text}`);
    }
  }

  // Write session template
  const outputPath = join(SESSIONS_DIR, `${sessions[0].id}.json`);
  await writeFile(outputPath, JSON.stringify(sessions, null, 2) + '\n');
  console.log(`\nSession template written to: ${outputPath}`);
  console.log('\nTo complete this session interactively with Claude Code:');
  console.log('  1. Open the feature file and target URL in the browser');
  console.log('  2. For each step, attempt verification and record observations');
  console.log('  3. Update the session JSON with results');
  console.log('  4. Run publish-results.ts to seed feedback into the content graph');
}

main().catch(err => {
  console.error('Exploration session failed:', err);
  process.exit(1);
});
