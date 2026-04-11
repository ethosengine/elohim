/**
 * Presences Seed Data Validator
 *
 * Validates genesis/data/presences/*.md (markdown with YAML frontmatter)
 * against presences.schema.json. Hand-rolled to match the validate-humans.ts
 * and validate-collectives.ts precedent (no Ajv/Zod).
 *
 * Two levels of validation:
 * 1. Per-presence frontmatter validation (enums, required fields, regex,
 *    observation structural checks).
 * 2. Directory-level referential integrity (duplicate ids, duplicate
 *    external-identifier pairs for early merge-conflict detection, optional
 *    image file existence, optional cross-refs to known humans and content).
 */

import { readFile, readdir, stat } from 'node:fs/promises';
import { join, basename } from 'node:path';
import { parse as parseYaml } from 'yaml';

// =============================================================================
// Constants — sources of truth (mirrored from presences.schema.json)
// =============================================================================

export const PRESENCE_TYPES = ['person', 'organization'] as const;

export const EXTERNAL_ID_TYPES = [
  'orcid',
  'isni',
  'wikipedia',
  'wikidata',
  'linkedin',
  'twitter',
  'mastodon',
  'personal-domain',
  'doi',
  'isbn',
  'arxiv',
  'github',
  'homepage',
  'email',
] as const;

export const WORK_KINDS = [
  'book',
  'paper',
  'talk',
  'podcast',
  'video',
  'project',
  'organization',
  'website',
  'other',
] as const;

export const PRESENCE_SLUG_PATTERN = /^presence-[a-z0-9][a-z0-9-]*[a-z0-9]$/;
export const IMAGE_LOCAL_PATTERN = /^images\/[a-z0-9-]+\.webp$/;
export const HUMAN_ID_PATTERN = /^human-[a-z0-9][a-z0-9-]*[a-z0-9]$/;
// Loose ISO-8601 date-time check: YYYY-MM-DDTHH:MM:SS with optional fraction
// and Z or numeric offset. Matches YAML::parse's string output.
const ISO_8601_RE =
  /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:?\d{2})$/;

// =============================================================================
// Types
// =============================================================================

export interface Observation {
  observerId: string;
  observedAt: string;
  context: string;
  contextContentId?: string | null;
}

export interface ExternalIdentifier {
  type: string;
  value: string;
}

export interface Work {
  title: string;
  kind: string;
  year?: number | null;
  url?: string | null;
  citedInContentIds?: string[];
}

export interface PresenceFrontmatter {
  id: string;
  displayName: string;
  presenceType: 'person' | 'organization';
  bio?: string | null;
  observations: Observation[];
  primaryStewardId?: string | null;
  stewardshipStartedAt?: string | null;
  externalIdentifiers?: ExternalIdentifier[];
  sameAsPresenceIds?: string[];
  works?: Work[];
  suggestedCollectiveIds?: string[];
  tags?: string[];
  image?: {
    local: string;
    placeholder?: boolean;
    sourceUrl?: string | null;
  } | null;
  note?: string | null;
}

export interface PresenceValidationOptions {
  knownHumanIds?: Set<string>;
  knownContentIds?: Set<string>;
  imagesDir?: string;
}

export interface FrontmatterValidationResult {
  errors: string[];
  warnings: string[];
}

export interface PresenceValidationResult {
  errors: string[];
  warnings: string[];
  presences: Map<string, PresenceFrontmatter>;
}

// =============================================================================
// Helpers
// =============================================================================

function isNonEmptyString(v: unknown): v is string {
  return typeof v === 'string' && v.length > 0;
}

