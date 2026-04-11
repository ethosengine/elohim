/**
 * Humans Seed Data Validator
 *
 * Validates genesis/data/humans/*.md (markdown with YAML frontmatter) and
 * genesis/data/humans/relationships.md against humans.schema.json. Hand-rolled
 * to match the validate-collectives.ts precedent (no Ajv/Zod).
 *
 * Two levels of validation:
 * 1. Per-human frontmatter validation (enums, required fields, regex, warnings)
 * 2. Directory-level referential integrity (duplicate ids / displayNames,
 *    guardian resolution, relationship source/target resolution, circular
 *    guardianship)
 */

import { readFile, readdir } from 'node:fs/promises';
import { readdirSync, readFileSync } from 'node:fs';
import { join, basename } from 'node:path';
import { parse as parseYaml } from 'yaml';

// =============================================================================
// Constants — sources of truth (mirrored from humans.schema.json)
// =============================================================================

export const CATEGORIES = [
  'core-family',
  'workplace',
  'community',
  'affinity',
  'local-economy',
  'newcomer',
  'visitor',
  'edge-case',
  'red-team',
] as const;

export const PROFILE_REACH = [
  'private',
  'self',
  'intimate',
  'trusted',
  'familiar',
  'community',
  'public',
  'commons',
  'hidden',
] as const;

export const AGENCY_PHASES = [
  'doorway',
  'hosted',
  'device',
  'node',
  'retired',
] as const;

export const AGE_CATEGORIES = ['minor', 'adult', 'elder'] as const;

export const LOCATION_LAYERS = [
  'neighborhood',
  'municipality',
  'city',
  'county_regional',
  'bioregion',
  'nation',
  'global',
] as const;

export const INTIMACY_LEVELS = [
  'recognition',
  'connection',
  'trusted',
  'intimate',
] as const;

export const CLAIM_STATUSES = [
  'pending',
  'verified',
  'unverified',
  'disputed',
  'revoked',
] as const;

export const FLAG_SEVERITIES = [
  'info',
  'caution',
  'warning',
  'restriction',
] as const;

export const LEARNING_LEVELS = ['beginner', 'intermediate', 'advanced'] as const;

export const HUMAN_RELATIONSHIP_TYPES = [
  'spouse',
  'parent-of',
  'child-of',
  'sibling',
  'grandparent-of',
  'grandchild-of',
  'extended-family',
  'guardian-of',
  'ward-of',
  'caregiver-of',
  'key-steward-of',
  'coworker',
  'supervises',
  'reports-to',
  'business-partner',
  'neighbor',
  'congregation-member',
  'community-member',
  'mentor-of',
  'mentee-of',
  'learning-partner',
  'acquaintance',
] as const;

export const SLUG_PATTERN = /^human-[a-z0-9][a-z0-9-]*[a-z0-9]$/;

// =============================================================================
// Types
// =============================================================================

export interface HumanFrontmatter {
  id: string;
  displayName: string;
  bio?: string | null;
  agencyPhase?: string | null;
  category: string;
  profileReach: string;
  location?: { layer: string; name: string; h3Cell?: string | null } | null;
  organizations?: Array<{ id: string; name: string; role: string }>;
  communities?: string[];
  affinities?: string[];
  guardianIds?: string[];
  ageCategory?: string | null;
  isPseudonymous?: boolean;
  acceptingConnections?: boolean;
  languagePreferences?: {
    primary: string;
    secondary?: string | null;
    learningLevel?: string | null;
  } | null;
  accessibilityNeeds?: string[];
  flags?: Array<{
    type: string;
    reason: string;
    count?: number | null;
    severity?: string | null;
  }>;
  claimedAttestations?: Array<{
    claim: string;
    status: string;
    challengedAt?: string | null;
    verifiedBy?: string | null;
  }>;
}

export interface RelationshipEntry {
  source: string;
  target: string;
  relationshipType: string;
  intimacyLevel: string;
  context?: string | null;
  startedAt?: string | null;
  expiresAt?: string | null;
  notes?: string | null;
}

export interface FrontmatterValidationResult {
  errors: string[];
  warnings: string[];
}

export interface ValidationResult {
  errors: string[];
  warnings: string[];
  humans: Map<string, HumanFrontmatter>;
  relationships: RelationshipEntry[];
}

