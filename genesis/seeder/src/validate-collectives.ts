/**
 * Collectives Seed Data Validator
 *
 * Validates genesis/data/collectives/collectives.json against the
 * collectives.schema.json IoC contract. Hand-rolled to match the
 * existing seeder validation pattern (no Ajv/Zod).
 *
 * Two levels of validation:
 * 1. Per-collective field validation (enums, required fields, types)
 * 2. Cross-collective referential integrity (parent refs, relationship refs, cycles)
 */

import { readFileSync } from 'node:fs';
import { REACH_LEVELS } from './validation-constants.js';

// =============================================================================
// Constants — sources of truth
// =============================================================================

/** Source: Rust governance_layers::ALL in elohim-storage/src/db/models.rs */
const GOVERNANCE_LAYERS = [
  'family', 'neighborhood', 'faith', 'education', 'interest',
  'geographic', 'workplace', 'economic', 'community',
] as const;

/** Source: qahal collective-metadata.schema.json */
const GOVERNANCE_MODELS = [
  'consent', 'steward-consent', 'community-vote', 'constitutional', 'consensus',
] as const;

/** Source: collectives.schema.json domain enum */
const DOMAINS = [
  'household', 'curriculum', 'worship', 'infrastructure',
  'trade', 'land-use', 'economy', 'defense',
] as const;

/** Source: collectives.schema.json relationship types */
const RELATIONSHIP_TYPES = [
  'contains', 'participates-in', 'delegates-to', 'peers-with', 'succeeds',
] as const;

const SLUG_PATTERN = /^[a-z0-9][a-z0-9-]*[a-z0-9]$/;

// =============================================================================
// Types
// =============================================================================

interface CollectiveEntry {
  id: string;
  name: string;
  governanceLayer: string;
  reach: string;
  constitutionalParentId?: string | null;
  description?: string | null;
  governanceModel?: string | null;
  domain?: string | null;
  place?: string | null;
  coupling?: {
    lamad?: string;
    shefa?: string;
    qahal?: string;
  } | null;
}

interface RelationshipEntry {
  type: string;
  from: string;
  to: string;
  description?: string | null;
  domainOverlap?: string | null;
}

interface CollectivesData {
  version: string;
  collectives: CollectiveEntry[];
  relationships?: RelationshipEntry[];
}

export interface CollectiveValidationResult {
  errors: string[];
  warnings: string[];
  collectiveCount: number;
  relationshipCount: number;
  collectiveIds: Set<string>;
  data: CollectivesData | null;
}

// =============================================================================
// Validation
// =============================================================================

function validateEnum(
  value: string | undefined | null,
  field: string,
  allowed: readonly string[],
  entityId: string,
  required: boolean,
): string | null {
  if (value == null) {
    return required ? `${entityId}: ${field} is required` : null;
  }
  if (!(allowed as readonly string[]).includes(value)) {
    return `${entityId}: ${field} "${value}" is not valid. Allowed: ${allowed.join(', ')}`;
  }
  return null;
}

export function validateCollectivesFile(filePath: string): CollectiveValidationResult {
  const result: CollectiveValidationResult = {
    errors: [],
    warnings: [],
    collectiveCount: 0,
    relationshipCount: 0,
    collectiveIds: new Set(),
    data: null,
  };

  let data: CollectivesData;
  try {
    data = JSON.parse(readFileSync(filePath, 'utf-8'));
    result.data = data;
  } catch (e) {
    result.errors.push(`Failed to parse JSON: ${e instanceof Error ? e.message : String(e)}`);
    return result;
  }

  if (!data.version) {
    result.errors.push('Missing required field: version');
  }

  if (!Array.isArray(data.collectives)) {
    result.errors.push('Missing or invalid field: collectives (must be array)');
    return result;
  }

  result.collectiveCount = data.collectives.length;
  result.relationshipCount = data.relationships?.length ?? 0;

  // Validate each collective
  for (const coll of data.collectives) {
    const id = coll.id ?? '(missing id)';

    // Required fields
    if (!coll.id) {
      result.errors.push(`${id}: missing required field 'id'`);
      continue;
    }
    if (!SLUG_PATTERN.test(coll.id)) {
      result.errors.push(`${id}: id must match slug pattern [a-z0-9-]`);
    }
    if (result.collectiveIds.has(coll.id)) {
      result.errors.push(`${id}: duplicate id`);
    }
    result.collectiveIds.add(coll.id);

    if (!coll.name) {
      result.errors.push(`${id}: missing required field 'name'`);
    }

    // Enum validations
    const layerErr = validateEnum(coll.governanceLayer, 'governanceLayer', GOVERNANCE_LAYERS, id, true);
    if (layerErr) result.errors.push(layerErr);

    const reachErr = validateEnum(coll.reach, 'reach', REACH_LEVELS as unknown as string[], id, true);
    if (reachErr) result.errors.push(reachErr);

    const modelErr = validateEnum(coll.governanceModel, 'governanceModel', GOVERNANCE_MODELS, id, false);
    if (modelErr) result.errors.push(modelErr);

    const domainErr = validateEnum(coll.domain, 'domain', DOMAINS, id, false);
    if (domainErr) result.errors.push(domainErr);

    // Coupling validation
    if (coll.coupling != null && typeof coll.coupling === 'object') {
      const validKeys = new Set(['lamad', 'shefa', 'qahal']);
      for (const key of Object.keys(coll.coupling)) {
        if (!validKeys.has(key)) {
          result.errors.push(`${id}: coupling has unknown key '${key}'. Allowed: lamad, shefa, qahal`);
        }
      }
    }
  }

  // Validate relationships
  if (data.relationships) {
    for (let i = 0; i < data.relationships.length; i++) {
      const rel = data.relationships[i];
      const label = `relationship[${i}] (${rel.from} -> ${rel.to})`;

      const typeErr = validateEnum(rel.type, 'type', RELATIONSHIP_TYPES, label, true);
      if (typeErr) result.errors.push(typeErr);

      if (!rel.from) result.errors.push(`${label}: missing required field 'from'`);
      if (!rel.to) result.errors.push(`${label}: missing required field 'to'`);

      // peers-with requires domainOverlap
      if (rel.type === 'peers-with' && !rel.domainOverlap) {
        result.errors.push(`${label}: peers-with relationships require 'domainOverlap'`);
      }
    }
  }

  return result;
}

