#!/usr/bin/env node
/**
 * Shefa codegen — generates TypeScript types from shefa domain manifest + schemas.
 *
 * Reads:
 *   - Shefa manifest (elohim/sdk/domains/shefa/manifest.json)
 *   - Shefa companion schemas (elohim/sdk/domains/shefa/schemas/*.schema.json)
 *
 * Produces (to app/elohim-app/src/app/shefa/generated/):
 *   - metadata-types.ts — StewardshipMetadata, ExchangeMetadata, AgreementMetadata interfaces
 *   - coupling-map.ts — SHEFA_COUPLING_MAP (value flows + governance signals per content type)
 *   - manifest-types.ts — content type lists, signal map, relationship types
 *
 * Usage:
 *   node codegen.mjs           # Generate all
 *   node codegen.mjs --verify  # Check if generated files are stale
 */
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { resolve, dirname, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SHEFA_DIR = resolve(__dirname, '..');
const REPO_ROOT = resolve(__dirname, '../../../../../');

const VERIFY = process.argv.includes('--verify');

const MANIFEST_PATH = resolve(SHEFA_DIR, 'manifest.json');

// Output location — shefa only outputs to the Angular app (no seeder output)
const OUTPUT_DIRS = [
  resolve(REPO_ROOT, 'app/elohim-app/src/app/shefa/generated'),
];

const DOMAIN_HEADER = `// AUTO-GENERATED from shefa manifest + companion schemas.
// DO NOT EDIT — regenerate with: pnpm run shefa:codegen
`;

// ---------------------------------------------------------------------------
// Schema → TypeScript conversion (same lightweight approach as lamad)
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

  // Generate $defs first
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
// Manifest constants generation
// ---------------------------------------------------------------------------

function formatTsConst(name, values) {
  const singleLine = `export const ${name} = [${values.map((v) => `'${v}'`).join(', ')}] as const;`;
  if (singleLine.length <= 100) return singleLine;
  const items = values.map((v) => `  '${v}',`).join('\n');
  return `export const ${name} = [\n${items}\n] as const;`;
}

function generateManifestTypes(manifest) {
  const appName = manifest.name; // "shefa"
  const prefix = appName.toUpperCase(); // SHEFA
  const titlePrefix = capitalize(appName); // Shefa
  const relManifestPath = relative(REPO_ROOT, MANIFEST_PATH);

  const contentTypes = Object.keys(manifest.vocabulary?.contentTypes || {});
  const relationships = Object.keys(manifest.vocabulary?.relationships || {});
  const signals = Object.keys(manifest.vocabulary?.signals || {});

  const blocks = [];

  // Content types
  blocks.push(formatTsConst(`${prefix}_CONTENT_TYPES`, contentTypes));
  blocks.push(`export type ${titlePrefix}ContentType = (typeof ${prefix}_CONTENT_TYPES)[number];`);
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

  // Remove trailing empty blocks
  while (blocks.length > 0 && blocks[blocks.length - 1] === '') blocks.pop();

  const header = `// AUTO-GENERATED from app manifest: ${relManifestPath}
// DO NOT EDIT — regenerate with: pnpm run shefa:codegen
`;

  return header + '\n' + blocks.join('\n') + '\n';
}

// ---------------------------------------------------------------------------
// Coupling map generation (for signal harness)
// ---------------------------------------------------------------------------

function generateCouplingMap(manifest) {
  const contentTypes = manifest.vocabulary?.contentTypes || {};
  const relManifestPath = relative(REPO_ROOT, MANIFEST_PATH);

  const header = `// AUTO-GENERATED from app manifest: ${relManifestPath}
// DO NOT EDIT — regenerate with: pnpm run shefa:codegen
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

export const SHEFA_COUPLING_MAP: Record<string, ContentTypeCoupling> = {`);

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
  const fullPath = resolve(SHEFA_DIR, refPath);
  return JSON.parse(await readFile(fullPath, 'utf8'));
}

async function main() {
  const manifest = JSON.parse(await readFile(MANIFEST_PATH, 'utf8'));
  const contentTypes = manifest.vocabulary?.contentTypes || {};

  // Also collect metadata schemas from protocolPrimitives that have them
  const protocolPrimitives = manifest.vocabulary?.protocolPrimitives || {};

  // --- Collect metadata schemas ---
  const metadataSchemas = new Map(); // key -> { schema, title }
  for (const [typeName, typeDef] of Object.entries(contentTypes)) {
    if (typeDef.metadataSchema?.$ref) {
      const schema = await loadSchema(typeDef.metadataSchema.$ref);
      metadataSchemas.set(typeName, { schema, title: schema.title });
    }
  }
  for (const [typeName, typeDef] of Object.entries(protocolPrimitives)) {
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

  // --- Generate manifest-types.ts ---
  const manifestContent = generateManifestTypes(manifest);

  // --- Generate coupling-map.ts ---
  const couplingContent = generateCouplingMap(manifest);

  // --- Output ---
  const files = new Map([
    ['metadata-types.ts', metadataContent],
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
            `FAIL: ${relative(REPO_ROOT, outPath)} is stale. Run: pnpm run shefa:codegen`,
          );
          hasFailure = true;
        }
      }
    }
    if (hasFailure) process.exit(1);
    console.log('Shefa codegen is up to date.');
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
    `\nShefa codegen complete: ${files.size} files × ${OUTPUT_DIRS.length} locations`,
  );
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