// =============================================================================
// Per-frontmatter validation
// =============================================================================

function isNonEmptyString(v: unknown): v is string {
  return typeof v === 'string' && v.length > 0;
}

/**
 * Enum-check helper mirroring `validateEnum` in validate-collectives.ts.
 *
 * - If `value == null`: emits an "is required" error iff `required`, else null.
 * - If value is a non-null string not in `allowed`: emits "is not valid" error.
 * - Returns the error string, or null when the value checks out.
 *
 * Note: callers that already emit a "missing required field" error via
 * isNonEmptyString should pass `required: false` here to avoid duplicate
 * messages and preserve existing error text.
 */
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

function pushEnum(
  errors: string[],
  value: unknown,
  field: string,
  allowed: readonly string[],
  entityId: string,
  required: boolean,
): void {
  const err = validateEnum(
    value as string | undefined | null,
    field,
    allowed,
    entityId,
    required,
  );
  if (err) errors.push(err);
}

function filenameStem(filename: string): string {
  const base = basename(filename);
  const dot = base.lastIndexOf('.');
  return dot > 0 ? base.slice(0, dot) : base;
}

export function validateHumanFrontmatter(
  fm: Partial<HumanFrontmatter> | Record<string, unknown>,
  filename: string,
): FrontmatterValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];
  const f = fm as Record<string, unknown>;
  const label = filename;

  // --- Required fields -----------------------------------------------------
  if (!isNonEmptyString(f.id)) {
    errors.push(`${label}: missing required field 'id'`);
  }
  if (!isNonEmptyString(f.displayName)) {
    errors.push(`${label}: missing required field 'displayName'`);
  }
  if (!isNonEmptyString(f.category)) {
    errors.push(`${label}: missing required field 'category'`);
  }
  if (!isNonEmptyString(f.profileReach)) {
    errors.push(`${label}: missing required field 'profileReach'`);
  }

  // --- id shape + filename match ------------------------------------------
  if (isNonEmptyString(f.id)) {
    if (!SLUG_PATTERN.test(f.id)) {
      errors.push(
        `${label}: id "${f.id}" does not match slug pattern ^human-[a-z0-9][a-z0-9-]*[a-z0-9]$`,
      );
    }
    const stem = filenameStem(filename);
    const expected = `human-${stem}`;
    if (f.id !== expected) {
      errors.push(
        `${label}: id "${f.id}" does not match filename stem (expected "${expected}")`,
      );
    }
  }

  // --- Enums ---------------------------------------------------------------
  // For `category` and `profileReach`, the "missing required field" error is
  // already emitted above via isNonEmptyString — pass required: false here so
  // the helper only emits the "is not valid" error when a value is present.
  if (isNonEmptyString(f.category)) {
    pushEnum(errors, f.category, 'category', CATEGORIES, label, false);
  }
  if (isNonEmptyString(f.profileReach)) {
    pushEnum(errors, f.profileReach, 'profileReach', PROFILE_REACH, label, false);
  }
  pushEnum(errors, f.agencyPhase, 'agencyPhase', AGENCY_PHASES, label, false);
  pushEnum(errors, f.ageCategory, 'ageCategory', AGE_CATEGORIES, label, false);

  // --- location ------------------------------------------------------------
  if (f.location != null) {
    const loc = f.location as Record<string, unknown>;
    pushEnum(errors, loc.layer, 'location.layer', LOCATION_LAYERS, label, true);
    if (!isNonEmptyString(loc.name)) {
      errors.push(`${label}: location.name is required`);
    }
  }

  // --- languagePreferences -------------------------------------------------
  if (f.languagePreferences != null) {
    const lp = f.languagePreferences as Record<string, unknown>;
    if (!isNonEmptyString(lp.primary)) {
      errors.push(`${label}: languagePreferences.primary is required`);
    }
    pushEnum(
      errors,
      lp.learningLevel,
      'languagePreferences.learningLevel',
      LEARNING_LEVELS,
      label,
      false,
    );
  }

  // --- flags ---------------------------------------------------------------
  if (Array.isArray(f.flags)) {
    for (let i = 0; i < f.flags.length; i++) {
      const flag = f.flags[i] as Record<string, unknown>;
      if (!isNonEmptyString(flag.type)) {
        errors.push(`${label}: flags[${i}].type is required`);
      }
      if (!isNonEmptyString(flag.reason)) {
        errors.push(`${label}: flags[${i}].reason is required`);
      }
      pushEnum(
        errors,
        flag.severity,
        `flags[${i}].severity`,
        FLAG_SEVERITIES,
        label,
        false,
      );
    }
  }

  // --- claimedAttestations -------------------------------------------------
  if (Array.isArray(f.claimedAttestations)) {
    for (let i = 0; i < f.claimedAttestations.length; i++) {
      const att = f.claimedAttestations[i] as Record<string, unknown>;
      if (!isNonEmptyString(att.claim)) {
        errors.push(`${label}: claimedAttestations[${i}].claim is required`);
      }
      // status is required; helper's "is required" text matches the existing
      // "claimedAttestations[i].status is required" message exactly.
      pushEnum(
        errors,
        att.status,
        `claimedAttestations[${i}].status`,
        CLAIM_STATUSES,
        label,
        true,
      );
    }
  }

  // --- organizations -------------------------------------------------------
  if (Array.isArray(f.organizations)) {
    for (let i = 0; i < f.organizations.length; i++) {
      const org = f.organizations[i] as Record<string, unknown>;
      if (!isNonEmptyString(org.id)) {
        errors.push(`${label}: organizations[${i}].id is required`);
      }
      if (!isNonEmptyString(org.name)) {
        errors.push(`${label}: organizations[${i}].name is required`);
      }
      if (!isNonEmptyString(org.role)) {
        errors.push(`${label}: organizations[${i}].role is required`);
      }
    }
  }

  // --- Warnings ------------------------------------------------------------
  if (f.ageCategory === 'minor') {
    const guardians = Array.isArray(f.guardianIds) ? f.guardianIds : [];
    if (guardians.length === 0) {
      warnings.push(
        `${label}: minor has no guardianIds (Tiffany red-team case — verify this is intentional)`,
      );
    }
  }

  return { errors, warnings };
}

