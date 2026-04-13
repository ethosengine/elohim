/**
 * Generate devices.json from genesis/data/devices/*.md frontmatter.
 *
 * Usage:
 *   npx tsx src/generate-devices-json.ts
 *
 * Output:
 *   genesis/data/devices/devices.json
 */

import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { parse as parseYaml } from 'yaml';

const DEVICES_DIR = join(import.meta.dirname, '../../data/devices');
const OUTPUT = join(DEVICES_DIR, 'devices.json');

function extractFrontmatter(content: string): Record<string, unknown> | null {
  const match = content.match(/^---\n([\s\S]*?)\n---/);
  if (!match) return null;
  return parseYaml(match[1]) as Record<string, unknown>;
}

const files = readdirSync(DEVICES_DIR)
  .filter(f => f.endsWith('.md') && !f.startsWith('_'))
  .sort();

const devices = files.map(file => {
  const content = readFileSync(join(DEVICES_DIR, file), 'utf-8');
  const fm = extractFrontmatter(content);
  if (!fm) throw new Error(`${file}: no frontmatter`);
  return fm;
});

const output = { devices };
writeFileSync(OUTPUT, JSON.stringify(output, null, 2) + '\n');
console.log(`Generated ${OUTPUT} with ${devices.length} devices`);
