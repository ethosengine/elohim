/**
 * Seed Bounded delegates-compute Commitments
 *
 * POSTs to the gated storage endpoint POST /admin/seed/delegates-compute
 * (Task 2 — NOT /api/v1/commitments which lands in the wrong table).
 * The endpoint writes mishpat_commitments directly with status='active'.
 *
 * The minimum-bounds guard (assertBoundedMinimum) enforces spec §14:
 *  - epr_scope ["*"] requires explicit finite rate_per_hour AND rotation_ttl_days (>=1)
 *  - reach_ceiling outside {commons,community} requires reach_elevation_acknowledged=true
 *
 * Usage:
 *   STORAGE_URL=http://localhost:8090 STORAGE_TOKEN=xxx \
 *     MATTHEW_AGENT_CID=<cid> SEED_NOW_ISO=<iso> SEED_VALID_UNTIL_ISO=<iso> \
 *     npx tsx src/seed-delegates-compute.ts
 *
 * Or with a custom pairs file:
 *   DELEGATES_PAIRS_JSON=./pairs.json npx tsx src/seed-delegates-compute.ts
 *
 * MATTHEW_AGENT_CID / SEED_NOW_ISO / SEED_VALID_UNTIL_ISO are all optional on
 * a live mesh: when unset, MATTHEW_AGENT_CID resolves from STORAGE_URL's own
 * `GET /auth/me` (matthew's live session — the same resolution seed-commitments
 * uses for custody peer ids), and the dates default to now / now+30d. This is a
 * mesh-local fallback only — no CI stage exports these envs today (this script
 * is a manual/local-mesh lever, not wired into genesis/Jenkinsfile), so the
 * fallback is exercised whenever the caller doesn't supply them, not just in
 * dev. Passing MATTHEW_AGENT_CID/SEED_NOW_ISO/SEED_VALID_UNTIL_ISO explicitly
 * always takes precedence.
 */

import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

// =============================================================================
// Minimum-bounds guard (spec §14)
// =============================================================================

/** spec §14 schema rule: ack required when ceiling is NOT commons/community. */
const SAFE_CEILINGS = new Set(['commons', 'community']);

export interface DelegatesComputeBounds {
  epr_scope: string[];
  reach_ceiling: string;
  rate_per_hour?: number;
  rotation_ttl_days?: number;
  reach_elevation_acknowledged?: boolean;
}

export interface DelegatesComputePair {
  scope: string;                 // commitment.scope, e.g. 'orchestrate-node'
  providerAgentCid: string;      // the <PERFORMER_CLAIM> value of the granting steward (Matthew)
  recipientAgentCid: string;     // the <PERFORMER_CLAIM> value the client acts AS (Matthew, self-contract)
  bounds: DelegatesComputeBounds;
  validFromIso: string;
  validUntilIso: string;
  fixture?: string;
}

export function assertBoundedMinimum(b: DelegatesComputeBounds): void {
  const hasWildcard = b.epr_scope.includes('*');
  const rateOk = typeof b.rate_per_hour === 'number' && b.rate_per_hour >= 1;
  const ttlOk = typeof b.rotation_ttl_days === 'number' && b.rotation_ttl_days >= 1;
  if (hasWildcard && !rateOk) throw new Error('minimum-bounds: epr_scope ["*"] requires a finite rate_per_hour (>=1)');
  if (hasWildcard && !ttlOk) throw new Error('minimum-bounds: epr_scope ["*"] requires a finite rotation_ttl_days (>=1)');
  if (!b.reach_ceiling) throw new Error('minimum-bounds: reach_ceiling is required');
  if (!SAFE_CEILINGS.has(b.reach_ceiling) && b.reach_elevation_acknowledged !== true) {
    throw new Error(`minimum-bounds: reach_ceiling '${b.reach_ceiling}' outside {commons,community} requires reach_elevation_acknowledged=true`);
  }
}

// =============================================================================
// Suspended-persona guard (mirrors seed-commitments.ts pattern)
// =============================================================================

/**
 * Load suspended human prefixes from deployments.json.
 * Fail-open on unreadable registry; fail-fast on readable registry that suspends
 * a referenced agent. Mirrors loadSuspendedHumanPrefixes() in seed-commitments.ts.
 */
function loadSuspendedPersonas(): Set<string> {
  try {
    const here = dirname(fileURLToPath(import.meta.url));
    const jsonPath = resolve(here, '../../orchestrator/data/deployments.json');
    const raw = readFileSync(jsonPath, 'utf-8');
    const data = JSON.parse(raw) as { humans?: { name: string; suspended?: boolean }[] };
    const suspended = new Set<string>();
    for (const h of data.humans ?? []) {
      if (h.suspended) suspended.add(h.name.toLowerCase());
    }
    return suspended;
  } catch {
    return new Set();
  }
}

