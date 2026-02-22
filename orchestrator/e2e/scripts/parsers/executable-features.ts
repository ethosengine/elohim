/**
 * Parse executable feature files from the unified E2E location.
 *
 * All executable features now live in orchestrator/e2e/features/.
 * (Cypress has been migrated and removed.)
 *
 * Extracts @tag lines and Scenario: names.
 */

import { readdir, readFile, access } from 'node:fs/promises';
import { join, relative } from 'node:path';

import type { ExecutableScenario } from '../types.js';

interface FeatureSource {
  dir: string;
  framework: 'cucumber-js';
}

const SOURCES: FeatureSource[] = [{ dir: 'orchestrator/e2e/features', framework: 'cucumber-js' }];

async function walkFeatureFiles(dir: string): Promise<string[]> {
  try {
    await access(dir);
  } catch {
    return [];
  }
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

function parseTags(content: string): string[] {
  const tags: string[] = [];
  for (const line of content.split('\n')) {
    const trimmed = line.trim();
    // Tag lines start with @ and appear before Feature:
    if (trimmed.startsWith('@')) {
      // Tags can be space-separated (@e2e @federation) or colon-formatted (@epic:value)
      const found = trimmed.match(/@[\w:_-]+/g); // eslint-disable-line sonarjs/duplicates-in-character-class
      if (found) tags.push(...found);
    }
    // Stop scanning after Feature line
    if (trimmed.startsWith('Feature:')) break;
  }
  return tags;
}

function parseScenarioNames(content: string): string[] {
  const names: string[] = [];
  for (const line of content.split('\n')) {
    const match = /^\s*Scenario(?:\s+Outline)?:\s*(.+)$/.exec(line); // eslint-disable-line sonarjs/slow-regex
    if (match) names.push(match[1].trim());
  }
  return names;
}

export async function parseExecutableFeatures(repoRoot: string): Promise<ExecutableScenario[]> {
  const scenarios: ExecutableScenario[] = [];

  for (const source of SOURCES) {
    const dir = join(repoRoot, source.dir);
    const featureFiles = await walkFeatureFiles(dir);

    for (const filePath of featureFiles) {
      const content = await readFile(filePath, 'utf-8');
      const tags = parseTags(content);
      const names = parseScenarioNames(content);
      const relPath = relative(repoRoot, filePath);

      for (const name of names) {
        scenarios.push({ name, featureFile: relPath, tags, framework: source.framework });
      }
    }
  }

  return scenarios;
}