// =============================================================================
// Relationship validation
// =============================================================================

export function validateRelationshipEntry(
  rel: Partial<RelationshipEntry> | Record<string, unknown>,
  index: number,
): string[] {
  const errors: string[] = [];
  const r = rel as Record<string, unknown>;
  const label = `relationships[${index}]`;

  if (!isNonEmptyString(r.source)) {
    errors.push(`${label}: missing required field 'source'`);
  }
  if (!isNonEmptyString(r.target)) {
    errors.push(`${label}: missing required field 'target'`);
  }
  if (!isNonEmptyString(r.relationshipType)) {
    errors.push(`${label}: missing required field 'relationshipType'`);
  } else {
    // Missing case already reported above via the "missing required field"
    // wording; use required: false so the helper only emits the "is not valid"
    // error when a value is present.
    pushEnum(
      errors,
      r.relationshipType,
      'relationshipType',
      HUMAN_RELATIONSHIP_TYPES,
      label,
      false,
    );
  }
  if (!isNonEmptyString(r.intimacyLevel)) {
    errors.push(`${label}: missing required field 'intimacyLevel'`);
  } else {
    pushEnum(
      errors,
      r.intimacyLevel,
      'intimacyLevel',
      INTIMACY_LEVELS,
      label,
      false,
    );
  }

  // coworker requires context
  if (r.relationshipType === 'coworker' && !isNonEmptyString(r.context)) {
    errors.push(`${label}: coworker relationships require 'context'`);
  }

  return errors;
}

// =============================================================================
// Directory-level validation
// =============================================================================

const FRONTMATTER_RE = /^---\r?\n([\s\S]*?)\r?\n---/;

function extractFrontmatter(source: string): unknown {
  const m = FRONTMATTER_RE.exec(source);
  if (!m) return null;
  return parseYaml(m[1]);
}

