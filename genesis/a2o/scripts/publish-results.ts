#!/usr/bin/env tsx
/**
 * Publish E2E Results as ContentNodes
 *
 * Reads Cucumber JSON reports or exploration session files and creates
 * ContentNodes with contentType "test-result" or "exploration-feedback",
 * then posts them to a running doorway — dogfooding the content graph.
 *
 * Usage:
 *   npx tsx scripts/publish-results.ts                                      # publish latest cucumber report
 *   npx tsx scripts/publish-results.ts --exploration reports/exploration-sessions/explore-*.json
 *   npx tsx scripts/publish-results.ts --target https://doorway-alpha.elohim.host
 */

import { readFile, readdir } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { request } from 'undici';

import type { ExplorationSession } from './explore-session.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPORTS_DIR = join(__dirname, '..', 'reports');

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface ContentNode {
  contentType: string;
  title: string;
  description: string;
  content: string;
  contentFormat: string;
  tags: string[];
}

interface CucumberScenario {
  name: string;
  keyword: string;
  tags: { name: string }[];
  steps: {
    keyword: string;
    name: string;
    result: { status: string; duration?: number; error_message?: string };
  }[];
}

interface CucumberFeature {
  name: string;
  uri: string;
  tags: { name: string }[];
  elements: CucumberScenario[];
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

interface CliArgs {
  targetUrl: string;
  token?: string;
  exploration?: string;
  dryRun: boolean;
}

function parseArgs(): CliArgs {
  const args = process.argv.slice(2);
  const result: CliArgs = {
    targetUrl: process.env['E2E_DOORWAY_URL'] ?? 'https://doorway-alpha.elohim.host',
    dryRun: false,
  };

  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--target' && args[i + 1]) result.targetUrl = args[++i];
    else if (args[i] === '--token' && args[i + 1]) result.token = args[++i];
    else if (args[i] === '--exploration' && args[i + 1]) result.exploration = args[++i];
    else if (args[i] === '--dry-run') result.dryRun = true;
  }

  return result;
}

// ---------------------------------------------------------------------------
// Cucumber report -> ContentNodes
// ---------------------------------------------------------------------------

async function cucumberToContentNodes(): Promise<ContentNode[]> {
  const reportPath = join(REPORTS_DIR, 'cucumber-report.json');
  let raw: string;
  try {
    raw = await readFile(reportPath, 'utf-8');
  } catch {
    console.log('No cucumber report found at', reportPath);
    return [];
  }

  const features: CucumberFeature[] = JSON.parse(raw);
  const nodes: ContentNode[] = [];

  for (const feature of features) {
    for (const scenario of feature.elements) {
      const passed = scenario.steps.every(s => s.result.status === 'passed');
      const pending = scenario.steps.some(
        s => s.result.status === 'pending' || s.result.status === 'undefined'
      );
      let status: 'pass' | 'pending' | 'fail';
      if (passed) status = 'pass';
      else if (pending) status = 'pending';
      else status = 'fail';

      const tags = [
        'e2e-result',
        `status:${status}`,
        ...scenario.tags.map(t => t.name.replace('@', '')),
        ...feature.tags.map(t => t.name.replace('@', '')),
      ];

      const stepSummary = scenario.steps
        .map(s => {
          let badge: string;
          if (s.result.status === 'passed') badge = '[PASS]';
          else if (s.result.status === 'pending') badge = '[PENDING]';
          else badge = '[FAIL]';
          return `${badge} ${s.keyword}${s.name}`;
        })
        .join('\n');

      nodes.push({
        contentType: 'test-result',
        title: `E2E: ${scenario.name}`,
        description: `Automated test result for "${scenario.name}" from ${feature.name}`,
        content: `# ${scenario.name}\n\nFeature: ${feature.name}\nStatus: ${status}\n\n## Steps\n\n${stepSummary}`,
        contentFormat: 'markdown',
        tags,
      });
    }
  }

  return nodes;
}

// ---------------------------------------------------------------------------
// Exploration sessions -> ContentNodes
// ---------------------------------------------------------------------------

