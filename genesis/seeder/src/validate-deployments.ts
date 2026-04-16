/**
 * Deployment Fixture Validator
 *
 * Validates genesis/orchestrator/data/deployments.json — the operational CI
 * fixture that drives the elohim-edge pipeline's per-human manifest rendering.
 * Cross-checks every deployment record against two other fixture stores:
 *
 *   humanId        → genesis/data/humans/humans.json (.humans[].id)
 *   deviceArchetype → genesis/data/devices/devices.json (.devices[].id)
 *
 * Also checks: pattern is legacy|consolidated, template/manifest file exists,
 * nodeTypes is a non-empty subset of the allowed set, no duplicate humanIds.
 *
 * Usage:
 *   cd genesis/seeder && pnpm run validate:deployments
 *
 * Category C operational IoC contract — this validator gates whether a
 * deployments.json change can reach the pipeline. If a human is added or
 * renamed in humans.json but the deployments.json record still points at
 * the old id, this catches it before Jenkins does.
 */

import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

// =============================================================================
// Constants — sources of truth for deployment vocabulary
// =============================================================================

export const PATTERNS = ['legacy', 'consolidated'] as const;

// K8s node-type labels the cluster is provisioned with. Keep in sync with
// genesis/orchestrator/manifests/humans/*.yaml affinity blocks and with the
// operator runbook that sets `node-type=<value>` on each worker node.
export const NODE_TYPES = [
  'performance',
  'operations',
  'edge',
  'remote',
] as const;

// =============================================================================
// Types
// =============================================================================

interface DeploymentRecord {
  name: string;
  role: string;
  humanLabel: string;
  humanId: string;
  pattern: string;
  manifest?: string;
  template?: string;
  deviceArchetype: string;
  affinityComment?: string;
  nodeTypes: string[];
  edgenodeMemoryRequest?: string;
  edgenodeMemoryLimit?: string;
  edgenodeCpuRequest?: string;
  edgenodeCpuLimit?: string;
}

interface DeploymentsJson {
  $comment?: string;
  schemaVersion: number;
  humans: DeploymentRecord[];
}

interface HumansJson {
  humans: Array<{ id: string; displayName: string }>;
}

interface DevicesJson {
  devices: Array<{ id: string; displayName: string }>;
}

// =============================================================================
// Loaders
// =============================================================================

const REPO_ROOT = resolve(import.meta.dirname, '../../..');
const DEPLOYMENTS_PATH = resolve(REPO_ROOT, 'genesis/orchestrator/data/deployments.json');
const HUMANS_PATH = resolve(REPO_ROOT, 'genesis/data/humans/humans.json');
const DEVICES_PATH = resolve(REPO_ROOT, 'genesis/data/devices/devices.json');

function loadJson<T>(path: string): T {
  return JSON.parse(readFileSync(path, 'utf-8')) as T;
}

// =============================================================================
// Validation
// =============================================================================

