#!/usr/bin/env node
/**
 * Qahal codegen — generates TypeScript types from the qahal domain manifest.
 *
 * Reads:
 *   - Qahal manifest (elohim/sdk/domains/qahal/manifest.json)
 *   - Qahal companion schemas (elohim/sdk/domains/qahal/schemas/*.schema.json)
 *
 * Produces (to app/elohim-app/src/app/qahal/generated/):
 *   - metadata-types.ts — CollectiveMetadata, ProposalMetadata, ChallengeMetadata, StatementMetadata
 *   - content-node-types.ts — type guards: isCollectiveNode(), isProposalNode(), etc.
 *   - manifest-types.ts — content type lists, signal map
 *   - coupling-map.ts — QAHAL_COUPLING_MAP
 *
 * Usage:
 *   node codegen.mjs           # Generate all
 *   node codegen.mjs --verify  # Check if generated files are stale
 */
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { resolve, dirname, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const DOMAIN_DIR = resolve(__dirname, '..');
const REPO_ROOT = resolve(__dirname, '../../../../../');

const VERIFY = process.argv.includes('--verify');

const MANIFEST_PATH = resolve(DOMAIN_DIR, 'manifest.json');

const OUTPUT_DIRS = [
  resolve(REPO_ROOT, 'app/elohim-app/src/app/qahal/generated'),
];

const DOMAIN_HEADER = `// AUTO-GENERATED from qahal manifest + companion schemas.
// DO NOT EDIT — regenerate with: pnpm run qahal:codegen
`;

// ---------------------------------------------------------------------------
// Schema → TypeScript conversion
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

  if (schema.$defs) {
    for (const [defName, defSchema] of Object.entries(schema.$defs)) {
      blocks.push(schemaToInterface(defSchema, defName));
    }
  }

  blocks.push(schemaToInterface(schema, title));
  return blocks.join('\n\n');
}

// ---------------------------------------------------------------------------
// Manifest constants generation
// ---------------------------------------------------------------------------

function formatTsConst(name, values) {
  const singleLine = `export const ${name} = [${values.map((v) => `'${v}'`).join(', ')}] as const;`;
  if (singleLine.length <= 100) return singleLine;
  const items = values.map((v) => `  '${v}',`).join('\n');
  return `export const ${name} = [\n${items}\n] as const;`;
}

function generateManifestTypes(manifest) {
  const appName = manifest.name;
  const prefix = appName.toUpperCase();
  const titlePrefix = capitalize(appName);
  const relManifestPath = relative(REPO_ROOT, MANIFEST_PATH);

  const contentTypes = Object.keys(manifest.vocabulary?.contentTypes || {});
  const relationships = Object.keys(manifest.vocabulary?.relationships || {});
  const signals = Object.keys(manifest.vocabulary?.signals || {});

  const blocks = [];

  blocks.push(formatTsConst(`${prefix}_CONTENT_TYPES`, contentTypes));
  blocks.push(`export type ${titlePrefix}ContentType = (typeof ${prefix}_CONTENT_TYPES)[number];`);
  blocks.push('');

  blocks.push(formatTsConst(`${prefix}_RELATIONSHIPS`, relationships));
  blocks.push(
    `export type ${titlePrefix}Relationship = (typeof ${prefix}_RELATIONSHIPS)[number];`,
  );
  blocks.push('');

  blocks.push(formatTsConst(`${prefix}_SIGNALS`, signals));
  blocks.push(`export type ${titlePrefix}Signal = (typeof ${prefix}_SIGNALS)[number];`);

  while (blocks.length > 0 && blocks[blocks.length - 1] === '') blocks.pop();

  const header = `// AUTO-GENERATED from app manifest: ${relManifestPath}
// DO NOT EDIT — regenerate with: pnpm run qahal:codegen
`;

  return header + '\n' + blocks.join('\n') + '\n';
}

// ---------------------------------------------------------------------------
// Coupling map generation
// ---------------------------------------------------------------------------