async function explorationToContentNodes(pattern: string): Promise<ContentNode[]> {
  const sessionsDir = join(REPORTS_DIR, 'exploration-sessions');
  const nodes: ContentNode[] = [];

  let files: string[];
  try {
    const allFiles = await readdir(sessionsDir);
    files = allFiles.filter(f => f.endsWith('.json') && f.includes(pattern.replace('*', '')));
    if (files.length === 0) files = allFiles.filter(f => f.endsWith('.json'));
  } catch {
    console.log('No exploration sessions found');
    return [];
  }

  for (const file of files) {
    const raw = await readFile(join(sessionsDir, file), 'utf-8');
    const sessions: ExplorationSession[] = JSON.parse(raw);

    for (const session of sessions) {
      const stepSummary = session.steps
        .map(s => {
          const suffix = s.observation ? ` — ${s.observation}` : '';
          return `[${s.status.toUpperCase()}] ${s.step}${suffix}`;
        })
        .join('\n');

      const tags = [
        'e2e-exploration',
        `verdict:${session.verdict}`,
        ...session.tags.map(t => t.replace('@', '')),
      ];

      nodes.push({
        contentType: 'exploration-feedback',
        title: `Exploration: ${session.scenario}`,
        description: `Claude-driven exploratory test of "${session.scenario}"`,
        content: [
          `# ${session.scenario}`,
          ``,
          `Feature: ${session.featureFile}`,
          `Target: ${session.targetUrl}`,
          `Verdict: ${session.verdict}`,
          `Duration: ${session.duration_ms}ms`,
          ``,
          `## Steps`,
          ``,
          stepSummary,
          session.feedback ? `\n## Feedback\n\n${session.feedback}` : '',
        ].join('\n'),
        contentFormat: 'markdown',
        tags,
      });
    }
  }

  return nodes;
}

// ---------------------------------------------------------------------------
// Publish
// ---------------------------------------------------------------------------

async function publishNodes(
  nodes: ContentNode[],
  targetUrl: string,
  token?: string
): Promise<void> {
  console.log(`Publishing ${nodes.length} ContentNode(s) to ${targetUrl}...`);

  const headers: Record<string, string> = { 'content-type': 'application/json' };
  if (token) headers['authorization'] = `Bearer ${token}`;

  // Use bulk endpoint if available, otherwise create individually
  const bulkUrl = `${targetUrl}/db/content/bulk`;
  try {
    const { statusCode, body } = await request(bulkUrl, {
      method: 'POST',
      headers,
      body: JSON.stringify(nodes),
    });
    await body.text();
    if (statusCode >= 200 && statusCode < 300) {
      console.log(`  Bulk created ${nodes.length} node(s)`);
      return;
    }
    console.log(`  Bulk endpoint returned ${statusCode}, falling back to individual creates`);
  } catch {
    console.log('  Bulk endpoint not available, creating individually...');
  }

  // Fallback: create one at a time
  let created = 0;
  let failed = 0;
  for (const node of nodes) {
    try {
      const { statusCode, body } = await request(`${targetUrl}/db/content`, {
        method: 'POST',
        headers,
        body: JSON.stringify(node),
      });
      await body.text();
      if (statusCode >= 200 && statusCode < 300) {
        created++;
      } else {
        failed++;
      }
    } catch {
      failed++;
    }
  }

  console.log(`  Created: ${created}, Failed: ${failed}`);
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
  const { targetUrl, token, exploration, dryRun } = parseArgs();

  let nodes: ContentNode[];

  if (exploration) {
    nodes = await explorationToContentNodes(exploration);
  } else {
    nodes = await cucumberToContentNodes();
  }

  if (nodes.length === 0) {
    console.log('No results to publish');
    return;
  }

  console.log(`Prepared ${nodes.length} ContentNode(s)`);
  for (const n of nodes.slice(0, 5)) {
    console.log(`  - [${n.contentType}] ${n.title} (${n.tags.join(', ')})`);
  }
  if (nodes.length > 5) console.log(`  ... and ${nodes.length - 5} more`);

  if (dryRun) {
    console.log('\n[DRY RUN] Skipping publish');
    return;
  }

  await publishNodes(nodes, targetUrl, token);
}

main().catch(err => {
  console.error('Publish failed:', err);
  process.exit(1);
});
