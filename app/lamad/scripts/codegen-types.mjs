#!/usr/bin/env node
/**
 * Generates TypeScript domain types from lamad manifest + companion schemas.
 *
 * Produces:
 *   - metadata-types.ts — PathMetadata, ConceptMetadata, AssessmentMetadata interfaces
 *   - body-types.ts — EprCompositeBody, Section, Item interfaces
 *   - content-node-types.ts — discriminated TypedContentNode union + type guards
 *
 * Usage:
 *   node codegen-types.mjs           # Generate all
 *   node codegen-types.mjs --verify  # Check if generated files are stale
 */
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { resolve, dirname, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const LAMAD_DIR = resolve(__dirname, '..');
const REPO_ROOT = resolve(__dirname, '../../../');

const VERIFY = process.argv.includes('--verify');

const MANIFEST_PATH = resolve(LAMAD_DIR, 'manifest.json');

// Output locations (identical copies)
const OUTPUT_DIRS = [
  resolve(REPO_ROOT, 'app/elohim-app/src/app/lamad/generated'),
  resolve(REPO_ROOT, 'genesis/seeder/src/generated'),
];

const HEADER = `// AUTO-GENERATED from lamad manifest + companion schemas.
// DO NOT EDIT — regenerate with: pnpm run lamad:codegen
`;

// ---------------------------------------------------------------------------
// Schema → TypeScript conversion (lightweight, no json-schema-to-typescript)
// ---------------------------------------------------------------------------

function capitalize(s) {
  return s.replace(/(^|-)(\w)/g, (_, _sep, c) => c.toUpperCase());
}

function schemaToInterface(schema, name, indent = '') {
  const lines = [];
  const props = schema.properties || {};
  const required = new Set(schema.required || []);

  lines.push(`${indent}export interface ${name} {`);
  for (const [key, prop] of Object.entries(props)) {
    const opt = required.has(key) ? '' : '?';
    const desc = prop.description ? `  /** ${prop.description} */\n${indent}` : '';
    lines.push(`${desc}  ${indent}${key}${opt}: ${schemaTypeToTs(prop)};`);
  }
  if (schema.additionalProperties) {
    lines.push(`  ${indent}[key: string]: unknown;`);
  }
  lines.push(`${indent}}`);
  return lines.join('\n');
}

function schemaTypeToTs(prop) {
  if (prop.$ref && prop.$ref.startsWith('#/$defs/')) {
    return prop.$ref.replace('#/$defs/', '');
  }
  if (prop.enum) {
    return prop.enum.map((v) => `'${v}'`).join(' | ');
  }
  if (prop.type === 'string') return 'string';
  if (prop.type === 'number' || prop.type === 'integer') return 'number';
  if (prop.type === 'boolean') return 'boolean';
  if (prop.type === 'array') {
    if (prop.items) return `${schemaTypeToTs(prop.items)}[]`;
    return 'unknown[]';
  }
  if (prop.type === 'object') {
    if (prop.properties) {
      // Inline object with known properties → emit inline type
      const entries = Object.entries(prop.properties).map(([k, v]) => {
        const req = (prop.required || []).includes(k);
        return `${k}${req ? '' : '?'}: ${schemaTypeToTs(v)}`;
      });
      return `{ ${entries.join('; ')} }`;
    }
    return 'Record<string, unknown>';
  }
  return 'unknown';
}

function generateInterfacesFromSchema(schema) {
  const blocks = [];
  const title = schema.title;

  // Generate $defs first (Section, Item, etc.)
  if (schema.$defs) {
    for (const [defName, defSchema] of Object.entries(schema.$defs)) {
      blocks.push(schemaToInterface(defSchema, defName));
    }
  }

  // Generate main interface
  blocks.push(schemaToInterface(schema, title));
  return blocks.join('\n\n');
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function loadSchema(refPath) {
  const fullPath = resolve(LAMAD_DIR, refPath);
  return JSON.parse(await readFile(fullPath, 'utf8'));
}

async function main() {
  const manifest = JSON.parse(await readFile(MANIFEST_PATH, 'utf8'));
  const contentTypes = manifest.vocabulary?.contentTypes || {};
  const contentFormats = manifest.vocabulary?.contentFormats || {};

  // --- Collect metadata schemas ---
  const metadataSchemas = new Map(); // contentType -> { schema, title }
  for (const [typeName, typeDef] of Object.entries(contentTypes)) {
    if (typeDef.metadataSchema?.$ref) {
      const schema = await loadSchema(typeDef.metadataSchema.$ref);
      metadataSchemas.set(typeName, { schema, title: schema.title });
    }
  }

  // --- Collect body schemas ---
  const bodySchemas = new Map(); // formatName -> { schema, title }
  for (const [formatName, formatDef] of Object.entries(contentFormats)) {
    if (formatDef.bodySchema?.$ref) {
      const schema = await loadSchema(formatDef.bodySchema.$ref);
      bodySchemas.set(formatName, { schema, title: schema.title });
    }
  }

  // --- Generate metadata-types.ts ---
  const metadataBlocks = [];
  for (const [, { schema }] of metadataSchemas) {
    metadataBlocks.push(generateInterfacesFromSchema(schema));
  }
  const metadataContent =
    HEADER +
    '\n' +
    metadataBlocks.join('\n\n') +
    '\n';

  // --- Generate body-types.ts ---
  const bodyBlocks = [];
  for (const [, { schema }] of bodySchemas) {
    bodyBlocks.push(generateInterfacesFromSchema(schema));
  }
  const bodyContent =
    HEADER +
    '\n' +
    bodyBlocks.join('\n\n') +
    '\n';

  // --- Generate content-node-types.ts ---
  const unionMembers = [];
  for (const [typeName, { title }] of metadataSchemas) {
    unionMembers.push(
      `  | (ContentView & { contentType: '${typeName}'; metadata: ${title} })`,
    );
  }
  // Fallback for untyped content types
  unionMembers.push(
    `  | (ContentView & { contentType: string; metadata: Record<string, unknown> })`,
  );

  const typeGuards = [];
  for (const [typeName, { title }] of metadataSchemas) {
    const guardName = `is${capitalize(typeName)}Node`;
    typeGuards.push(
      `export function ${guardName}(node: ContentView): node is ContentView & { contentType: '${typeName}'; metadata: ${title} } {
  return node.contentType === '${typeName}';
}`,
    );
  }

  const contentNodeContent =
    HEADER +
    `
import type { ContentView } from '../../generated/content-view';
import type { ${[...metadataSchemas.values()].map((v) => v.title).join(', ')} } from './metadata-types';

export type TypedContentNode =
${unionMembers.join('\n')};

${typeGuards.join('\n\n')}
`;

  // Also generate a seeder-compatible version without the @app/ import
  const seederContentNodeContent =
    HEADER +
    `
import type { ContentView } from './content-view';
import type { ${[...metadataSchemas.values()].map((v) => v.title).join(', ')} } from './metadata-types';

export type TypedContentNode =
${unionMembers.join('\n')};

${typeGuards.join('\n\n')}
`;

  // --- Output ---
  const files = new Map([
    ['metadata-types.ts', metadataContent],
    ['body-types.ts', bodyContent],
    ['content-node-types.ts', null], // handled per-directory
  ]);

  if (VERIFY) {
    let hasFailure = false;
    for (const dir of OUTPUT_DIRS) {
      const isSeeder = dir.includes('seeder');
      for (const [filename, content] of files) {
        const fileContent =
          filename === 'content-node-types.ts'
            ? isSeeder
              ? seederContentNodeContent
              : contentNodeContent
            : content;
        const outPath = resolve(dir, filename);
        let existing;
        try {
          existing = await readFile(outPath, 'utf8');
        } catch {
          console.error(`FAIL: Generated file does not exist: ${outPath}`);
          hasFailure = true;
          continue;
        }
        if (existing !== fileContent) {
          console.error(
            `FAIL: ${relative(REPO_ROOT, outPath)} is stale. Run: pnpm run lamad:codegen`,
          );
          hasFailure = true;
        }
      }
    }
    if (hasFailure) process.exit(1);
    console.log('Lamad domain type codegen is up to date.');
    return;
  }

  for (const dir of OUTPUT_DIRS) {
    await mkdir(dir, { recursive: true });
    const isSeeder = dir.includes('seeder');
    for (const [filename, content] of files) {
      const fileContent =
        filename === 'content-node-types.ts'
          ? isSeeder
            ? seederContentNodeContent
            : contentNodeContent
          : content;
      const outPath = resolve(dir, filename);
      await writeFile(outPath, fileContent);
      console.log(`Generated: ${relative(REPO_ROOT, outPath)}`);
    }
  }

  console.log(
    `\nLamad domain type codegen complete: ${files.size} files × ${OUTPUT_DIRS.length} locations`,
  );
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