export function validateReferentialIntegrity(
  result: CollectiveValidationResult,
): string[] {
  const errors: string[] = [];
  const ids = result.collectiveIds;
  const data = result.data;
  if (!data) return errors;

  // Check constitutionalParentId references
  for (const coll of data.collectives) {
    if (coll.constitutionalParentId && !ids.has(coll.constitutionalParentId)) {
      errors.push(
        `${coll.id}: constitutionalParentId "${coll.constitutionalParentId}" does not reference a known collective`,
      );
    }
  }

  // Check relationship from/to references
  if (data.relationships) {
    for (let i = 0; i < data.relationships.length; i++) {
      const rel = data.relationships[i];
      if (rel.from && !ids.has(rel.from)) {
        errors.push(`relationship[${i}]: 'from' "${rel.from}" does not reference a known collective`);
      }
      if (rel.to && !ids.has(rel.to)) {
        errors.push(`relationship[${i}]: 'to' "${rel.to}" does not reference a known collective`);
      }
    }
  }

  // Check for circular constitutionalParentId chains
  const parentMap = new Map<string, string>();
  for (const coll of data.collectives) {
    if (coll.constitutionalParentId) {
      parentMap.set(coll.id, coll.constitutionalParentId);
    }
  }

  for (const startId of parentMap.keys()) {
    const visited = new Set<string>();
    let current: string | undefined = startId;
    while (current && parentMap.has(current)) {
      if (visited.has(current)) {
        errors.push(`circular constitutionalParentId chain detected involving: ${[...visited].join(' -> ')} -> ${current}`);
        break;
      }
      visited.add(current);
      current = parentMap.get(current);
    }
  }

  return errors;
}

// =============================================================================
// CLI
// =============================================================================

if (process.argv[1]?.endsWith('validate-collectives.ts') || process.argv[1]?.endsWith('validate-collectives.js')) {
  const filePath = process.argv[2] ?? new URL('../../data/collectives/collectives.json', import.meta.url).pathname;

  console.log('=== Validate Collectives ===\n');
  console.log(`File: ${filePath}\n`);

  const result = validateCollectivesFile(filePath);
  const integrityErrors = validateReferentialIntegrity(result);

  console.log(`Collectives:    ${result.collectiveCount}`);
  console.log(`Relationships:  ${result.relationshipCount}`);
  console.log(`Errors:         ${result.errors.length}`);
  console.log(`Integrity:      ${integrityErrors.length}`);
  console.log(`Warnings:       ${result.warnings.length}\n`);

  if (result.errors.length > 0) {
    console.error('ERRORS:');
    for (const e of result.errors) console.error(`  x ${e}`);
  }

  if (integrityErrors.length > 0) {
    console.error('\nREFERENTIAL INTEGRITY:');
    for (const e of integrityErrors) console.error(`  x ${e}`);
  }

  if (result.warnings.length > 0) {
    console.log('\nWARNINGS:');
    for (const w of result.warnings) console.log(`  ! ${w}`);
  }

  if (result.errors.length === 0 && integrityErrors.length === 0) {
    console.log('All validations passed');
  }

  process.exit(result.errors.length + integrityErrors.length > 0 ? 1 : 0);
}
