#!/usr/bin/env tsx
/**
 * Extract failed scenarios from cucumber-report.json into a structured feed
 * that an agentic-developer shift can consume as Objective input.
 *
 * Reads:   genesis/a2o/reports/cucumber-report.json
 *          genesis/a2o/reports/{screenshots,console,observations,traces}/
 * Writes:  .claude/data/a2o-failures.jsonl  (append-only, one JSON record per failure)
 *
 * Each record carries enough context for an Opus-level operator to open a sprint:
 * scenario identity, failure step, error message, and absolute paths to every
 * captured artifact (screenshot, console errors, observation report, trace).
 *
 * Usage:
 *   npx tsx scripts/extract-failures.ts
 *   npx tsx scripts/extract-failures.ts --report path/to/cucumber.json --out path/to/failures.jsonl
 */

import { existsSync, mkdirSync, readFileSync, appendFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const A2O_ROOT = resolve(__dirname, '..');
const REPO_ROOT = resolve(A2O_ROOT, '..', '..');

interface CucumberStep {
  keyword: string;
  name?: string;
  line?: number;
  result: { status: string; duration?: number; error_message?: string };
  match?: { location?: string };
}

interface CucumberElement {
  id: string;
  name: string;
  line: number;
  keyword: string;
  type?: string;
  tags?: { name: string }[];
  steps: CucumberStep[];
}

interface CucumberFeature {
  uri: string;
  name: string;
  elements: CucumberElement[];
}

interface FailureRecord {
  timestamp: string;
  feature: { uri: string; name: string };
  scenario: { name: string; line: number; tags: string[]; id: string };
  failedStep: { keyword: string; name: string; location?: string; line?: number } | null;
  errorMessage: string;
  artifacts: {
    screenshot?: string;
    consoleErrors?: string;
    consoleAll?: string;
    observation?: string;
    trace?: string;
  };
  reproCommand: string;
}

function parseArgs(argv: string[]): { report: string; out: string } {
  const args = new Map<string, string>();
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a?.startsWith('--')) args.set(a.slice(2), argv[i + 1] ?? '');
  }
  return {
    report: args.get('report') ?? join(A2O_ROOT, 'reports', 'cucumber-report.json'),
    out: args.get('out') ?? join(REPO_ROOT, '.claude', 'data', 'a2o-failures.jsonl'),
  };
}

function safeName(s: string): string {
  return s.replace(/[^a-zA-Z0-9]/g, '-');
}

function findArtifact(dir: string, prefix: string, ext: string): string | undefined {
  if (!existsSync(dir)) return undefined;
  // Match exactly: `${prefix}.${ext}` or `${prefix}-${human}.${ext}`
  // We already include the FAIL- prefix in screenshots/traces; observations/console may not.
  const path = join(dir, `${prefix}.${ext}`);
  if (existsSync(path)) return path;
  return undefined;
}

function collectArtifacts(scenarioName: string): FailureRecord['artifacts'] {
  const safe = safeName(scenarioName);
  const reports = join(A2O_ROOT, 'reports');
  const out: FailureRecord['artifacts'] = {};

  // Screenshots/traces use FAIL-{scenario}-{human}.{ext} — we list directory for any match
  const screenshotDir = join(reports, 'screenshots');
  const traceDir = join(reports, 'traces');
  const consoleDir = join(reports, 'console');
  const observationDir = join(reports, 'observations');

  // Observations: {scenario}.json (no human suffix)
  const obs = findArtifact(observationDir, safe, 'json');
  if (obs) out.observation = obs;

  // Screenshots/traces/console may have human suffix — find first match by prefix
  for (const [key, dir, prefix, ext] of [
    ['screenshot', screenshotDir, `FAIL-${safe}`, 'png'],
    ['trace', traceDir, `FAIL-${safe}`, 'zip'],
    ['consoleErrors', consoleDir, `${safe}`, 'json'],
  ] as const) {
    if (!existsSync(dir)) continue;
    try {
      const fs = require('node:fs') as typeof import('node:fs');
      const matches = fs
        .readdirSync(dir)
        .filter(f => f.startsWith(prefix) && f.endsWith(`.${ext}`));
      if (matches.length && matches[0]) out[key] = join(dir, matches[0]);
    } catch {
      // best-effort
    }
  }

  return out;
}

function extractFailure(feature: CucumberFeature, element: CucumberElement): FailureRecord | null {
  const failed = element.steps.find(s => s.result.status === 'failed');
  if (!failed) return null;

  const tags = (element.tags ?? []).map(t => t.name);
  const tagFilter = tags.length ? tags.filter(t => t.startsWith('@')).join(' or ') : '';
  const reproCommand = tagFilter
    ? `cd genesis/a2o && E2E_DEVICE_MODE=playwright E2E_HEADLESS=true E2E_TRACE=true npm run test -- --tags "${tagFilter}"`
    : `cd genesis/a2o && E2E_DEVICE_MODE=playwright E2E_HEADLESS=true E2E_TRACE=true npm run test -- ${feature.uri}:${element.line}`;

  return {
    timestamp: new Date().toISOString(),
    feature: { uri: feature.uri, name: feature.name },
    scenario: { name: element.name, line: element.line, tags, id: element.id },
    failedStep: {
      keyword: failed.keyword.trim(),
      name: failed.name ?? '(hidden hook)',
      location: failed.match?.location,
      line: failed.line,
    },
    errorMessage: (failed.result.error_message ?? '').split('\n').slice(0, 20).join('\n'),
    artifacts: collectArtifacts(element.name),
    reproCommand,
  };
}

function main(): void {
  const { report, out } = parseArgs(process.argv.slice(2));

  if (!existsSync(report)) {
    console.error(`No cucumber report at ${report}`);
    process.exit(1);
  }

  const features: CucumberFeature[] = JSON.parse(readFileSync(report, 'utf-8'));
  const failures: FailureRecord[] = [];

  for (const feature of features) {
    for (const element of feature.elements ?? []) {
      const rec = extractFailure(feature, element);
      if (rec) failures.push(rec);
    }
  }

  if (!failures.length) {
    console.log('No failures in report.');
    return;
  }

  mkdirSync(dirname(out), { recursive: true });
  for (const f of failures) appendFileSync(out, JSON.stringify(f) + '\n');

  console.log(`Wrote ${failures.length} failure record(s) to ${out}`);
  for (const f of failures) {
    console.log(`  - ${f.scenario.name}`);
    console.log(`      ${f.feature.uri}:${f.scenario.line}`);
    const artKeys = Object.keys(f.artifacts);
    if (artKeys.length) console.log(`      artifacts: ${artKeys.join(', ')}`);
  }
}

main();
