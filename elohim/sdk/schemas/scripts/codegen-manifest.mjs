#!/usr/bin/env node
/**
 * Generates TypeScript types from an app manifest JSON file.
 *
 * Produces:
 *   - Content type const array + union type
 *   - Content format const array + union type
 *   - Relationship const array + union type
 *   - Signal const array + union type
 *   - Renderer map (format -> component name)
 *
 * Usage:
 *   node codegen-manifest.mjs <manifest-path> <output-path>
 *   node codegen-manifest.mjs --verify <manifest-path> <output-path>
 */
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { resolve, dirname, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '../../../../');

const VERIFY = process.argv.includes('--verify');
const args = process.argv.filter((a) => !a.startsWith('--'));
const manifestArg = args[2];
const outputArg = args[3];

if (!manifestArg || !outputArg) {
  console.error('Usage: node codegen-manifest.mjs [--verify] <manifest-path> <output-path>');
  process.exit(1);
}

const manifestPath = resolve(REPO_ROOT, manifestArg);
const outputPath = resolve(REPO_ROOT, outputArg);

function capitalize(s) {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

function formatTsConst(name, values) {
  const singleLine = `export const ${name} = [${values.map((v) => `'${v}'`).join(', ')}] as const;`;
  if (singleLine.length <= 100) return singleLine;
  const items = values.map((v) => `  '${v}',`).join('\n');
  return `export const ${name} = [\n${items}\n] as const;`;
}

async function main() {
  const raw = await readFile(manifestPath, 'utf8');
  const manifest = JSON.parse(raw);

  const appName = manifest.name; // e.g. "lamad"
  const prefix = appName.toUpperCase(); // LAMAD
  const titlePrefix = capitalize(appName); // Lamad
  const relManifestPath = relative(REPO_ROOT, manifestPath);

  const contentTypes = Object.keys(manifest.vocabulary?.contentTypes || {});
  const contentFormats = Object.keys(manifest.vocabulary?.contentFormats || {});
  const relationships = Object.keys(manifest.vocabulary?.relationships || {});
  const signals = Object.keys(manifest.vocabulary?.signals || {});

  // Build renderer map: format -> component name
  const rendererMap = {};
  const rendering = manifest.rendering || {};
  for (const [, rendererDef] of Object.entries(rendering)) {
    const component = rendererDef.component;
    for (const fmt of rendererDef.formats || []) {
      rendererMap[fmt] = component;
    }
  }

  const blocks = [];

  // Content types
  blocks.push(formatTsConst(`${prefix}_CONTENT_TYPES`, contentTypes));
  blocks.push(`export type ${titlePrefix}ContentType = (typeof ${prefix}_CONTENT_TYPES)[number];`);
  blocks.push('');

  // Content formats
  blocks.push(formatTsConst(`${prefix}_CONTENT_FORMATS`, contentFormats));
  blocks.push(
    `export type ${titlePrefix}ContentFormat = (typeof ${prefix}_CONTENT_FORMATS)[number];`,
  );
  blocks.push('');

  // Relationships
  blocks.push(formatTsConst(`${prefix}_RELATIONSHIPS`, relationships));
  blocks.push(
    `export type ${titlePrefix}Relationship = (typeof ${prefix}_RELATIONSHIPS)[number];`,
  );
  blocks.push('');

  // Signals
  blocks.push(formatTsConst(`${prefix}_SIGNALS`, signals));
  blocks.push(`export type ${titlePrefix}Signal = (typeof ${prefix}_SIGNALS)[number];`);
  blocks.push('');

  // Renderer map
  const rendererEntries = Object.entries(rendererMap)
    .map(([fmt, comp]) => `  '${fmt}': '${comp}',`)
    .join('\n');
  blocks.push(
    `export const ${prefix}_RENDERER_MAP: Record<string, string> = {\n${rendererEntries}\n};`,
  );

  const header = `// AUTO-GENERATED from app manifest: ${relManifestPath}
// DO NOT EDIT — regenerate with: pnpm run manifest:codegen
`;

  // Remove trailing empty blocks
  while (blocks.length > 0 && blocks[blocks.length - 1] === '') blocks.pop();
  const content = header + '\n' + blocks.join('\n') + '\n';

  if (VERIFY) {
    let existing;
    try {
      existing = await readFile(outputPath, 'utf8');
    } catch {
      console.error(`FAIL: Generated file does not exist: ${outputPath}`);
      process.exit(1);
    }
    if (existing !== content) {
      console.error(
        `FAIL: ${relative(REPO_ROOT, outputPath)} is stale. Run: pnpm run manifest:codegen`,
      );
      process.exit(1);
    }
    console.log(`VERIFY OK: ${relative(REPO_ROOT, outputPath)}`);
    return;
  }

  await mkdir(dirname(outputPath), { recursive: true });
  await writeFile(outputPath, content);
  console.log(`Generated: ${relative(REPO_ROOT, outputPath)}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
