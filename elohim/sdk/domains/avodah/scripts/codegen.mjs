#!/usr/bin/env node
/**
 * Avodah codegen — generates TypeScript types from the avodah app manifest.
 *
 * Reads:
 *   - Avodah manifest (elohim/sdk/domains/avodah/manifest.json)
 *   - Avodah companion schemas (elohim/sdk/domains/avodah/schemas/*.schema.json)
 *
 * Produces (to app/elohim-app/src/app/avodah/generated/):
 *   - metadata-types.ts — WorkStoryMeta, WorkProjectMeta
 *   - content-node-types.ts — type guards: isWorkStoryNode(), isWorkProjectNode()
 *   - manifest-types.ts — content type lists, signal map
 *   - coupling-map.ts — AVODAH_COUPLING_MAP
 *
 * Usage:
 *   node codegen.mjs           # Generate all
 *   node codegen.mjs --verify  # Check if generated files are stale
 */
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { resolve, dirname, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const APP_DIR = resolve(__dirname, '..');
const REPO_ROOT = resolve(__dirname, '../../../../../');

const VERIFY = process.argv.includes('--verify');

const MANIFEST_PATH = resolve(APP_DIR, 'manifest.json');

const OUTPUT_DIRS = [
  resolve(REPO_ROOT, 'app/elohim-app/src/app/avodah/generated'),
];

const DOMAIN_HEADER = `// AUTO-GENERATED from avodah manifest + companion schemas.
// DO NOT EDIT — regenerate with: pnpm run avodah:codegen
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

/**
 * Emit the shared ObservationKindDeclaration + SignalKindDeclaration interfaces
 * and the domain-specific observation_kinds const array.
 */
function generateObservationKindTypes(manifest, prefix) {
  const observationKinds = manifest.observation_kinds || [];
  const signalKinds = manifest.signalKinds || {};

  const blocks = [];

  blocks.push(`export interface DiversityThreshold {
  distinct_households?: number;
  distinct_collectives?: number;
  distinct_regions?: number;
  distinct_archetypes?: number;
  min_count?: number;
}

export interface ObservationKindDeclaration {
  kind: string;
  namespace: string;
  schema: Record<string, string>;
  retention_class: 'operational' | 'contextual' | 'archival' | 'attestation-feeding' | 'wisdom';
  reach: 'agent-private' | 'household' | 'community' | 'commons' | 'commons-attested';
  diversity_threshold?: DiversityThreshold | null;
  graduates_to?: string | null;
  graduation_window_seconds?: number | null;
  graduation_policy?: 'self-threshold' | 'diversity-threshold' | 'summarize' | null;
}`);

  blocks.push('');

  blocks.push(`export interface SignalKindDeclaration {
  description: string;
  target_kinds: string[];
  evidence_required?: boolean;
  standing_impact_allowed?: Array<'advisory' | 'consequential' | 'binding'>;
}`);

  blocks.push('');

  if (observationKinds.length === 0) {
    blocks.push(
      `export const ${prefix}_OBSERVATION_KINDS: ObservationKindDeclaration[] = [];`,
    );
  } else {
    const kindEntries = observationKinds
      .map((ok) => {
        const schemaEntries = Object.entries(ok.schema || {})
          .map(([k, v]) => `    ${k}: '${v}',`)
          .join('\n');
        const dt = ok.diversity_threshold
          ? JSON.stringify(ok.diversity_threshold)
          : 'null';
        const gt = ok.graduates_to != null ? `'${ok.graduates_to}'` : 'null';
        const gw = ok.graduation_window_seconds != null ? String(ok.graduation_window_seconds) : 'null';
        const gp = ok.graduation_policy != null ? `'${ok.graduation_policy}'` : 'null';
        return `  {
    kind: '${ok.kind}',
    namespace: '${ok.namespace}',
    schema: {
${schemaEntries}
    },
    retention_class: '${ok.retention_class}',
    reach: '${ok.reach}',
    diversity_threshold: ${dt},
    graduates_to: ${gt},
    graduation_window_seconds: ${gw},
    graduation_policy: ${gp},
  },`;
      })
      .join('\n');
    blocks.push(
      `export const ${prefix}_OBSERVATION_KINDS: ObservationKindDeclaration[] = [\n${kindEntries}\n];`,
    );
  }

  blocks.push('');

  const signalKindEntries = Object.entries(signalKinds);
  if (signalKindEntries.length === 0) {
    blocks.push(
      `export const ${prefix}_SIGNAL_KINDS: Record<string, SignalKindDeclaration> = {};`,
    );
  } else {
    const entries = signalKindEntries
      .map(([name, decl]) => {
        const targetKinds = (decl.target_kinds || []).map((k) => `'${k}'`).join(', ');
        const evidenceRequired =
          decl.evidence_required != null ? String(decl.evidence_required) : 'undefined';
        const impactAllowed =
          decl.standing_impact_allowed
            ? `[${decl.standing_impact_allowed.map((v) => `'${v}'`).join(', ')}]`
            : 'undefined';
        return `  '${name}': {
    description: '${decl.description.replace(/'/g, "\\'")}',
    target_kinds: [${targetKinds}],
    evidence_required: ${evidenceRequired},
    standing_impact_allowed: ${impactAllowed},
  },`;
      })
      .join('\n');
    blocks.push(
      `export const ${prefix}_SIGNAL_KINDS: Record<string, SignalKindDeclaration> = {\n${entries}\n};`,
    );
  }

  return blocks.join('\n');
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
  blocks.push('');

  // ObservationKindDeclaration + SignalKindDeclaration + domain arrays
  blocks.push(generateObservationKindTypes(manifest, prefix));

  while (blocks.length > 0 && blocks[blocks.length - 1] === '') blocks.pop();

  const header = `// AUTO-GENERATED from app manifest: ${relManifestPath}
// DO NOT EDIT — regenerate with: pnpm run avodah:codegen
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
// DO NOT EDIT — regenerate with: pnpm run avodah:codegen
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

export const AVODAH_COUPLING_MAP: Record<string, ContentTypeCoupling> = {`);

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
  const fullPath = resolve(APP_DIR, refPath);
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

export type AvodahTypedContentNode =
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
            `FAIL: ${relative(REPO_ROOT, outPath)} is stale. Run: pnpm run avodah:codegen`,
          );
          hasFailure = true;
        }
      }
    }
    if (hasFailure) process.exit(1);
    console.log('Avodah codegen is up to date.');
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
    `\nAvodah codegen complete: ${files.size} files × ${OUTPUT_DIRS.length} locations`,
  );
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