function detectCircularGuardianship(
  humans: Map<string, HumanFrontmatter>,
): string[] {
  const errors: string[] = [];
  const guardianMap = new Map<string, string[]>();
  for (const [id, fm] of humans.entries()) {
    if (Array.isArray(fm.guardianIds) && fm.guardianIds.length > 0) {
      guardianMap.set(id, fm.guardianIds);
    }
  }

  const reported = new Set<string>();

  for (const startId of guardianMap.keys()) {
    // DFS over guardian chain; detect when we revisit startId or hit a cycle
    const stack: Array<{ id: string; path: string[] }> = [
      { id: startId, path: [] },
    ];
    const visitedLocal = new Set<string>();
    while (stack.length > 0) {
      const { id, path } = stack.pop()!;
      if (path.includes(id)) {
        const cycleKey = [...path, id].sort().join(',');
        if (!reported.has(cycleKey)) {
          reported.add(cycleKey);
          errors.push(
            `circular guardianship detected: ${[...path, id].join(' -> ')}`,
          );
        }
        continue;
      }
      if (visitedLocal.has(id)) continue;
      visitedLocal.add(id);
      const guardians = guardianMap.get(id);
      if (!guardians) continue;
      for (const g of guardians) {
        stack.push({ id: g, path: [...path, id] });
      }
    }
  }

  return errors;
}

export async function validateHumansDirectory(
  dir: string,
): Promise<ValidationResult> {
  const result: ValidationResult = {
    errors: [],
    warnings: [],
    humans: new Map(),
    relationships: [],
  };

  let entries: string[];
  try {
    entries = await readdir(dir);
  } catch (e) {
    result.errors.push(
      `Failed to read directory ${dir}: ${e instanceof Error ? e.message : String(e)}`,
    );
    return result;
  }

  const skipFiles = new Set(['relationships.md', 'README.md']);
  const humanFiles = entries
    .filter(f => f.endsWith('.md') && !skipFiles.has(f))
    .sort();

  // Track duplicates
  const idToFile = new Map<string, string>();
  const displayNameToFile = new Map<string, string>();

  for (const file of humanFiles) {
    const full = join(dir, file);
    let source: string;
    try {
      source = await readFile(full, 'utf-8');
    } catch (e) {
      result.errors.push(
        `${file}: failed to read: ${e instanceof Error ? e.message : String(e)}`,
      );
      continue;
    }

    let fm: unknown;
    try {
      fm = extractFrontmatter(source);
    } catch (e) {
      result.errors.push(
        `${file}: failed to parse YAML frontmatter: ${e instanceof Error ? e.message : String(e)}`,
      );
      continue;
    }
    if (fm == null || typeof fm !== 'object') {
      result.errors.push(`${file}: missing or empty YAML frontmatter`);
      continue;
    }

    const perFile = validateHumanFrontmatter(
      fm as Record<string, unknown>,
      file,
    );
    result.errors.push(...perFile.errors);
    result.warnings.push(...perFile.warnings);

    const frontmatter = fm as HumanFrontmatter;
    if (isNonEmptyString(frontmatter.id)) {
      const prior = idToFile.get(frontmatter.id);
      if (prior) {
        result.errors.push(
          `${file}: duplicate id "${frontmatter.id}" (also declared in ${prior})`,
        );
      } else {
        idToFile.set(frontmatter.id, file);
        result.humans.set(frontmatter.id, frontmatter);
      }
    }
    if (isNonEmptyString(frontmatter.displayName)) {
      const prior = displayNameToFile.get(frontmatter.displayName);
      if (prior) {
        result.errors.push(
          `${file}: duplicate displayName "${frontmatter.displayName}" (also used in ${prior}) — Pete aliasing rule: every human must be distinguishable by name alone`,
        );
      } else {
        displayNameToFile.set(frontmatter.displayName, file);
      }
    }
  }

  // Referential integrity: guardianIds
  for (const [id, fm] of result.humans.entries()) {
    if (Array.isArray(fm.guardianIds)) {
      for (const guardianId of fm.guardianIds) {
        if (!result.humans.has(guardianId)) {
          result.errors.push(
            `${id}: guardianIds references unknown human "${guardianId}"`,
          );
        }
      }
    }
  }

  // Circular guardianship
  result.errors.push(...detectCircularGuardianship(result.humans));

  // Relationships
  const relationshipsPath = join(dir, 'relationships.md');
  if (entries.includes('relationships.md')) {
    let relSource: string;
    try {
      relSource = await readFile(relationshipsPath, 'utf-8');
    } catch (e) {
      result.errors.push(
        `relationships.md: failed to read: ${e instanceof Error ? e.message : String(e)}`,
      );
      return result;
    }
    let relFm: unknown;
    try {
      relFm = extractFrontmatter(relSource);
    } catch (e) {
      result.errors.push(
        `relationships.md: failed to parse YAML frontmatter: ${e instanceof Error ? e.message : String(e)}`,
      );
      return result;
    }
    if (relFm == null || typeof relFm !== 'object') {
      result.errors.push(`relationships.md: missing or empty YAML frontmatter`);
      return result;
    }
    const relArray = (relFm as Record<string, unknown>).relationships;
    if (!Array.isArray(relArray)) {
      result.errors.push(
        `relationships.md: frontmatter must contain a 'relationships' array`,
      );
      return result;
    }
    for (let i = 0; i < relArray.length; i++) {
      const rel = relArray[i] as RelationshipEntry;
      const errs = validateRelationshipEntry(rel, i);
      result.errors.push(...errs);
      // Referential integrity
      if (isNonEmptyString(rel.source) && !result.humans.has(rel.source)) {
        result.errors.push(
          `relationships[${i}]: source "${rel.source}" does not reference a known human`,
        );
      }
      if (isNonEmptyString(rel.target) && !result.humans.has(rel.target)) {
        result.errors.push(
          `relationships[${i}]: target "${rel.target}" does not reference a known human`,
        );
      }
      result.relationships.push(rel);
    }
  }

  return result;
}

