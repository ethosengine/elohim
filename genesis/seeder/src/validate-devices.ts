/**
 * Device Archetypes Seed Data Validator
 *
 * Validates genesis/data/devices/*.md (markdown with YAML frontmatter) against
 * devices.schema.json enums. Hand-rolled to match the validate-humans.ts
 * precedent (no Ajv/Zod).
 *
 * Checks:
 * 1. Per-device frontmatter validation (enums, required fields, numeric ranges)
 * 2. Semantic capability checks (canSteward/canInfer/canDoorway vs capabilityLevel)
 * 3. Directory-level checks (duplicate ids / displayNames)
 */

import { readFile, readdir } from 'node:fs/promises';
import { join, basename } from 'node:path';
import { parse as parseYaml } from 'yaml';

// =============================================================================
// Constants — sources of truth (mirrored from devices.schema.json)
// =============================================================================

export const FORM_FACTORS = [
  'phone',
  'tablet',
  'laptop',
  'desktop',
  'sbc',
  'mini-pc',
  'rack-module',
  'server',
  'iot-sensor',
  'wearable',
  'thin-client',
  'fob',
  'container',
] as const;

export const STORAGE_TYPES = [
  'emmc',
  'sd-card',
  'ssd',
  'nvme',
  'hdd',
  'flash',
] as const;

export const NAT_TYPES = [
  'public',
  'symmetric-nat',
  'port-restricted',
  'carrier-grade-nat',
  'offline-first',
] as const;

export const SERVICEABILITY = [
  'none',
  'consumer',
  'modular',
  'full',
] as const;

export const CIRCULARITY = [
  'disposable',
  'recyclable',
  'repairable',
  'modular-upgradeable',
] as const;

export const DEGRADATION_MODES = [
  'cliff',
  'graceful',
  'modular',
] as const;

export const SENSORS = [
  'camera',
  'microphone',
  'lidar',
  'infrared',
  'biometric',
  'environmental',
  'accelerometer',
  'gps',
  'nfc',
  'lora',
] as const;

export const HEALTH_SURFACES = [
  'smart',
  'thermal',
  'power',
  'usb-enumeration',
  'battery-health',
  'memory-ecc',
  'fan-rpm',
] as const;

export const ATTESTATION_CAPABILITIES = [
  'voice-presence',
  'facial-presence',
  'physical-presence',
  'hardware-key-signing',
  'spatial-occupancy',
  'environmental-conditions',
  'biometric-identity',
] as const;

export const REPLACEMENT_LEAD_TIMES = [
  'hours',
  'days',
  'weeks',
] as const;

// =============================================================================
// Types
// =============================================================================

export interface DeviceFrontmatter {
  id: string;
  displayName: string;
  formFactor: string;
  capabilityLevel: number;
  stage?: number | null;
  memoryGb: number;
  storageGb: number;
  storageType: string;
  cpuCores: number;
  cpuClass?: string | null;
  gpu?: string | null;
  sensors?: string[];
  battery?: boolean | null;
  powerWatts?: number | null;
  alwaysOn?: boolean | null;
  natType?: string | null;
  bandwidthDownMbps?: number | null;
  bandwidthUpMbps?: number | null;
  latencyMs?: number | null;
  canSteward?: boolean | null;
  canInfer?: boolean | null;
  canDoorway?: boolean | null;
  streamsTo?: string | null;
  serviceability?: string | null;
  healthSurfaces?: string[];
  circularity?: string | null;
  degradationMode?: string | null;
  replacementLeadTime?: string | null;
  expectedLifespanYears?: number | null;
  attestationCapabilities?: string[];
}

export interface FrontmatterValidationResult {
  errors: string[];
  warnings: string[];
}

export interface ValidationResult {
  errors: string[];
  warnings: string[];
  devices: Map<string, DeviceFrontmatter>;
}

// =============================================================================
// Helpers
// =============================================================================

function isNonEmptyString(v: unknown): v is string {
  return typeof v === 'string' && v.length > 0;
}

