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

import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { parseAllDocuments } from "yaml";

// =============================================================================
// Constants — sources of truth for deployment vocabulary
// =============================================================================

export const PATTERNS = ["legacy", "consolidated"] as const;

// K8s node-type labels the cluster is provisioned with. Keep in sync with
// genesis/orchestrator/manifests/humans/*.yaml affinity blocks and with the
// operator runbook that sets `node-type=<value>` on each worker node.
export const NODE_TYPES = [
  "performance",
  "operations",
  "edge",
  "remote",
] as const;

// =============================================================================
// Types
// =============================================================================

export interface DeploymentRecord {
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
  edgenodeDbPoolSize?: string;
  edgenodeArcFactor?: string;
  k2GossipRoundTimeoutMs?: string;
  k2GossipMaxAcceptedRounds?: string;
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

const REPO_ROOT = resolve(import.meta.dirname, "../../..");
const DEPLOYMENTS_PATH = resolve(
  REPO_ROOT,
  "genesis/orchestrator/data/deployments.json",
);
const HUMANS_PATH = resolve(REPO_ROOT, "genesis/data/humans/humans.json");
const DEVICES_PATH = resolve(REPO_ROOT, "genesis/data/devices/devices.json");
const ARCHETYPE_BUDGETS_PATH = resolve(
  REPO_ROOT,
  "genesis/data/devices/archetype-resource-budgets.json",
);

function loadJson<T>(path: string): T {
  return JSON.parse(readFileSync(path, "utf-8")) as T;
}

// =============================================================================
// Validation
// =============================================================================

export function validateRecord(
  record: DeploymentRecord,
  knownHumanIds: Set<string>,
  knownDeviceIds: Set<string>,
): string[] {
  const errors: string[] = [];
  const tag = record.name ? `[${record.name}]` : "[?]";

  // Required presence
  for (const field of [
    "name",
    "role",
    "humanLabel",
    "humanId",
    "pattern",
    "deviceArchetype",
  ] as const) {
    if (!record[field]) errors.push(`${tag} missing required field: ${field}`);
  }

  // Cross-reference: humanId exists in humans.json
  if (record.humanId && !knownHumanIds.has(record.humanId)) {
    errors.push(`${tag} humanId "${record.humanId}" not found in humans.json`);
  }

  // Cross-reference: deviceArchetype exists in devices.json
  if (record.deviceArchetype && !knownDeviceIds.has(record.deviceArchetype)) {
    errors.push(
      `${tag} deviceArchetype "${record.deviceArchetype}" not found in devices.json`,
    );
  }

  // Pattern enum
  if (
    record.pattern &&
    !PATTERNS.includes(record.pattern as (typeof PATTERNS)[number])
  ) {
    errors.push(
      `${tag} pattern "${record.pattern}" not in: ${PATTERNS.join(", ")}`,
    );
  }

  // Pattern-specific file existence
  if (record.pattern === "legacy") {
    if (!record.template) {
      errors.push(`${tag} pattern=legacy requires 'template' field`);
    } else if (!existsSync(resolve(REPO_ROOT, record.template))) {
      errors.push(`${tag} template file missing: ${record.template}`);
    }
  } else if (record.pattern === "consolidated") {
    // adam carries an explicit `manifest`; every other consolidated human
    // sed-renders the shared `template` (deployments.json's own $comment
    // documents this convention). Accept whichever is present.
    const source = record.manifest ?? record.template;
    if (!source) {
      errors.push(
        `${tag} pattern=consolidated requires 'manifest' or 'template' field`,
      );
    } else if (!existsSync(resolve(REPO_ROOT, source))) {
      errors.push(`${tag} deployment source file missing: ${source}`);
    }
  }

  // edgenodeDbPoolSize renders STORAGE_DB_POOL_SIZE via sed — must be a positive
  // integer string (a non-numeric value would render a garbage env the Rust
  // parser silently discards back to the default, losing the intended override).
  if (
    record.edgenodeDbPoolSize !== undefined &&
    !/^[1-9]\d*$/.test(record.edgenodeDbPoolSize)
  ) {
    errors.push(
      `${tag} edgenodeDbPoolSize must be a positive integer string: ${JSON.stringify(record.edgenodeDbPoolSize)}`,
    );
  }

  // edgenodeArcFactor renders network.target_arc_factor via sed — the deployed
  // lever is {0,1} ONLY (0 = leecher, 1 = full authority arc; fractional is
  // upstream-blocked in kitsune2). Anything else renders an invalid conductor
  // network factor.
  if (
    record.edgenodeArcFactor !== undefined &&
    record.edgenodeArcFactor !== "0" &&
    record.edgenodeArcFactor !== "1"
  ) {
    errors.push(
      `${tag} edgenodeArcFactor must be "0" (leecher) or "1" (full arc): ${JSON.stringify(record.edgenodeArcFactor)}`,
    );
  }

  // k2GossipRoundTimeoutMs / k2GossipMaxAcceptedRounds render the consolidated
  // template's advanced.k2Gossip block via sed (2026-08-09: per-human override
  // of the household 60000/4 slow-WAN profile for multi-tenant shem
  // conductors — see _edgenode-consolidated.template.yaml). Both are
  // positive-integer strings, same contract as edgenodeDbPoolSize.
  if (
    record.k2GossipRoundTimeoutMs !== undefined &&
    !/^[1-9]\d*$/.test(record.k2GossipRoundTimeoutMs)
  ) {
    errors.push(
      `${tag} k2GossipRoundTimeoutMs must be a positive integer string: ${JSON.stringify(record.k2GossipRoundTimeoutMs)}`,
    );
  }
  if (
    record.k2GossipMaxAcceptedRounds !== undefined &&
    !/^[1-9]\d*$/.test(record.k2GossipMaxAcceptedRounds)
  ) {
    errors.push(
      `${tag} k2GossipMaxAcceptedRounds must be a positive integer string: ${JSON.stringify(record.k2GossipMaxAcceptedRounds)}`,
    );
  }

  // nodeTypes: non-empty, all in allowed set
  if (!Array.isArray(record.nodeTypes) || record.nodeTypes.length === 0) {
    errors.push(`${tag} nodeTypes must be a non-empty array`);
  } else {
    for (const nt of record.nodeTypes) {
      if (!NODE_TYPES.includes(nt as (typeof NODE_TYPES)[number])) {
        errors.push(`${tag} nodeType "${nt}" not in: ${NODE_TYPES.join(", ")}`);
      }
    }
  }

  // Legacy humans must declare edgenode resource sizing (consolidated humans
  // hardcode theirs in the manifest).
  if (record.pattern === "legacy") {
    for (const field of [
      "edgenodeMemoryRequest",
      "edgenodeMemoryLimit",
      "edgenodeCpuRequest",
      "edgenodeCpuLimit",
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
    "affinityComment",
    "edgenodeMemoryRequest",
    "edgenodeMemoryLimit",
    "edgenodeCpuRequest",
    "edgenodeCpuLimit",
    "edgenodeDbPoolSize",
    "edgenodeArcFactor",
    "k2GossipRoundTimeoutMs",
    "k2GossipMaxAcceptedRounds",
    "humanLabel",
    "humanId",
  ] as const;
  for (const field of sedInterpolatedFields) {
    const value = record[field];
    if (typeof value !== "string") continue;
    if (value.includes("|")) {
      errors.push(
        `${tag} ${field} contains '|' (the sed delimiter) — would truncate substitution: ${JSON.stringify(value)}`,
      );
    }
    if (/[\n\r]/.test(value)) {
      errors.push(
        `${tag} ${field} contains a line break — would split the sed expression: ${JSON.stringify(value)}`,
      );
    }
  }

  // humanId should match the pattern human-<humanLabel>
  if (
    record.humanId &&
    record.humanLabel &&
    record.humanId !== `human-${record.humanLabel}`
  ) {
    errors.push(
      `${tag} humanId "${record.humanId}" must equal "human-${record.humanLabel}"`,
    );
  }

  return errors;
}

// =============================================================================
// Resource-budget conformance (archetype ↔ deployment ↔ manifest)
// =============================================================================
//
// Closes the drift that silently under-provisioned adam vs its family-node-base
// archetype-mate matthew (backlog archetype-resource-conformance-validation-gap):
// matthew's 2026-06-15 CPU/pool bump moved deployments.json but never reached
// adam's separate explicit manifest, and NOTHING compared them. Three checks:
//   1. every consolidated human declares all four edgenode* budget fields;
//   2. declared resources are >= the deviceArchetype floor (below = drift);
//   3. explicit-manifest humans' manifest resources MATCH the declared budget.

export interface ArchetypeBudget {
  cpuRequest: string;
  cpuLimit: string;
  memoryRequest: string;
  memoryLimit: string;
  note?: string;
}

interface BudgetsJson {
  $comment?: string;
  schemaVersion: number;
  budgets: Record<string, ArchetypeBudget>;
}

/** CPU quantity → millicores. "1500m"→1500, "2"→2000. */
export function cpuToMillicores(v: string): number {
  const s = v.trim();
  if (s.endsWith("m")) return parseInt(s.slice(0, -1), 10);
  return Math.round(parseFloat(s) * 1000);
}

/** Memory quantity → mebibytes. "2Gi"→2048, "768Mi"→768, bare bytes→/1Mi. */
export function memToMi(v: string): number {
  const m = v.trim().match(/^(\d+(?:\.\d+)?)(Ki|Mi|Gi|Ti)?$/);
  if (!m) return NaN;
  const n = parseFloat(m[1]);
  switch (m[2]) {
    case "Ti":
      return n * 1024 * 1024;
    case "Gi":
      return n * 1024;
    case "Mi":
      return n;
    case "Ki":
      return n / 1024;
    default:
      return n / (1024 * 1024); // bare bytes
  }
}

interface EffectiveResources {
  cpuRequest?: string;
  cpuLimit?: string;
  memoryRequest?: string;
  memoryLimit?: string;
}

/**
 * Parse the elohim-node container's resources from an explicit manifest YAML
 * (multi-doc k8s). Returns null if the StatefulSet / container / resources
 * can't be found. Lets the validator assert an explicit-manifest human's
 * hardcoded resources match the budget declared in its deployments.json record
 * — the adam two-copy-drift guard.
 */
export function extractManifestResources(
  manifestAbsPath: string,
): EffectiveResources | null {
  if (!existsSync(manifestAbsPath)) return null;
  const docs = parseAllDocuments(readFileSync(manifestAbsPath, "utf-8"));
  for (const doc of docs) {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const obj = doc.toJS() as any;
    if (!obj || obj.kind !== "StatefulSet") continue;
    const containers = obj?.spec?.template?.spec?.containers;
    if (!Array.isArray(containers)) continue;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const c = containers.find((x: any) => x?.name === "elohim-node");
    const r = c?.resources;
    if (!r) continue;
    return {
      cpuRequest: r.requests?.cpu,
      cpuLimit: r.limits?.cpu,
      memoryRequest: r.requests?.memory,
      memoryLimit: r.limits?.memory,
    };
  }
  return null;
}

// [budget-key, deployment-record field, converter, human-readable label]
const BUDGET_DIMS = [
  ["cpuRequest", "edgenodeCpuRequest", cpuToMillicores, "CPU request"],
  ["cpuLimit", "edgenodeCpuLimit", cpuToMillicores, "CPU limit"],
  ["memoryRequest", "edgenodeMemoryRequest", memToMi, "memory request"],
  ["memoryLimit", "edgenodeMemoryLimit", memToMi, "memory limit"],
] as const;

/**
 * Resource-budget conformance for one consolidated record. See section header.
 */
export function validateResourceBudget(
  record: DeploymentRecord,
  budgets: Record<string, ArchetypeBudget>,
): string[] {
  const errors: string[] = [];
  const tag = record.name ? `[${record.name}]` : "[?]";
  if (record.pattern !== "consolidated") return errors; // legacy sized elsewhere

  // (1) consolidated humans must declare all four edgenode* budget fields —
  //     this is the single source of truth the archetype floor is checked
  //     against (adam's record now mirrors its explicit manifest).
  for (const [, field] of BUDGET_DIMS) {
    if (!record[field]) {
      errors.push(
        `${tag} consolidated human must declare ${field} (resource-budget source of truth)`,
      );
    }
  }

  // (2) declared resources must meet the deviceArchetype floor.
  const budget = budgets[record.deviceArchetype];
  if (!budget) {
    errors.push(
      `${tag} deviceArchetype "${record.deviceArchetype}" has no entry in archetype-resource-budgets.json — declare its k8s resource floor`,
    );
  } else {
    for (const [bkey, field, conv, label] of BUDGET_DIMS) {
      const declared = record[field];
      if (!declared) continue; // already flagged in (1)
      const have = conv(declared);
      const floor = conv(budget[bkey]);
      if (Number.isNaN(have)) {
        errors.push(`${tag} ${field} "${declared}" is not a valid k8s quantity`);
      } else if (have < floor) {
        errors.push(
          `${tag} ${label} ${declared} is BELOW the ${record.deviceArchetype} floor ${budget[bkey]} (archetype-resource-budgets.json) — under-provisioning drift`,
        );
      }
    }
  }

  // (3) explicit-manifest humans: the manifest's hardcoded resources must MATCH
  //     the declared edgenode* budget (keeps the two copies from drifting —
  //     the exact adam failure). Template humans render FROM the record, so
  //     there is no second copy to check.
  if (record.manifest) {
    const mres = extractManifestResources(resolve(REPO_ROOT, record.manifest));
    if (mres === null) {
      errors.push(
        `${tag} explicit manifest ${record.manifest} — could not extract elohim-node resources to verify against the declared budget`,
      );
    } else {
      for (const [bkey, field, conv, label] of BUDGET_DIMS) {
        const declared = record[field];
        const inManifest = mres[bkey];
        if (!declared || !inManifest) continue;
        if (conv(declared) !== conv(inManifest)) {
          errors.push(
            `${tag} manifest ${label} (${inManifest}) does not match declared ${field} (${declared}) — explicit-manifest budget drift; keep ${record.manifest} in lockstep with the record`,
          );
        }
      }
    }
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
  const budgetsJson = loadJson<BudgetsJson>(ARCHETYPE_BUDGETS_PATH);

  const knownHumanIds = new Set(humans.humans.map((h) => h.id));
  const knownDeviceIds = new Set(devices.devices.map((d) => d.id));

  const errors: string[] = [];

  // Per-record validation
  for (const record of deployments.humans) {
    errors.push(...validateRecord(record, knownHumanIds, knownDeviceIds));
    errors.push(...validateResourceBudget(record, budgetsJson.budgets));
  }

  // Directory-level: duplicate humanIds
  const idCounts = new Map<string, number>();
  for (const r of deployments.humans) {
    idCounts.set(r.humanId, (idCounts.get(r.humanId) ?? 0) + 1);
  }
  for (const [id, count] of idCounts) {
    if (count > 1)
      errors.push(
        `duplicate humanId "${id}" in deployments.json (${count} records)`,
      );
  }

  // Directory-level: duplicate names
  const nameCounts = new Map<string, number>();
  for (const r of deployments.humans) {
    nameCounts.set(r.name, (nameCounts.get(r.name) ?? 0) + 1);
  }
  for (const [name, count] of nameCounts) {
    if (count > 1)
      errors.push(
        `duplicate name "${name}" in deployments.json (${count} records)`,
      );
  }

  for (const error of errors) console.error(`ERROR ${error}`);

  console.log(
    `\nValidated ${deployments.humans.length} deployment records: ${errors.length} errors`,
  );

  if (errors.length > 0) process.exit(1);
  console.log("All deployment records valid.");
}

const isCli =
  process.argv[1]?.endsWith("validate-deployments.ts") ||
  process.argv[1]?.endsWith("validate-deployments.js");

if (isCli) {
  main().catch((err) => {
    console.error("Unexpected error:", err);
    process.exit(1);
  });
}