function generateCouplingMap(manifest) {
  const contentTypes = manifest.vocabulary?.contentTypes || {};
  const relManifestPath = relative(REPO_ROOT, MANIFEST_PATH);

  const header = `// AUTO-GENERATED from app manifest: ${relManifestPath}
// DO NOT EDIT — regenerate with: pnpm run qahal:codegen
`;

  const lines = [];

  lines.push(`export interface ValueFlow {
  action: string;
  resourceConformsTo: string;
  recognition: string;
}

export interface ContentTypeCoupling {
  value: {
    onConsume?: ValueFlow;
    onComplete?: ValueFlow;
    onContribute?: ValueFlow;
  };
  governance: {
    defaultReach: string;
    minimumReach: string;
    governanceModel: string;
    signalTypes: string[];
  };
}

export const QAHAL_COUPLING_MAP: Record<string, ContentTypeCoupling> = {`);

  for (const [typeName, typeDef] of Object.entries(contentTypes)) {
    const coupling = typeDef.coupling;
    if (!coupling) continue;

    const value = coupling.value || {};
    const gov = coupling.governance || {};

    const valueEntries = [];
    for (const lifecycle of ['onConsume', 'onComplete', 'onContribute']) {
      if (value[lifecycle]) {
        const v = value[lifecycle];
        valueEntries.push(
          `      ${lifecycle}: { action: '${v.action}', resourceConformsTo: '${v.resourceConformsTo}', recognition: '${v.recognition}' },`,
        );
      }
    }

    const signalTypes = (gov.signalTypes || []).map((s) => `'${s}'`).join(', ');

    lines.push(`  '${typeName}': {
    value: {
${valueEntries.join('\n')}
    },
    governance: {
      defaultReach: '${gov.defaultReach || 'commons'}',
      minimumReach: '${gov.minimumReach || 'community'}',
      governanceModel: '${gov.governanceModel || 'steward-consent'}',
      signalTypes: [${signalTypes}],
    },
  },`);
  }

  lines.push(`};`);

  return header + '\n' + lines.join('\n') + '\n';
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function loadSchema(refPath) {
  const fullPath = resolve(DOMAIN_DIR, refPath);
  return JSON.parse(await readFile(fullPath, 'utf8'));
}

async function main() {
  const manifest = JSON.parse(await readFile(MANIFEST_PATH, 'utf8'));
  const contentTypes = manifest.vocabulary?.contentTypes || {};

  // --- Collect metadata schemas ---
  const metadataSchemas = new Map();
  for (const [typeName, typeDef] of Object.entries(contentTypes)) {
    if (typeDef.metadataSchema?.$ref) {
      const schema = await loadSchema(typeDef.metadataSchema.$ref);
      metadataSchemas.set(typeName, { schema, title: schema.title });
    }
  }

  // --- Generate metadata-types.ts ---
  const metadataBlocks = [];
  for (const [, { schema }] of metadataSchemas) {
    metadataBlocks.push(generateInterfacesFromSchema(schema));
  }
  const metadataContent =
    DOMAIN_HEADER +
    '\n' +
    metadataBlocks.join('\n\n') +
    '\n';

  // --- Generate content-node-types.ts ---
  const metadataImports = [...metadataSchemas.values()].map((v) => v.title).join(', ');

  const unionMembers = [];
  for (const [typeName, { title }] of metadataSchemas) {
    unionMembers.push(
      `  | (ContentView & { contentType: '${typeName}'; metadata: ${title} })`,
    );
  }
  unionMembers.push(
    `  | (ContentView & { contentType: string; metadata: Record<string, unknown> })`,
  );

  const typeGuards = [];
  for (const [typeName, { title }] of metadataSchemas) {
    const guardName = `is${capitalize(typeName)}Node`;
    typeGuards.push(
      `export function ${guardName}<T extends { contentType: string }>(node: T): node is T & { contentType: '${typeName}'; metadata: ${title} } {
  return node.contentType === '${typeName}';
}`,
    );
  }

  const contentNodeContent =
    DOMAIN_HEADER +
    `
import type { ContentView } from '../../generated/content-view';
import type { ${metadataImports} } from './metadata-types';

export type QahalTypedContentNode =
${unionMembers.join('\n')};

${typeGuards.join('\n\n')}
`;

  // --- Generate manifest-types.ts ---
  const manifestContent = generateManifestTypes(manifest);

  // --- Generate coupling-map.ts ---
  const couplingContent = generateCouplingMap(manifest);

  // --- Output ---
  const files = new Map([
    ['metadata-types.ts', metadataContent],
    ['content-node-types.ts', contentNodeContent],
    ['manifest-types.ts', manifestContent],
    ['coupling-map.ts', couplingContent],
  ]);

  if (VERIFY) {
    let hasFailure = false;
    for (const dir of OUTPUT_DIRS) {
      for (const [filename, fileContent] of files) {
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
            `FAIL: ${relative(REPO_ROOT, outPath)} is stale. Run: pnpm run qahal:codegen`,
          );
          hasFailure = true;
        }
      }
    }
    if (hasFailure) process.exit(1);
    console.log('Qahal codegen is up to date.');
    return;
  }

  for (const dir of OUTPUT_DIRS) {
    await mkdir(dir, { recursive: true });
    for (const [filename, fileContent] of files) {
      const outPath = resolve(dir, filename);
      await writeFile(outPath, fileContent);
      console.log(`Generated: ${relative(REPO_ROOT, outPath)}`);
    }
  }

  console.log(
    `\nQahal codegen complete: ${files.size} files × ${OUTPUT_DIRS.length} locations`,
  );
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
