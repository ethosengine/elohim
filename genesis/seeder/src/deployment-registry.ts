/**
 * Deployment Registry Loader
 *
 * Returns the PRESENCE set — every human declared in deployments.json,
 * regardless of `suspended` flag. This set drives "whose account package
 * should be imported into the network" — i.e., whose ContributorPresence,
 * relationships, stewardship, and authored content should land on a peer's
 * storage. Per the contract in
 * docs/superpowers/specs/2026-05-04-compute-commitment-substrate-floor-design.md,
 * presence is attribution-class and is independent of compute liveness.
 *
 * Note: the field name `deployedHumanIds` predates the presence/compute
 * decoupling. It is preserved for back-compat; semantically it is the
 * presence set. The active-compute set (humans whose storage can receive
 * imports) is derived separately from topology.json by the genesis pipeline,
 * passed in as SEEDER_TARGET_PEERS, and consumed by seed-accounts.ts.
 *
 * Resolution order:
 *   1. opts.deployedHumans (explicit list, from --deployed-humans flag
 *      or SEEDER_DEPLOYED_HUMANS env)
 *   2. opts.registryPath (from --registry flag or SEEDER_REGISTRY env)
 *   3. default: genesis/orchestrator/data/deployments.json relative to
 *      this module
 */

import { readFileSync, existsSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

export interface DeploymentRegistry {
  deployedHumanIds: Set<string>;
  source: 'file' | 'flag' | 'env';
  path?: string;
}

export interface LoadRegistryOptions {
  registryPath?: string;
  deployedHumans?: string[];
}

interface RegistryFile {
  humans?: Array<{ humanId: string; name?: string }>;
}

const DEFAULT_REGISTRY_RELATIVE = '../../orchestrator/data/deployments.json';

export function loadDeploymentRegistry(opts: LoadRegistryOptions = {}): DeploymentRegistry {
  if (opts.deployedHumans && opts.deployedHumans.length > 0) {
    for (const id of opts.deployedHumans) {
      if (!id.startsWith('human-')) {
        throw new Error(
          `Invalid --deployed-humans entry "${id}": expected full humanId ` +
          `(e.g. "human-adam-firstman"), not a short name. This keeps the ` +
          `contract unambiguous and avoids a circular dependency on the registry file.`
        );
      }
    }
    return {
      deployedHumanIds: new Set(opts.deployedHumans),
      source: 'flag',
    };
  }

  const __dirname = dirname(fileURLToPath(import.meta.url));
  const path = opts.registryPath ?? resolve(__dirname, DEFAULT_REGISTRY_RELATIVE);

  if (!existsSync(path)) {
    throw new Error(`Deployment registry not found: ${path}`);
  }

  let parsed: RegistryFile;
  try {
    parsed = JSON.parse(readFileSync(path, 'utf-8'));
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    throw new Error(`Failed to parse deployment registry ${path}: ${msg}`);
  }

  if (!Array.isArray(parsed.humans)) {
    throw new Error(`Deployment registry ${path} has no "humans" array`);
  }

  const deployedHumanIds = new Set<string>();
  for (const h of parsed.humans) {
    if (h && typeof h.humanId === 'string') {
      deployedHumanIds.add(h.humanId);
    }
  }

  return {
    deployedHumanIds,
    source: 'file',
    path,
  };
}

/**
 * Parse --deployed-humans value (comma-separated) into an array.
 * Empty/undefined returns undefined (so loadDeploymentRegistry falls through).
 */
export function parseDeployedHumansArg(raw: string | undefined): string[] | undefined {
  if (!raw) return undefined;
  const list = raw.split(',').map(s => s.trim()).filter(Boolean);
  return list.length > 0 ? list : undefined;
}
