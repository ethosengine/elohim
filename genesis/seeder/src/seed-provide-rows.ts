/**
 * Seed Content-Provide Rows (EPR Resilience Card — Workstream D)
 *
 * Seeds `rea_commitments` rows with action='provide', state='active',
 * resource_classified_as='content:<reach>' for each deployed human.  These
 * rows are what `household_resilience::snapshot()` reads for the
 * `commitment_backed_collectives` count (the "Commitment-backed" column on
 * the resilience card).
 *
 * ## Why this script exists
 *
 * The production path that writes these rows (the provide-loop in
 * `provide_reconcile.rs`) is gated on `SELF_CID` being set at startup.
 * `SELF_CID` is sourced from the `SELF_CID` env var in the pod manifest,
 * which was never set, leaving the provide-loop permanently dormant and
 * `commitment_backed_collectives` permanently 0 (diagnosed:
 * `backlog/resilience-card-self-cid-provide-loop-gate.md`).
 *
 * The parallel elohim-storage agent is fixing the runtime path (deriving
 * self_cid at startup from the in-pod conductor). This script is the
 * seed-time path: it primes the rows so the card lights even before the
 * runtime loop ticks, and every `RESET_STORAGE` reseed restores them.
 *
 * ## Shape contract (must match `household_resilience::snapshot()`)
 *
 * The snapshot SQL at `household_resilience.rs:169-188`:
 *   SELECT count(DISTINCT humans.household_id)
 *   FROM rea_commitments
 *   JOIN humans ON humans.agent_pub_key = rea_commitments.provider
 *   WHERE action='provide' AND state='active'
 *     AND resource_classified_as='content:<reach>'
 *     AND humans.household_id IS NOT NULL
 *
 * So `provider` MUST equal `humans.agent_pub_key` — the Holochain
 * agent public key (`uhCAk…`), NOT the libp2p peer id. This script
 * fetches it from each pod's `GET /auth/me` endpoint, which returns the
 * `agent_pub_key` from the pod's local session.
 *
 * ## Conductor seeding pre-condition
 *
 * The `humans` table row for each deployed human is created by the conductor
 * identity seeder (`seed-conductor-identities.ts`) AND updated by the
 * `MembershipProjected` signal with `agent_pub_key`. This script runs AFTER
 * conductor seeding (the Jenkinsfile gates it on CONDUCTOR_SEEDING_READY).
 * If a pod has no local session yet, the `/auth/me` call returns 401 and
 * that human is skipped with a warning (non-fatal; the row won't join, but
 * the partial set still lights the card for the humans that did seed).
 *
 * ## Idempotency
 *
 * `provider` (agent_pub_key) + `resource_classified_as` ('content:<reach>')
 * uniquely identify the row via `provide_projection_id()` in Rust — which
 * writes `insert_or_ignore_into`. We POST with a content-addressed `id` and
 * handle 409 as idempotent re-runs. PATCH to `active` runs unconditionally
 * after create (idempotent: active→active is a no-op).
 *
 * ## Usage
 *
 *   DOORWAY_URL=https://doorway-alpha.elohim.host \
 *   DOORWAY_API_KEY=xxx \
 *   PEER_STORAGE_URLS=matthew=elohim-matthew-alpha.elohim-alpha.svc.cluster.local:8090,... \
 *     npx tsx src/seed-provide-rows.ts
 *
 *   PEER_STORAGE_URLS defaults to the PEER_STORAGE_URL_TEMPLATE pattern from
 *   peer-id.ts (cluster-local, same URL scheme the peer-id resolver uses).
 *
 * Exit codes:
 *   0 — all targeted humans seeded or already exist
 *   2 — partial: at least one human seeded, at least one skipped/failed
 *   1 — total failure: ZERO humans seeded (or pre-flight error)
 */

import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { DoorwayClient } from './doorway-client.js';
import { storageUrlForHuman } from './peer-id.js';

// =============================================================================
// Configuration
// =============================================================================

const DOORWAY_URL = process.env.DOORWAY_URL || 'http://localhost:8888';
const API_KEY = process.env.DOORWAY_API_KEY;

/**
 * Default reach used for the provide rows. 'commons' is the reach the
 * provide-loop authors at Stage A (the self-CID fix sprint). When the
 * runtime path is fixed, it will author per-content-reach rows; this seed
 * primes the commons level so the card is non-zero before that lands.
 */
const DEFAULT_REACH = 'commons';

/** Timeout for one /auth/me probe. */
const AUTH_ME_TIMEOUT_MS = 8000;

