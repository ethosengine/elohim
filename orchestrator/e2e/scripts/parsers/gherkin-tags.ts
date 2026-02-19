/**
 * Parse genesis conceptual feature files.
 *
 * Walks genesis/docs/content/elohim-protocol/ and extracts @key:value tag
 * lines plus Scenario: names. No Gherkin library needed — the tags follow a
 * consistent format on the first lines of each file.
 */

import { readdir, readFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import type { ConceptualScenario } from '../types.js';

const GENESIS_ROOT = 'genesis/docs/content/elohim-protocol';

async function walkFeatureFiles(dir: string): Promise<string[]> {
  const files: string[] = [];
  const entries = await readdir(dir, { withFileTypes: true });
  for (const entry of entries) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await walkFeatureFiles(full)));
    } else if (entry.name.endsWith('.feature')) {
      files.push(full);
    }
  }
  return files;
}

function parseTagValue(lines: string[], key: string): string {
  for (const line of lines) {
    const match = line.match(new RegExp(`^@${key}:(.+)$`));
    if (match) return match[1].trim();
  }
  return 'unknown';
}

function parseScenarioNames(content: string): string[] {
  const names: string[] = [];
  for (const line of content.split('\n')) {
    const match = line.match(/^\s*Scenario:\s*(.+)$/);
    if (match) names.push(match[1].trim());
  }
  return names;
}

export async function parseConceptualFeatures(repoRoot: string): Promise<ConceptualScenario[]> {
  const genesisDir = join(repoRoot, GENESIS_ROOT);
  const featureFiles = await walkFeatureFiles(genesisDir);
  const scenarios: ConceptualScenario[] = [];

  for (const filePath of featureFiles) {
    const content = await readFile(filePath, 'utf-8');
    const lines = content.split('\n');

    const epic = parseTagValue(lines, 'epic');
    const userType = parseTagValue(lines, 'user_type');
    const governanceLayer = parseTagValue(lines, 'governance_layer');

    const names = parseScenarioNames(content);
    const relPath = relative(repoRoot, filePath);

    for (const name of names) {
      scenarios.push({ name, featureFile: relPath, epic, userType, governanceLayer });
    }
  }

  return scenarios;
}