function validateRecord(
  record: DeploymentRecord,
  knownHumanIds: Set<string>,
  knownDeviceIds: Set<string>,
): string[] {
  const errors: string[] = [];
  const tag = record.name ? `[${record.name}]` : '[?]';

  // Required presence
  for (const field of ['name', 'role', 'humanLabel', 'humanId', 'pattern', 'deviceArchetype'] as const) {
    if (!record[field]) errors.push(`${tag} missing required field: ${field}`);
  }

  // Cross-reference: humanId exists in humans.json
  if (record.humanId && !knownHumanIds.has(record.humanId)) {
    errors.push(`${tag} humanId "${record.humanId}" not found in humans.json`);
  }

  // Cross-reference: deviceArchetype exists in devices.json
  if (record.deviceArchetype && !knownDeviceIds.has(record.deviceArchetype)) {
    errors.push(`${tag} deviceArchetype "${record.deviceArchetype}" not found in devices.json`);
  }

  // Pattern enum
  if (record.pattern && !PATTERNS.includes(record.pattern as typeof PATTERNS[number])) {
    errors.push(`${tag} pattern "${record.pattern}" not in: ${PATTERNS.join(', ')}`);
  }

  // Pattern-specific file existence
  if (record.pattern === 'legacy') {
    if (!record.template) {
      errors.push(`${tag} pattern=legacy requires 'template' field`);
    } else if (!existsSync(resolve(REPO_ROOT, record.template))) {
      errors.push(`${tag} template file missing: ${record.template}`);
    }
  } else if (record.pattern === 'consolidated') {
    if (!record.manifest) {
      errors.push(`${tag} pattern=consolidated requires 'manifest' field`);
    } else if (!existsSync(resolve(REPO_ROOT, record.manifest))) {
      errors.push(`${tag} manifest file missing: ${record.manifest}`);
    }
  }

  // nodeTypes: non-empty, all in allowed set
  if (!Array.isArray(record.nodeTypes) || record.nodeTypes.length === 0) {
    errors.push(`${tag} nodeTypes must be a non-empty array`);
  } else {
    for (const nt of record.nodeTypes) {
      if (!NODE_TYPES.includes(nt as typeof NODE_TYPES[number])) {
        errors.push(`${tag} nodeType "${nt}" not in: ${NODE_TYPES.join(', ')}`);
      }
    }
  }

  // Legacy humans must declare edgenode resource sizing (consolidated humans
  // hardcode theirs in the manifest).
  if (record.pattern === 'legacy') {
    for (const field of [
      'edgenodeMemoryRequest',
      'edgenodeMemoryLimit',
      'edgenodeCpuRequest',
      'edgenodeCpuLimit',
    ] as const) {
      if (!record[field]) {
        errors.push(`${tag} pattern=legacy requires ${field}`);
      }
    }
  }

  // Sed-hazard characters in any field that gets interpolated into a sed
  // expression inside deployHumanManifest. Even with the Jenkinsfile's
  // shell-escape dance for apostrophes (see commit 5e704040), a literal
  // '|' would truncate the sed substitution (it's the delimiter) and a
  // newline would split the expression across sed commands.
  //
  // This catches the class of "looks fine in JSON, breaks in Jenkins" bugs
  // — including Adam's "Matthew's role" apostrophe (caught at runtime, not
  // by static validation), which commit 5e704040 now handles in-flight but
  // we still want flagged at validation time as a defense-in-depth.
  const sedInterpolatedFields = [
    'affinityComment',
    'edgenodeMemoryRequest',
    'edgenodeMemoryLimit',
    'edgenodeCpuRequest',
    'edgenodeCpuLimit',
    'humanLabel',
    'humanId',
  ] as const;
  for (const field of sedInterpolatedFields) {
    const value = record[field];
    if (typeof value !== 'string') continue;
    if (value.includes('|')) {
      errors.push(`${tag} ${field} contains '|' (the sed delimiter) — would truncate substitution: ${JSON.stringify(value)}`);
    }
    if (/[\n\r]/.test(value)) {
      errors.push(`${tag} ${field} contains a line break — would split the sed expression: ${JSON.stringify(value)}`);
    }
  }

  // humanId should match the pattern human-<humanLabel>
  if (record.humanId && record.humanLabel && record.humanId !== `human-${record.humanLabel}`) {
    errors.push(`${tag} humanId "${record.humanId}" must equal "human-${record.humanLabel}"`);
  }

  return errors;
}

// =============================================================================
// Entry point
// =============================================================================

async function main(): Promise<void> {
  const deployments = loadJson<DeploymentsJson>(DEPLOYMENTS_PATH);
  const humans = loadJson<HumansJson>(HUMANS_PATH);
  const devices = loadJson<DevicesJson>(DEVICES_PATH);

  const knownHumanIds = new Set(humans.humans.map(h => h.id));
  const knownDeviceIds = new Set(devices.devices.map(d => d.id));

  const errors: string[] = [];

  // Per-record validation
  for (const record of deployments.humans) {
    errors.push(...validateRecord(record, knownHumanIds, knownDeviceIds));
  }

  // Directory-level: duplicate humanIds
  const idCounts = new Map<string, number>();
  for (const r of deployments.humans) {
    idCounts.set(r.humanId, (idCounts.get(r.humanId) ?? 0) + 1);
  }
  for (const [id, count] of idCounts) {
    if (count > 1) errors.push(`duplicate humanId "${id}" in deployments.json (${count} records)`);
  }

  // Directory-level: duplicate names
  const nameCounts = new Map<string, number>();
  for (const r of deployments.humans) {
    nameCounts.set(r.name, (nameCounts.get(r.name) ?? 0) + 1);
  }
  for (const [name, count] of nameCounts) {
    if (count > 1) errors.push(`duplicate name "${name}" in deployments.json (${count} records)`);
  }

  for (const error of errors) console.error(`ERROR ${error}`);

  console.log(`\nValidated ${deployments.humans.length} deployment records: ${errors.length} errors`);

  if (errors.length > 0) process.exit(1);
  console.log('All deployment records valid.');
}

main().catch(err => {
  console.error('Unexpected error:', err);
  process.exit(1);
});