// =============================================================================
// Parse PEER_STORAGE_URLS env (name=host:port,...)
// =============================================================================

/**
 * Build a map from human short name → storage base URL.
 * Falls back to the PEER_STORAGE_URL_TEMPLATE pattern from peer-id.ts when
 * PEER_STORAGE_URLS is absent.
 *
 * PEER_STORAGE_URLS format (from peerStorageUrlsCsv() in Jenkinsfile):
 *   matthew=elohim-matthew-alpha.elohim-alpha.svc.cluster.local:8090,...
 */
function buildStorageUrlMap(humanIds: string[]): Map<string, string> {
  const map = new Map<string, string>();

  const csv = process.env.PEER_STORAGE_URLS;
  if (csv) {
    for (const pair of csv.split(',')) {
      const eq = pair.indexOf('=');
      if (eq < 0) continue;
      const shortName = pair.slice(0, eq).trim();
      const hostPort = pair.slice(eq + 1).trim();
      if (shortName && hostPort) {
        // Host:port — add http:// scheme.
        map.set(shortName, `http://${hostPort}`);
      }
    }
  }

  // Fill gaps for any humanId not covered by the CSV using the peer-id.ts
  // storageUrlForHuman() helper (PEER_STORAGE_URL_TEMPLATE env or default).
  for (const humanId of humanIds) {
    const shortName = humanId.replace(/^human-/, '').split('-')[0];
    if (!map.has(shortName)) {
      map.set(shortName, storageUrlForHuman(humanId));
    }
  }

  return map;
}

// =============================================================================
// Fetch agent_pub_key from a pod's /auth/me
// =============================================================================

interface AuthMeResponse {
  agentPubKey?: string;
  // Other fields not needed here
}

/**
 * Fetch the Holochain agent public key from a per-pod storage endpoint.
 * Returns null on any failure (pod unreachable, no session, 401).
 *
 * Uses GET /auth/me (storage's own session endpoint, not doorway's), which
 * returns the same MeResponse shape including `agentPubKey` when a local
 * session is registered. The agent_pub_key is the `uhCAk…` base64url-encoded
 * Holochain key that also populates `humans.agent_pub_key` via the
 * MembershipProjected signal — the snapshot join key.
 */
async function fetchAgentPubKey(
  storageBaseUrl: string,
  humanId: string
): Promise<string | null> {
  const url = `${storageBaseUrl.replace(/\/+$/, '')}/auth/me`;
  try {
    const response = await fetch(url, {
      signal: AbortSignal.timeout(AUTH_ME_TIMEOUT_MS),
    });
    if (!response.ok) {
      // 401 = no local session yet (pod not yet identity-seeded by conductor
      // seeder or /auth/me requires authentication — either way, skip).
      if (response.status === 401 || response.status === 404) {
        console.warn(
          `  [?] ${humanId}: /auth/me → HTTP ${response.status} — pod has no ` +
            `local session yet (conductor seeding may not have run for this pod)`
        );
      } else {
        console.warn(
          `  [?] ${humanId}: /auth/me → HTTP ${response.status} — skipping`
        );
      }
      return null;
    }
    const body = (await response.json()) as AuthMeResponse;
    const key = body.agentPubKey;
    if (typeof key === 'string' && key.length > 0) {
      return key;
    }
    console.warn(
      `  [?] ${humanId}: /auth/me response missing agentPubKey field — skipping`
    );
    return null;
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.warn(
      `  [?] ${humanId}: /auth/me failed (${msg}) — pod may be unreachable; skipping`
    );
    return null;
  }
}

// =============================================================================
// Commitment shape (matches the snapshot join contract)
// =============================================================================

/**
 * Content-addressed id for a (provider, reach) provide row — mirrors the
 * `provide_projection_id()` Rust function so re-runs produce the SAME id
 * and the POST returns 409 (idempotent re-seed).
 *
 * Rust implementation (rea_commitments.rs):
 *   fn provide_projection_id(provider: &str, reach: &str) -> String {
 *     let digest = sha256(format!("provide:{provider}:{reach}"));
 *     format!("provide-{hex16}")
 *   }
 */
function provideProjectionId(provider: string, reach: string): string {
  const digest = createHash('sha256')
    .update(`provide:${provider}:${reach}`, 'utf8')
    .digest('hex')
    .slice(0, 16);
  return `provide-${digest}`;
}

interface ProvideCommitmentBody {
  id: string;
  action: 'provide';
  provider: string;
  receiver: string;
  resourceClassifiedAs: string[];
  note: string;
  metadata: Record<string, unknown>;
}