/** Exit 1 when any pair's providerAgentCid or recipientAgentCid is associated
 *  with a suspended persona name. Best-effort check — CIDs are opaque, but
 *  fixture-tagged pairs can embed the name for the guard. */
function assertPairsNotSuspendedByFixture(
  pairs: DelegatesComputePair[],
  suspended: Set<string>,
): void {
  const offenders: string[] = [];
  for (const pair of pairs) {
    if (pair.fixture) {
      for (const name of suspended) {
        if (pair.fixture.toLowerCase().includes(name)) {
          offenders.push(`${pair.fixture} (suspended: ${name})`);
        }
      }
    }
  }
  if (offenders.length > 0) {
    console.error(
      `ERROR: delegates-compute pair(s) reference SUSPENDED persona(s): ${offenders.join(', ')}.\n` +
        `deployments.json suspends them — seeding for suspended personas is dead data.\n` +
        `Fix the pair set (or un-suspend the persona) before seeding.`,
    );
    process.exit(1);
  }
}

// =============================================================================
// Factory functions
// =============================================================================

function delegatesComputeId(pair: DelegatesComputePair): string {
  const d = createHash('sha256')
    .update(`${pair.providerAgentCid}|${pair.recipientAgentCid}|${pair.scope}`)
    .digest('hex')
    .slice(0, 16);
  return `delegates-compute-${d}`;
}

/**
 * Body for the gated storage seed endpoint (Task 2). The endpoint writes
 * mishpat_commitments DIRECTLY (scope/bounds_json/valid_from/valid_until/
 * recipient/provider + synthesized anchor) — NOT /api/v1/commitments.
 */
export function buildDelegatesComputeBody(pair: DelegatesComputePair) {
  assertBoundedMinimum(pair.bounds);
  return {
    cid: delegatesComputeId(pair),
    action: 'delegates-compute' as const,
    scope: pair.scope,
    provider: pair.providerAgentCid,
    recipient: pair.recipientAgentCid,
    bounds: { ...pair.bounds, _provenance: 'dev-seed' }, // honesty marker (Task 2 stores in bounds_json)
    validFrom: pair.validFromIso,
    validUntil: pair.validUntilIso,
  };
}

/**
 * POST each pair to the gated storage seed endpoint
 * (ALLOW_SEED_DELEGATES_COMPUTE=1; 403 if off). Idempotent.
 */
export async function seedDelegatesComputeCommitments(
  storageUrl: string,
  token: string,
  pairs: DelegatesComputePair[],
): Promise<void> {
  let created = 0;
  let idempotent = 0;

  for (const pair of pairs) {
    const body = buildDelegatesComputeBody(pair);
    const res = await fetch(`${storageUrl}/admin/seed/delegates-compute`, {
      method: 'POST',
      headers: { 'content-type': 'application/json', authorization: `Bearer ${token}` },
      body: JSON.stringify(body),
    });
    if (res.ok) {
      console.log(`[+] delegates-compute ${body.cid} (active)`);
      created += 1;
      continue;
    }
    if (res.status === 403) {
      // Loud, unambiguous total-failure line — matches the other legs'
      // "=== Results: ... ===" convention so a chain grep never misses this
      // exit(1) (Jenkinsfile's runProbedSeeder partial-vs-total contract:
      // 0=clean, 1=total, 2=partial; this leg has no partial state).
      console.error('=== Results: 0 created, 0 idempotent, ALLOW_SEED_DELEGATES_COMPUTE not set on this node ===');
      process.exit(1);
    }
    const text = await res.text();
    if (res.status === 409 || /exists/i.test(text)) {
      console.log(`[=] delegates-compute ${body.cid} (idempotent)`);
      idempotent += 1;
      continue;
    }
    console.error(`=== Results: ${created} created, ${idempotent} idempotent, FAILED on ${body.cid}: HTTP ${res.status} ${text} ===`);
    process.exit(1);
  }

  console.log(`=== Results: ${created} created, ${idempotent} idempotent, 0 failed (${pairs.length} total) ===`);
}

/**
 * Resolve MATTHEW_AGENT_CID: explicit env wins, else fetch matthew's live
 * agentPubKey from STORAGE_URL's own `GET /auth/me`. Mirrors the peer-id
 * resolution seed-commitments.ts already relies on (`resolveCustodyPeerIds`) —
 * on a live mesh the storage pod IS the agent's session, so there is no need
 * to make the caller compute a value the pod already knows. Throws (never
 * process.exit) so the caller can print one clear final error line.
 */