// =============================================================================
// A2O narrative coherence (optional — called by CI, not by pre-push)
// =============================================================================

/**
 * Scans genesis/a2o/features/ for `human "Name"` patterns in Gherkin files.
 * Returns names that don't resolve to a known humans registry displayName,
 * excluding known synthetic-scenario personas (Alice, Bob, Newcomer, etc.).
 *
 * Intended for CI use, not pre-push — walking features is too slow for push latency.
 */
export async function validateA2oNarrativeCoherence(
  a2oFeaturesDir: string,
  humans: Map<string, HumanFrontmatter>,
): Promise<{ unresolved: string[] }> {
  const displayNames = new Set<string>();
  for (const h of humans.values()) {
    displayNames.add(h.displayName);
  }

  // Known synthetic scenario personas — created at scenario runtime, not registered as fixtures
  const syntheticNames = new Set([
    'Alice', 'Bob',                              // federation cross-doorway test personas
    'Newcomer', 'Lifecycle', 'Troublemaker',     // auth registration scenario subjects
    'IdentityTest', 'RegularUser',               // auth registration scenario subjects
    'Validator',                                 // staging persona
  ]);

  const unresolved = new Set<string>();

  function walk(dir: string): void {
    const entries = readdirSync(dir, { withFileTypes: true });
    for (const entry of entries) {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) {
        walk(path);
      } else if (entry.name.endsWith('.feature')) {
        const content = readFileSync(path, 'utf-8');
        const matches = content.matchAll(/human "([A-Z][a-zA-Z ]+)"/g);
        for (const m of matches) {
          const name = m[1].trim();
          if (!displayNames.has(name) && !syntheticNames.has(name)) {
            unresolved.add(name);
          }
        }
      }
    }
  }

  walk(a2oFeaturesDir);
  return { unresolved: Array.from(unresolved).sort() };
}

// =============================================================================
// CLI
// =============================================================================

const isCli =
  process.argv[1]?.endsWith('validate-humans.ts') ||
  process.argv[1]?.endsWith('validate-humans.js');

if (isCli) {
  const defaultDir = new URL('../../data/humans/', import.meta.url).pathname;
  const dir = process.argv[2] ?? defaultDir;

  console.log('=== Validate Humans ===\n');
  console.log(`Directory: ${dir}\n`);

  validateHumansDirectory(dir)
    .then(result => {
      console.log(`Humans:         ${result.humans.size}`);
      console.log(`Relationships:  ${result.relationships.length}`);
      console.log(`Errors:         ${result.errors.length}`);
      console.log(`Warnings:       ${result.warnings.length}\n`);

      if (result.errors.length > 0) {
        console.error('ERRORS:');
        for (const e of result.errors) console.error(`  x ${e}`);
      }
      if (result.warnings.length > 0) {
        console.log('\nWARNINGS:');
        for (const w of result.warnings) console.log(`  ! ${w}`);
      }
      if (result.errors.length === 0) {
        console.log('All validations passed');
      }
      process.exit(result.errors.length > 0 ? 1 : 0);
    })
    .catch(err => {
      console.error('Validator failed:', err);
      process.exit(2);
    });
}