/**
 * Enum-check helper mirroring `validateEnum` in validate-humans.ts.
 *
 * - If `value == null`: emits "is required" error iff `required`, else null.
 * - If value is a non-null string not in `allowed`: emits "is not valid" error.
 * - Returns the error string, or null when the value checks out.
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

const FRONTMATTER_RE = /^---\r?\n([\s\S]*?)\r?\n---/;

function extractFrontmatter(source: string): unknown {
  const m = FRONTMATTER_RE.exec(source);
  if (!m) return null;
  return parseYaml(m[1]);
}

// =============================================================================
// Per-frontmatter validation
// =============================================================================

export function validateDeviceFrontmatter(
  fm: Partial<DeviceFrontmatter> | Record<string, unknown>,
  filename: string,
): FrontmatterValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];
  const f = fm as Record<string, unknown>;
  const label = filename;

  // --- Required fields -------------------------------------------------------
  if (!isNonEmptyString(f.id)) {
    errors.push(`${label}: missing required field 'id'`);
  }
  if (!isNonEmptyString(f.displayName)) {
    errors.push(`${label}: missing required field 'displayName'`);
  }
  if (!isNonEmptyString(f.formFactor)) {
    errors.push(`${label}: missing required field 'formFactor'`);
  }
  if (f.capabilityLevel == null) {
    errors.push(`${label}: missing required field 'capabilityLevel'`);
  }
  if (f.memoryGb == null) {
    errors.push(`${label}: missing required field 'memoryGb'`);
  }
  if (f.storageGb == null) {
    errors.push(`${label}: missing required field 'storageGb'`);
  }
  if (!isNonEmptyString(f.storageType)) {
    errors.push(`${label}: missing required field 'storageType'`);
  }
  if (f.cpuCores == null) {
    errors.push(`${label}: missing required field 'cpuCores'`);
  }

  // --- id shape + filename match ---------------------------------------------
  if (isNonEmptyString(f.id)) {
    const stem = filenameStem(filename);
    const expected = `device-${stem}`;
    if (f.id !== expected) {
      errors.push(
        `${label}: id "${f.id}" does not match filename stem (expected "${expected}")`,
      );
    }
  }

  // --- Enums -----------------------------------------------------------------
  if (isNonEmptyString(f.formFactor)) {
    pushEnum(errors, f.formFactor, 'formFactor', FORM_FACTORS, label, false);
  }
  if (isNonEmptyString(f.storageType)) {
    pushEnum(errors, f.storageType, 'storageType', STORAGE_TYPES, label, false);
  }
  pushEnum(errors, f.natType, 'natType', NAT_TYPES, label, false);
  pushEnum(errors, f.serviceability, 'serviceability', SERVICEABILITY, label, false);
  pushEnum(errors, f.circularity, 'circularity', CIRCULARITY, label, false);
  pushEnum(errors, f.degradationMode, 'degradationMode', DEGRADATION_MODES, label, false);
  pushEnum(errors, f.replacementLeadTime, 'replacementLeadTime', REPLACEMENT_LEAD_TIMES, label, false);

  // --- Array enums: sensors, healthSurfaces, attestationCapabilities ---------
  if (Array.isArray(f.sensors)) {
    for (let i = 0; i < f.sensors.length; i++) {
      pushEnum(errors, f.sensors[i], `sensors[${i}]`, SENSORS, label, true);
    }
  }
  if (Array.isArray(f.healthSurfaces)) {
    for (let i = 0; i < f.healthSurfaces.length; i++) {
      pushEnum(errors, f.healthSurfaces[i], `healthSurfaces[${i}]`, HEALTH_SURFACES, label, true);
    }
  }
  if (Array.isArray(f.attestationCapabilities)) {
    for (let i = 0; i < f.attestationCapabilities.length; i++) {
      pushEnum(
        errors,
        f.attestationCapabilities[i],
        `attestationCapabilities[${i}]`,
        ATTESTATION_CAPABILITIES,
        label,
        true,
      );
    }
  }

  // --- Numeric ranges --------------------------------------------------------
  if (typeof f.capabilityLevel === 'number') {
    if (!Number.isInteger(f.capabilityLevel) || f.capabilityLevel < 0 || f.capabilityLevel > 5) {
      errors.push(`${label}: capabilityLevel must be an integer 0-5 (got ${f.capabilityLevel})`);
    }
  }
  if (f.stage != null && typeof f.stage === 'number') {
    if (!Number.isInteger(f.stage) || f.stage < 1 || f.stage > 4) {
      errors.push(`${label}: stage must be an integer 1-4 (got ${f.stage})`);
    }
  }
  if (typeof f.memoryGb === 'number' && f.memoryGb < 0) {
    errors.push(`${label}: memoryGb must be >= 0`);
  }
  if (typeof f.storageGb === 'number' && f.storageGb < 0) {
    errors.push(`${label}: storageGb must be >= 0`);
  }
  if (typeof f.cpuCores === 'number' && f.cpuCores < 1) {
    errors.push(`${label}: cpuCores must be >= 1`);
  }
  if (typeof f.bandwidthDownMbps === 'number' && f.bandwidthDownMbps < 0) {
    errors.push(`${label}: bandwidthDownMbps must be >= 0`);
  }
  if (typeof f.bandwidthUpMbps === 'number' && f.bandwidthUpMbps < 0) {
    errors.push(`${label}: bandwidthUpMbps must be >= 0`);
  }

  // --- Semantic capability checks --------------------------------------------
  const level = typeof f.capabilityLevel === 'number' ? f.capabilityLevel : null;
  if (level !== null) {
    if (f.canSteward === true && level <= 1) {
      errors.push(
        `${label}: canSteward is true but capabilityLevel is ${level} (level 0-1 cannot steward)`,
      );
    }
    if (f.canInfer === true && level < 4) {
      errors.push(
        `${label}: canInfer is true but capabilityLevel is ${level} (level < 4 cannot infer)`,
      );
    }
    if (f.canDoorway === true && level < 5) {
      errors.push(
        `${label}: canDoorway is true but capabilityLevel is ${level} (level < 5 cannot run doorway)`,
      );
    }
  }

  return { errors, warnings };
}

// =============================================================================
// Directory-level validation
// =============================================================================

export async function validateDevicesDirectory(
  dir: string,
): Promise<ValidationResult> {
  const result: ValidationResult = {
    errors: [],
    warnings: [],
    devices: new Map(),
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

  const deviceFiles = entries
    .filter(f => f.endsWith('.md') && !f.startsWith('_'))
    .sort();

  // Track duplicates
  const idToFile = new Map<string, string>();
  const displayNameToFile = new Map<string, string>();

  for (const file of deviceFiles) {
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

    const perFile = validateDeviceFrontmatter(fm as Record<string, unknown>, file);
    result.errors.push(...perFile.errors);
    result.warnings.push(...perFile.warnings);

    const frontmatter = fm as DeviceFrontmatter;

    if (isNonEmptyString(frontmatter.id)) {
      const prior = idToFile.get(frontmatter.id);
      if (prior) {
        result.errors.push(
          `${file}: duplicate id "${frontmatter.id}" (also declared in ${prior})`,
        );
      } else {
        idToFile.set(frontmatter.id, file);
        result.devices.set(frontmatter.id, frontmatter);
      }
    }

    if (isNonEmptyString(frontmatter.displayName)) {
      const prior = displayNameToFile.get(frontmatter.displayName);
      if (prior) {
        result.errors.push(
          `${file}: duplicate displayName "${frontmatter.displayName}" (also used in ${prior})`,
        );
      } else {
        displayNameToFile.set(frontmatter.displayName, file);
      }
    }
  }

  return result;
}

// =============================================================================
// Entry point
// =============================================================================

async function main(): Promise<void> {
  const dir = join(import.meta.dirname, '../../data/devices');
  const result = await validateDevicesDirectory(dir);

  for (const warning of result.warnings) {
    console.warn(`WARN  ${warning}`);
  }
  for (const error of result.errors) {
    console.error(`ERROR ${error}`);
  }

  const total = result.devices.size;
  const errorCount = result.errors.length;

  console.log(`\nValidated ${total} device archetypes: ${errorCount} errors`);

  if (errorCount > 0) {
    process.exit(1);
  }
}

main().catch(err => {
  console.error('Unexpected error:', err);
  process.exit(1);
});