function buildProvideBody(
  agentPubKey: string,
  reach: string,
  humanId: string
): ProvideCommitmentBody {
  const scope = `content:${reach}`;
  return {
    id: provideProjectionId(agentPubKey, reach),
    action: 'provide',
    provider: agentPubKey,
    receiver: '',
    resourceClassifiedAs: [scope],
    note: `${humanId} provides content at reach=${reach} (seed-time bootstrap)`,
    metadata: {
      seedGeneration: 'genesis',
      humanId,
      reach,
      seedScript: 'seed-provide-rows',
    },
  };
}

// =============================================================================
// Commitment client (thin extension of DoorwayClient)
// =============================================================================

class ProvideClient extends DoorwayClient {
  async createCommitment(body: ProvideCommitmentBody): Promise<Response> {
    return this.fetch('/api/v1/commitments', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
  }

  /**
   * STOPGAP (resolver spec §3.4 / §5 step 1):
   * `genesis/docs/superpowers/specs/2026-06-15-coherent-transport-identity-resolver-design.md`.
   *
   * Heal the slug-keyed `humans` row's NULL `agent_pub_key` so it equals the
   * commitment `provider` (the `uhCAk…` agent key) the snapshot join compares
   * against (`household_resilience.rs:172-174`). Production rows are created by
   * `seed-humans.ts` with `agentPubKey: null`; this stamps the truthful key
   * fetched from `/auth/me` so the existing direct-equality join lights the
   * `commitment_backed_collectives` column.
   *
   * NULL-only on the storage side — never overwrites a set key. Routed through
   * the SAME doorway channel as the provide-row write so the heal and the
   * commitment co-locate in the same projection DB (the join is intra-pod).
   * `household_id` is intentionally omitted: `seed-humans.ts` already set it.
   */
  async healHumanIdentity(humanId: string, agentPubKey: string): Promise<Response> {
    return this.fetch('/api/v1/identity/heal', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ id: humanId, agentPubKey }),
    });
  }

  async patchCommitmentState(id: string, state: string): Promise<Response> {
    return this.fetch(`/api/v1/commitments/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ state }),
    });
  }
}

// =============================================================================
// Per-human provide seeding
// =============================================================================

type ProvideResult = 'created' | 'exists' | 'failed' | 'skipped';

async function seedProvideForHuman(
  client: ProvideClient,
  humanId: string,
  agentPubKey: string,
  reach: string
): Promise<ProvideResult> {
  const body = buildProvideBody(agentPubKey, reach, humanId);
  const label = humanId.replace(/^human-/, '');

  // POST — create the row (proposed state)
  const createResp = await client.createCommitment(body);
  let alreadyExisted = false;

  if (createResp.ok) {
    console.log(`  [+] ${label} (agentKey=${agentPubKey.slice(0, 12)}…) → ${body.resourceClassifiedAs[0]}`);
  } else {
    const text = await createResp.text();
    if (
      createResp.status === 409 ||
      text.includes('UNIQUE') ||
      text.includes('already exists')
    ) {
      console.log(`  [=] ${label} → ${body.resourceClassifiedAs[0]} (already exists)`);
      alreadyExisted = true;
    } else {
      console.error(
        `  [X] ${label}: POST /api/v1/commitments HTTP ${createResp.status}\n` +
          `      Body: ${text.slice(0, 300)}\n` +
          `      Sent: ${JSON.stringify(body).slice(0, 300)}`
      );
      return 'failed';
    }
  }

  // PATCH → active (idempotent: active→active is a no-op in storage)
  const patchResp = await client.patchCommitmentState(body.id, 'active');
  if (patchResp.ok) {
    if (!alreadyExisted) {
      console.log(`  [A] ${label} → activated`);
    }
    return alreadyExisted ? 'exists' : 'created';
  }

  if (patchResp.status === 404) {
    // The POST above never landed; 404 on PATCH means the row is absent.
    console.warn(`  [?] ${label}: PATCH state=active 404 — POST did not persist; non-fatal`);
    return 'failed';
  }

  const patchText = await patchResp.text();
  console.error(
    `  [X] ${label}: PATCH state=active HTTP ${patchResp.status}\n` +
      `      Body: ${patchText.slice(0, 300)}`
  );
  return 'failed';
}

// =============================================================================
// Main
// =============================================================================

// Last-resort humans to seed provide rows for, used ONLY when
// deployments.json cannot be read/parsed (see resolveTargetHumanIds below).
// Mirrors the historical getHumanStorageUrls() fallback that used to live in
// genesis/Jenkinsfile (matthew + adam + jessica were the live alpha cluster
// peers at the time this trio was hardcoded).
const FALLBACK_HUMAN_IDS = [
  'human-matthew-manager',
  'human-adam-firstman',
  'human-jessica-spouse',
];

/** Humans known to be actively deployed today — used only for a sanity WARN
 *  (not an enforcement), so a topology change doesn't silently go unnoticed. */
const EXPECTED_ACTIVE_HUMAN_IDS = [
  'human-adam-firstman',
  'human-matthew-manager',
  'human-jessica-spouse',
];

const __dirname = dirname(fileURLToPath(import.meta.url));
const DEFAULT_DEPLOYMENTS_JSON_PATH = resolve(
  __dirname,
  '../../orchestrator/data/deployments.json'
);

// =============================================================================
// Derive the active-human set from deployments.json
// =============================================================================

export interface DeploymentHumanRecord {
  humanId?: string;
  suspended?: boolean;
}

export interface DeploymentsRegistryFile {
  humans?: DeploymentHumanRecord[];
}

/**
 * Pure derivation: humans declared in deployments.json whose `suspended`
 * flag is NOT `true` (absent/false = deployed and active). Mirrors the
 * suspended-filtering semantics documented in CLAUDE.md's "Substrate scope
 * trigger" section — `suspended` is the deploy-render/seed/a2o gate, and a
 * human with `suspended: true` has no live conductor to seed provide rows
 * against.
 *
 * Order-preserving (matches deployments.json's `humans` array order) so
 * output is deterministic across runs.
 */
export function deriveActiveHumanIds(data: DeploymentsRegistryFile): string[] {
  if (!data || !Array.isArray(data.humans)) return [];
  const ids: string[] = [];
  for (const h of data.humans) {
    if (h && typeof h.humanId === 'string' && h.suspended !== true) {
      ids.push(h.humanId);
    }
  }
  return ids;
}

/**
 * Reads + parses deployments.json and derives the active-human set. Throws
 * on any read/parse failure — callers decide the fallback behavior (this
 * function stays pure-ish and testable; resolveTargetHumanIds is where the
 * WARN + fallback policy lives).
 */
export function loadActiveHumanIdsFromDeployments(path: string = DEFAULT_DEPLOYMENTS_JSON_PATH): string[] {
  const raw = readFileSync(path, 'utf-8');
  const parsed = JSON.parse(raw) as DeploymentsRegistryFile;
  return deriveActiveHumanIds(parsed);
}

/**
 * Resolve the target human-id set, in precedence order:
 *   1. PROVIDE_HUMAN_IDS env (explicit override — always wins)
 *   2. deployments.json-derived active set (suspended !== true)
 *   3. FALLBACK_HUMAN_IDS (only if deployments.json is unreadable/unparsable
 *      or yields an empty set) — logged as a WARN, never silent.
 *
 * Exported for tests; accepts an optional deploymentsPath override so tests
 * don't need to touch the real genesis/orchestrator/data/deployments.json.
 */
export function resolveTargetHumanIds(deploymentsPath?: string): string[] {
  const envIds = (process.env.PROVIDE_HUMAN_IDS || '')
    .split(',')
    .map(s => s.trim())
    .filter(Boolean);
  if (envIds.length > 0) {
    return envIds;
  }

  try {
    const derived = loadActiveHumanIdsFromDeployments(deploymentsPath);
    if (derived.length === 0) {
      console.warn(
        'WARN: deployments.json parsed but yielded zero active (suspended!==true) humans — ' +
          `falling back to the hardcoded trio: ${FALLBACK_HUMAN_IDS.join(', ')}`
      );
      return FALLBACK_HUMAN_IDS;
    }
    for (const expected of EXPECTED_ACTIVE_HUMAN_IDS) {
      if (!derived.includes(expected)) {
        console.warn(
          `WARN: expected active human "${expected}" not found in the deployments.json-derived ` +
            `set (${derived.join(', ')}) — topology may have changed; verify deployments.json`
        );
      }
    }
    console.log(
      `Derived ${derived.length} active humans from deployments.json (suspended!==true): ${derived.join(', ')}`
    );
    return derived;
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.warn(
      `WARN: could not read/parse deployments.json (${msg}) — falling back to the hardcoded trio: ` +
        `${FALLBACK_HUMAN_IDS.join(', ')}`
    );
    return FALLBACK_HUMAN_IDS;
  }
}

// Extra reaches to seed beyond commons.  'commons' is always seeded;
// PROVIDE_REACHES env extends it (comma-separated, e.g. "commons,household").
function reachesToSeed(): string[] {
  const extra = (process.env.PROVIDE_REACHES || '').split(',').map(s => s.trim()).filter(Boolean);
  return [...new Set([DEFAULT_REACH, ...extra])];
}

async function main(): Promise<void> {
  const targetHumans = resolveTargetHumanIds();
  const reaches = reachesToSeed();

  console.log('='.repeat(60));
  console.log('Content Provide Row Seeder (EPR Resilience Card — Workstream D)');
  console.log(`  Target: ${DOORWAY_URL}`);
  console.log(`  Humans: ${targetHumans.join(', ')}`);
  console.log(`  Reaches: ${reaches.join(', ')}`);
  console.log('='.repeat(60));
  console.log();

  const client = new ProvideClient({ baseUrl: DOORWAY_URL, apiKey: API_KEY });

  // Pre-flight: doorway health check.
  const health = await client.checkHealth();
  if (!health.healthy) {
    console.error(`ERROR: Doorway not healthy — ${health.error}`);
    process.exit(1);
  }

  const storageUrlMap = buildStorageUrlMap(targetHumans);

  let created = 0;
  let alreadyExisted = 0;
  let skipped = 0;
  let failed = 0;

  for (const humanId of targetHumans) {
    const shortName = humanId.replace(/^human-/, '').split('-')[0];
    const storageUrl = storageUrlMap.get(shortName) ?? storageUrlForHuman(humanId);

    console.log(`[${humanId}] Fetching agent_pub_key from ${storageUrl}/auth/me ...`);
    const agentPubKey = await fetchAgentPubKey(storageUrl, humanId);

    if (!agentPubKey) {
      console.warn(
        `  SKIPPED: no agent_pub_key — conductor seeding must run first ` +
          `(Seed Conductor Identities stage) and the pod must have a local session`
      );
      skipped += 1;
      continue;
    }

    console.log(`  agent_pub_key: ${agentPubKey.slice(0, 16)}…`);

    // STOPGAP (resolver spec §3.4): heal humans.agent_pub_key = uhCAk BEFORE
    // writing provide rows, so the snapshot join (humans.agent_pub_key =
    // rea_commitments.provider) is satisfied. NULL-only on the storage side;
    // per-human non-fatal (mirror the /auth/me 401-skip pattern — a heal
    // failure must not block the provide seed for the remaining humans).
    try {
      const healResp = await client.healHumanIdentity(humanId, agentPubKey);
      if (healResp.ok) {
        console.log(`  [H] ${humanId.replace(/^human-/, '')}: agent_pub_key healed`);
      } else if (healResp.status === 404) {
        // Row not present yet (seed-humans bridge has not run for this human).
        console.warn(
          `  [?] ${humanId}: identity heal 404 — humans row absent (seed-humans must run first); non-fatal`
        );
      } else {
        const text = await healResp.text();
        console.warn(
          `  [?] ${humanId}: identity heal HTTP ${healResp.status} — ${text.slice(0, 200)}; non-fatal`
        );
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.warn(`  [?] ${humanId}: identity heal failed (${msg}); non-fatal`);
    }

    for (const reach of reaches) {
      const result = await seedProvideForHuman(client, humanId, agentPubKey, reach);
      if (result === 'created') created += 1;
      else if (result === 'exists') alreadyExisted += 1;
      else if (result === 'failed') failed += 1;
      // skipped handled at humanId level above
    }
  }

  console.log();
  console.log('='.repeat(60));
  console.log('Seed Provide Rows Summary');
  console.log(`  created=${created} exists=${alreadyExisted} skipped=${skipped} failed=${failed}`);
  console.log('='.repeat(60));

  const seeded = created + alreadyExisted;

  if (seeded === 0 && skipped > 0 && failed === 0) {
    // All humans skipped (no conductor session) — soft exit so the pipeline
    // continues; the card stays dark until conductor seeding runs.
    console.log(
      'INFO: All humans skipped (no conductor sessions). ' +
        'The card will light once Seed Conductor Identities runs and pods have sessions.'
    );
    process.exit(0);
  }

  if (seeded === 0) {
    console.error('ERROR: Zero provide rows seeded.');
    process.exit(1);
  }

  if (failed > 0) {
    console.warn(`WARN: ${failed} failures — partial seed.`);
    process.exit(2);
  }

  process.exit(0);
}

const isMain = import.meta.url === `file://${process.argv[1]}`;
if (isMain) {
  main().catch(err => {
    console.error('Fatal:', err);
    process.exit(1);
  });
}