/**
 * Enum-check helper mirroring `validateEnum` in validate-humans.ts /
 * validate-collectives.ts. Local copy to keep this file independently testable
 * and avoid cross-file coupling.
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

// =============================================================================
// Observation validation
// =============================================================================

export function validateObservationEntry(
  obs: Partial<Observation> | Record<string, unknown>,
  index: number,
  entityLabel: string,
  options?: PresenceValidationOptions,
): string[] {
  const errors: string[] = [];
  const o = obs as Record<string, unknown>;
  const label = `${entityLabel}: observations[${index}]`;

  if (!isNonEmptyString(o.observerId)) {
    errors.push(`${label}: missing required field 'observerId'`);
  } else if (!HUMAN_ID_PATTERN.test(o.observerId as string)) {
    errors.push(
      `${label}: observerId "${o.observerId}" does not match pattern ^human-[a-z0-9][a-z0-9-]*[a-z0-9]$`,
    );
  } else if (options?.knownHumanIds && !options.knownHumanIds.has(o.observerId as string)) {
    errors.push(
      `${label}: observerId "${o.observerId}" does not reference a known human`,
    );
  }

  if (!isNonEmptyString(o.observedAt)) {
    errors.push(`${label}: missing required field 'observedAt'`);
  } else if (!ISO_8601_RE.test(o.observedAt as string)) {
    errors.push(
      `${label}: observedAt "${o.observedAt}" is not a valid ISO-8601 date-time`,
    );
  }

  if (!isNonEmptyString(o.context)) {
    errors.push(`${label}: missing required field 'context'`);
  }

  if (o.contextContentId != null) {
    if (!isNonEmptyString(o.contextContentId)) {
      errors.push(`${label}: contextContentId must be a non-empty string or null`);
    } else if (
      options?.knownContentIds &&
      !options.knownContentIds.has(o.contextContentId as string)
    ) {
      errors.push(
        `${label}: contextContentId "${o.contextContentId}" does not reference a known content node`,
      );
    }
  }

  return errors;
}

// =============================================================================
// Per-frontmatter validation
// =============================================================================

export function validatePresenceFrontmatter(
  fm: Partial<PresenceFrontmatter> | Record<string, unknown>,
  filename: string,
  options?: PresenceValidationOptions,
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
  if (!isNonEmptyString(f.presenceType)) {
    errors.push(`${label}: missing required field 'presenceType'`);
  }
  if (!Array.isArray(f.observations)) {
    errors.push(`${label}: missing required field 'observations'`);
  }

  // --- id shape + filename match ------------------------------------------
  if (isNonEmptyString(f.id)) {
    if (!PRESENCE_SLUG_PATTERN.test(f.id)) {
      errors.push(
        `${label}: id "${f.id}" does not match slug pattern ^presence-[a-z0-9][a-z0-9-]*[a-z0-9]$`,
      );
    }
    const stem = filenameStem(filename);
    const expected = `presence-${stem}`;
    if (f.id !== expected) {
      errors.push(
        `${label}: id "${f.id}" does not match filename stem (expected "${expected}")`,
      );
    }
  }

  // --- Enums ---------------------------------------------------------------
  // presenceType "missing required" is emitted above; use required: false
  // here so we only add the "is not valid" error when a value is present.
  if (isNonEmptyString(f.presenceType)) {
    pushEnum(errors, f.presenceType, 'presenceType', PRESENCE_TYPES, label, false);
  }

  // --- Observations --------------------------------------------------------
  if (Array.isArray(f.observations)) {
    if (f.observations.length === 0) {
      errors.push(`${label}: observations must contain at least one entry`);
    }
    for (let i = 0; i < f.observations.length; i++) {
      const obs = f.observations[i] as Record<string, unknown>;
      const errs = validateObservationEntry(obs, i, label, options);
      errors.push(...errs);
    }
  }

  // --- externalIdentifiers -------------------------------------------------
  if (Array.isArray(f.externalIdentifiers)) {
    for (let i = 0; i < f.externalIdentifiers.length; i++) {
      const ext = f.externalIdentifiers[i] as Record<string, unknown>;
      pushEnum(
        errors,
        ext.type,
        `externalIdentifiers[${i}].type`,
        EXTERNAL_ID_TYPES,
        label,
        true,
      );
      if (!isNonEmptyString(ext.value)) {
        errors.push(`${label}: externalIdentifiers[${i}].value is required`);
      }
    }
  }

  // --- works ---------------------------------------------------------------
  if (Array.isArray(f.works)) {
    for (let i = 0; i < f.works.length; i++) {
      const work = f.works[i] as Record<string, unknown>;
      if (!isNonEmptyString(work.title)) {
        errors.push(`${label}: works[${i}].title is required`);
      }
      pushEnum(
        errors,
        work.kind,
        `works[${i}].kind`,
        WORK_KINDS,
        label,
        true,
      );
      if (work.year != null && typeof work.year !== 'number') {
        errors.push(
          `${label}: works[${i}].year must be an integer or null`,
        );
      }
      if (Array.isArray(work.citedInContentIds) && options?.knownContentIds) {
        for (const cid of work.citedInContentIds as unknown[]) {
          if (typeof cid === 'string' && !options.knownContentIds.has(cid)) {
            errors.push(
              `${label}: works[${i}].citedInContentIds references unknown content "${cid}"`,
            );
          }
        }
      }
    }
  }

  // --- image ---------------------------------------------------------------
  if (f.image != null) {
    const img = f.image as Record<string, unknown>;
    if (!isNonEmptyString(img.local)) {
      errors.push(`${label}: image.local is required`);
    } else if (!IMAGE_LOCAL_PATTERN.test(img.local)) {
      errors.push(
        `${label}: image.local "${img.local}" does not match pattern ^images/[a-z0-9-]+\\.webp$`,
      );
    }
  }

  return { errors, warnings };
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

async function fileExists(path: string): Promise<boolean> {
  try {
    const s = await stat(path);
    return s.isFile();
  } catch {
    return false;
  }
}

export async function validatePresencesDirectory(
  dir: string,
  options?: PresenceValidationOptions,
): Promise<PresenceValidationResult> {
  const result: PresenceValidationResult = {
    errors: [],
    warnings: [],
    presences: new Map(),
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
  const presenceFiles = entries
    .filter(f => f.endsWith('.md') && !skipFiles.has(f))
    .sort();

  // Track duplicates
  const idToFile = new Map<string, string>();
  // Track (type,value) pairs for external identifier merge-conflict detection
  const extIdToFile = new Map<string, string>();

  for (const file of presenceFiles) {
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

    const perFile = validatePresenceFrontmatter(
      fm as Record<string, unknown>,
      file,
      options,
    );
    result.errors.push(...perFile.errors);
    result.warnings.push(...perFile.warnings);

    const frontmatter = fm as PresenceFrontmatter;

    // Duplicate id tracking
    if (isNonEmptyString(frontmatter.id)) {
      const prior = idToFile.get(frontmatter.id);
      if (prior) {
        result.errors.push(
          `${file}: duplicate id "${frontmatter.id}" (also declared in ${prior})`,
        );
      } else {
        idToFile.set(frontmatter.id, file);
        result.presences.set(frontmatter.id, frontmatter);
      }
    }

    // Cross-presence external identifier uniqueness (early merge-conflict)
    if (Array.isArray(frontmatter.externalIdentifiers)) {
      for (const ext of frontmatter.externalIdentifiers) {
        if (!isNonEmptyString(ext?.type) || !isNonEmptyString(ext?.value)) {
          continue;
        }
        const key = `${ext.type}::${ext.value}`;
        const prior = extIdToFile.get(key);
        if (prior) {
          result.errors.push(
            `${file}: duplicate externalIdentifier (${ext.type}, "${ext.value}") also declared in ${prior} — possible merge candidate`,
          );
        } else {
          extIdToFile.set(key, file);
        }
      }
    }

    // Optional image file existence check
    if (options?.imagesDir && frontmatter.image != null) {
      const local = (frontmatter.image as { local?: string }).local;
      if (isNonEmptyString(local) && IMAGE_LOCAL_PATTERN.test(local)) {
        // local is "images/foo.webp"; imagesDir already points at the images
        // directory, so strip the "images/" prefix when resolving.
        const filename = local.replace(/^images\//, '');
        const resolved = join(options.imagesDir, filename);
        const exists = await fileExists(resolved);
        if (!exists) {
          result.errors.push(
            `${file}: image "${local}" does not exist at ${resolved}`,
          );
        }
      }
    }
  }

  return result;
}

// =============================================================================
// CLI
// =============================================================================

const isCli =
  process.argv[1]?.endsWith('validate-presences.ts') ||
  process.argv[1]?.endsWith('validate-presences.js');

if (isCli) {
  const defaultDir = new URL('../../data/presences/', import.meta.url).pathname;
  const dir = process.argv[2] ?? defaultDir;
  const humansDir = new URL('../../data/humans/', import.meta.url).pathname;

  console.log('=== Validate Presences ===\n');
  console.log(`Directory: ${dir}\n`);

  // Load known human ids so observer/steward references can be cross-checked.
  // This wires the knownHumanIds option that the validator plumbs through.
  (async () => {
    const knownHumanIds = new Set<string>();
    try {
      const { validateHumansDirectory } = await import('./validate-humans.js');
      const humansResult = await validateHumansDirectory(humansDir);
      for (const id of humansResult.humans.keys()) knownHumanIds.add(id);
      console.log(`Loaded ${knownHumanIds.size} known human ids for cross-reference\n`);
    } catch (err) {
      console.warn(`Warning: could not load humans directory for cross-reference (${err instanceof Error ? err.message : String(err)})`);
      console.warn('Presence observer/steward references will not be cross-validated.\n');
    }
    return validatePresencesDirectory(dir, {
      imagesDir: join(dir, 'images'),
      knownHumanIds: knownHumanIds.size > 0 ? knownHumanIds : undefined,
    });
  })()
    .then(result => {
      console.log(`Presences:      ${result.presences.size}`);
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