async function resolveMatthewAgentCid(storageUrl: string): Promise<string> {
  const envCid = process.env.MATTHEW_AGENT_CID;
  if (envCid) return envCid;

  const endpoint = `${storageUrl.replace(/\/+$/, '')}/auth/me`;
  let response: Response;
  try {
    response = await fetch(endpoint, { signal: AbortSignal.timeout(5000) });
  } catch (err) {
    throw new Error(
      `MATTHEW_AGENT_CID not set and ${endpoint} was unreachable — ${err instanceof Error ? err.message : String(err)}`,
    );
  }
  if (!response.ok) {
    throw new Error(`MATTHEW_AGENT_CID not set and ${endpoint} returned HTTP ${response.status}`);
  }
  const body = (await response.json()) as { agentPubKey?: unknown };
  if (typeof body.agentPubKey !== 'string' || !body.agentPubKey.startsWith('uhCAk')) {
    throw new Error(`MATTHEW_AGENT_CID not set and ${endpoint} did not return a Holochain agentPubKey`);
  }
  console.log(`[seed-delegates-compute] MATTHEW_AGENT_CID unset — resolved ${body.agentPubKey.slice(0, 16)}… from ${endpoint}`);
  return body.agentPubKey;
}

/** Explicit env wins; otherwise a loud, logged default (now / now+30d). */
function resolveDateEnv(key: string, fallback: () => string): string {
  const v = process.env[key];
  if (v) return v;
  const resolved = fallback();
  console.log(`[seed-delegates-compute] ${key} unset — defaulting to ${resolved}`);
  return resolved;
}

/**
 * Default pair: Matthew→Che self-contract (provider == recipient == MATTHEW_AGENT_CID).
 * Phase-0 <PERFORMER_CLAIM> value from env, or resolved live from storageUrl's
 * /auth/me when unset. Dates from env, or defaulted to now / now+30d when unset.
 */
export async function defaultDelegatesComputePairs(storageUrl: string): Promise<DelegatesComputePair[]> {
  const m = await resolveMatthewAgentCid(storageUrl);
  return [
    {
      scope: 'orchestrate-node',
      providerAgentCid: m,
      recipientAgentCid: m, // self-contract (spec §2 self-custody)
      bounds: {
        epr_scope: ['*'],
        reach_ceiling: 'commons',
        rate_per_hour: 60,
        rotation_ttl_days: 30,
      },
      validFromIso: resolveDateEnv('SEED_NOW_ISO', () => new Date().toISOString()),
      validUntilIso: resolveDateEnv('SEED_VALID_UNTIL_ISO', () => new Date(Date.now() + 30 * 24 * 3600 * 1000).toISOString()),
      fixture: 'che-dogfood-self-contract',
    },
  ];
}

// =============================================================================
// Standalone execution
// =============================================================================

const isMain = import.meta.url === `file://${process.argv[1]}`;
if (isMain) {
  const storageUrl = process.env.STORAGE_URL || 'http://localhost:8090';
  const token = process.env.STORAGE_TOKEN || '';

  const pairsJsonPath = process.env.DELEGATES_PAIRS_JSON;
  let pairs: DelegatesComputePair[];
  try {
    pairs = pairsJsonPath
      ? (JSON.parse(readFileSync(pairsJsonPath, 'utf-8')) as DelegatesComputePair[])
      : await defaultDelegatesComputePairs(storageUrl);
  } catch (err) {
    console.error(`=== Results: 0 created, 0 idempotent, FAILED to resolve pairs: ${err instanceof Error ? err.message : String(err)} ===`);
    process.exit(1);
  }

  // Suspended-persona guard (mirrors the terrance-drift guard in seed-commitments.ts).
  assertPairsNotSuspendedByFixture(pairs, loadSuspendedPersonas());

  console.log('='.repeat(60));
  console.log('delegates-compute Commitment Seeder');
  console.log(`  Target: ${storageUrl}/admin/seed/delegates-compute`);
  console.log(`  Pairs:  ${pairs.length}`);
  console.log('='.repeat(60));
  console.log();

  // Health-check: verify the storage endpoint is reachable.
  const health = await fetch(`${storageUrl}/health`).catch(() => null);
  if (!health || !health.ok) {
    console.error(`ERROR: Storage not reachable at ${storageUrl}/health`);
    process.exit(1);
  }

  await seedDelegatesComputeCommitments(storageUrl, token, pairs);
  process.exit(0);
}
