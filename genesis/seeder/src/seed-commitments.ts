/**
 * Seed REA Custody-Blob Commitments
 *
 * Writes agent-CID-keyed custody-blob commitments via the doorway REST POST
 * /api/v1/commitments handler. The shape exactly matches what
 * the custody-facing fold expects:
 *   action = "custody-blob"
 *   provider, receiver = Holochain agent keys (uhCAk…)
 *   resource_classified_as = "sha256-<blob_hash>"
 *   resource_quantity_value = bytes count
 *
 * Drift in any of these fields = views silently filter out the row.
 *
 * Usage:
 *   DOORWAY_URL=http://localhost:8888 npx tsx src/seed-commitments.ts
 *   DOORWAY_URL=https://doorway-alpha.elohim.host DOORWAY_API_KEY=xxx \
 *     CUSTODY_PAIRS_JSON=./pairs.json npx tsx src/seed-commitments.ts
 *
 * If CUSTODY_PAIRS_JSON is not set, falls back to the M1 default pair set
 * (matthew-desktop ↔ jessica-desktop, both directions, one blob — the
 * in-household pair; restored 2026-06-04 after the two-sweep divergent
 * rename RCA: the timothy→terrance seeder sweep and the timothy→jessica
 * Jenkinsfile sweep chose different successors, and no validation tied
 * the seeded pair to the documented story. The named-pair a2o scenario in
 * features/resilience/household-reciprocity.feature is the standing flag.)
 */

import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { DoorwayClient } from './doorway-client.js';
import { storageUrlForHuman, type Archetype, type ResolvePeerIdOptions } from './peer-id.js';

// =============================================================================
// Suspended-persona guard
// =============================================================================

/**
 * Fail-fast when a custody pair references a persona suspended in
 * deployments.json. This is the flag that was MISSING during the 2026-06-04
 * terrance-drift RCA: the seeder silently seeded custody pairs for a persona
 * whose deployment was suspended (dead data, zero complaint), letting a
 * divergent persona-rename sweep sit unnoticed for three weeks.
 * deployments.json already gates deploy + seed-humans + a2o tests; custody-pair
 * seeding now respects it too.
 *
 * Mirrors loadSuspendedNames() in seed-humans.ts. Fail-open on an unreadable
 * registry (partial availability is the steady state); fail-FAST on a readable
 * registry that suspends a referenced persona.
 */
function loadSuspendedHumanPrefixes(): Set<string> {
  try {
    const here = dirname(fileURLToPath(import.meta.url));
    const jsonPath = resolve(here, '../../orchestrator/data/deployments.json');
    const raw = readFileSync(jsonPath, 'utf-8');
    const data = JSON.parse(raw) as {
      humans?: { name: string; suspended?: boolean }[];
    };
    const suspended = new Set<string>();
    for (const h of data.humans ?? []) {
      if (h.suspended) suspended.add(`human-${h.name.toLowerCase()}`);
    }
    return suspended;
  } catch {
    return new Set();
  }
}

/** Exit 1 when any pair references a suspended persona (exported for tests). */
export function assertPairsNotSuspended(pairs: CustodyPair[], suspended: Set<string>): string[] {
  const offenders = [
    ...new Set(
      pairs
        .flatMap((p) => [p.providerHumanId, p.receiverHumanId])
        .filter((id) => [...suspended].some((prefix) => id.startsWith(prefix))),
    ),
  ];
  if (offenders.length > 0) {
    console.error(
      `ERROR: custody pair(s) reference SUSPENDED persona(s): ${offenders.join(', ')}.\n` +
        `deployments.json suspends them — seeding custody for non-deployed peers is dead data.\n` +
        `Fix the pair set (or un-suspend the persona) before seeding.`,
    );
    process.exit(1);
  }
  return offenders;
}

// =============================================================================
// Types
// =============================================================================

export interface CustodyPair {
  providerHumanId: string;
  providerArchetype: Archetype;
  receiverHumanId: string;
  receiverArchetype: Archetype;
  blobHash: string; // raw hex (no prefix), or full "sha256-<hex>"
  blobSizeBytes: number;
  /** Formation provenance marker — present only on interim fixtures. */
  fixture?: string;
}

interface CommitmentBody {
  id: string;
  action: 'custody-blob';
  provider: string;
  receiver: string;
  resourceConformsTo: 'blob';
  /** ValueFlows resourceClassifiedAs is a list — matches CreateReaCommitmentInputView. */
  resourceClassifiedAs: string[];
  resourceQuantity: { hasNumericalValue: number; hasUnit: 'B' };
  note: string;
  metadata: Record<string, unknown>;
}

// =============================================================================
// Body builder (testable in isolation)
// =============================================================================

function normalizeBlobHash(input: string): string {
  return input.startsWith('sha256-') ? input : `sha256-${input}`;
}

/** Resolved provider/receiver agent-CID pair for one custody commitment. */
export interface CustodyPeerIds {
  provider: string;
  receiver: string;
}

/**
 * Resolve the Holochain agent CIDs for a custody pair from each human's
 * storage pod. shard_locations.peer_id is agent-CID keyed, so authoring a
 * transport peer ID here leaves an active commitment unjoinable by the
 * custody-facing fold. Unlike the older transport resolver, this has no
 * deterministic fallback: an unavailable local session is an honest seed
 * failure, not authority to mint a row in the wrong identity namespace.
 */
export async function resolveCustodyPeerIds(
  pair: CustodyPair,
  opts: ResolvePeerIdOptions = {},
): Promise<CustodyPeerIds> {
  const fetchImpl = opts.fetchImpl ?? fetch;
  const resolveAgentCid = async (humanId: string): Promise<string> => {
    const storageUrl = opts.storageUrl ?? storageUrlForHuman(humanId);
    const response = await fetchImpl(`${storageUrl.replace(/\/+$/, '')}/auth/me`, {
      signal: AbortSignal.timeout(opts.timeoutMs ?? 5000),
    });
    if (!response.ok) {
      throw new Error(`${humanId}: /auth/me returned HTTP ${response.status}`);
    }
    const body = (await response.json()) as { agentPubKey?: unknown };
    if (typeof body.agentPubKey !== 'string' || !body.agentPubKey.startsWith('uhCAk')) {
      throw new Error(`${humanId}: /auth/me did not return a Holochain agentPubKey`);
    }
    return body.agentPubKey;
  };

  return {
    provider: await resolveAgentCid(pair.providerHumanId),
    receiver: await resolveAgentCid(pair.receiverHumanId),
  };
}

/**
 * Build a CreateReaCommitmentInputView body for a single custody-blob pair.
 *
 * `id` is a deterministic content-addressed hash of the (provider_peer,
 * receiver_peer, blob_hash) tuple — re-runs produce identical ids and the
 * doorway POST handler returns 409 (idempotent). Distinct tuples produce
 * distinct ids so genuine shape drift surfaces as 400 (fail-fast).
 *
 * `peerIds` must be the resolved Holochain agent CIDs from
 * `resolveCustodyPeerIds`. Making this explicit prevents callers from
 * accidentally reviving the transport-peer-ID namespace that cannot join
 * `shard_locations`.
 */
export function buildCustodyCommitmentBody(pair: CustodyPair, peerIds: CustodyPeerIds): CommitmentBody {
  const providerPeerId = peerIds.provider;
  const receiverPeerId = peerIds.receiver;
  const blob = normalizeBlobHash(pair.blobHash);

  const idDigest = createHash('sha256')
    .update(`${providerPeerId}|${receiverPeerId}|${blob}`, 'utf8')
    .digest('hex')
    .slice(0, 16);

  return {
    id: `custody-blob-${idDigest}`,
    action: 'custody-blob',
    provider: providerPeerId,
    receiver: receiverPeerId,
    resourceConformsTo: 'blob',
    resourceClassifiedAs: [blob],
    resourceQuantity: { hasNumericalValue: pair.blobSizeBytes, hasUnit: 'B' },
    note: `${pair.providerHumanId} commits to host ${blob} for ${pair.receiverHumanId}`,
    metadata: {
      seedGeneration: 'genesis',
      blobHash: blob,
      providerHumanId: pair.providerHumanId,
      receiverHumanId: pair.receiverHumanId,
      ...(pair.fixture ? { fixture: pair.fixture, retireAt: 'ceremony-landing' } : {}),
    },
  };
}

// =============================================================================
// Default M1 pair set
// =============================================================================

/**
 * Build the default M1 custody pair set (reads env at call time so tests can
 * set process.env before invoking — the function is only called from main or
 * from tests, never at module initialisation).
 */
export function defaultCustodyPairs(): CustodyPair[] {
  const blobHash = process.env.CONTENT_BLOB_HASH || '';
  const blobSizeBytes = parseInt(process.env.CONTENT_BLOB_SIZE_BYTES || '0', 10);

  if (!blobHash || blobSizeBytes <= 0) {
    console.error('ERROR: CONTENT_BLOB_HASH and CONTENT_BLOB_SIZE_BYTES must be set (or pass CUSTODY_PAIRS_JSON).');
    process.exit(1);
  }

  const m1 = { blobHash, blobSizeBytes };
  const fixture = { ...m1, fixture: 'formation-output' as const };

  return [
    // M1 named-pair flag (anti-drift, stays after fixture retirement)
    {
      providerHumanId: 'human-matthew-manager',
      providerArchetype: 'desktop',
      receiverHumanId: 'human-jessica-spouse',
      receiverArchetype: 'desktop',
      ...m1,
    },
    {
      providerHumanId: 'human-jessica-spouse',
      providerArchetype: 'desktop',
      receiverHumanId: 'human-matthew-manager',
      receiverArchetype: 'desktop',
      ...m1,
    },
    // INTERIM FIXTURES (2026-06-04 fork: emergent + marked fixtures) — retired at
    // ceremony landing (stage-1 plan Task 10). Loud provenance via metadata.
    {
      providerHumanId: 'human-matthew-manager',
      providerArchetype: 'desktop',
      receiverHumanId: 'human-james-son',
      receiverArchetype: 'mobile',
      ...fixture,
    },
    {
      providerHumanId: 'human-james-son',
      providerArchetype: 'mobile',
      receiverHumanId: 'human-matthew-manager',
      receiverArchetype: 'desktop',
      ...fixture,
    },
    {
      providerHumanId: 'human-jessica-spouse',
      providerArchetype: 'desktop',
      receiverHumanId: 'human-james-son',
      receiverArchetype: 'mobile',
      ...fixture,
    },
    {
      providerHumanId: 'human-james-son',
      providerArchetype: 'mobile',
      receiverHumanId: 'human-jessica-spouse',
      receiverArchetype: 'desktop',
      ...fixture,
    },
  ];
}

// =============================================================================
// Client
// =============================================================================

export class CommitmentClient extends DoorwayClient {
  async createCommitment(body: CommitmentBody): Promise<Response> {
    return this.fetch('/api/v1/commitments', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
  }

  /** Read one commitment by id (200 with the row, or 404). A pure read — used
   * to recognize an already-active row before touching the write path. */
  async getCommitment(id: string): Promise<Response> {
    return this.fetch(`/api/v1/commitments/${id}`, { method: 'GET' });
  }

  async patchCommitmentState(id: string, state: string, metadata?: Record<string, unknown>): Promise<Response> {
    return this.fetch(`/api/v1/commitments/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      // metadata (when given) reconciles rows created before the create-path
      // wire fix persisted it — idempotent heal on every re-seed.
      body: JSON.stringify(metadata ? { state, metadata } : { state }),
    });
  }
}

// =============================================================================
// Seeding (fail-fast on non-409 errors)
// =============================================================================

export async function seedCustodyCommitments(client: CommitmentClient, pairs: CustodyPair[]): Promise<void> {
  console.log(`[seed-commitments] Seeding ${pairs.length} custody-blob commitments...`);

  let created = 0;
  let alreadyExists = 0;

  for (const pair of pairs) {
    // Real peer ids from the live storage pods — per-host cached, so the
    // activate phase below recomputes the SAME content-addressed body.id.
    const body = buildCustodyCommitmentBody(pair, await resolveCustodyPeerIds(pair));
    const label = `${pair.providerHumanId.replace(/^human-/, '')}→${pair.receiverHumanId.replace(/^human-/, '')}`;

    const response = await client.createCommitment(body);

    if (response.ok) {
      console.log(`  [+] ${label} ${pair.blobHash.slice(0, 16)}...`);
      created += 1;
      continue;
    }

    const text = await response.text();
    if (response.status === 409 || text.includes('UNIQUE') || text.includes('already exists')) {
      console.log(`  [=] ${label} (idempotent re-run)`);
      alreadyExists += 1;
      continue;
    }

    // ANY other failure is a shape mismatch or doorway issue — fail fast.
    console.error(`  [X] ${label}: HTTP ${response.status}`);
    console.error(`      Body: ${text.slice(0, 500)}`);
    console.error(`      Sent: ${JSON.stringify(body, null, 2)}`);
    process.exit(1);
  }

  console.log(`[seed-commitments] Done. created=${created} already-exists=${alreadyExists} total=${pairs.length}`);

  await activateCustodyCommitments(client, pairs);
}

/**
 * Idempotency rule for the activation phase, given a commitment's currently
 * stored state (or `null` when GET returned 404).
 *
 * Re-submitting a commitment that is ALREADY in the target state is a
 * meaningful success, not a redundant write: a PATCH active→active still routes
 * through the conductor/projection write path (`project-epr` is unconditional)
 * and gets SHED under seed back-pressure with 503 {"status":"catching-up"}. The
 * genesis "Seed Custody Commitments" stage went Unstable on exactly this — a
 * reseed re-activating rows a prior run had already activated, exhausting the
 * catching-up retry budget on no-op writes. Recognizing "already active" from a
 * plain read keeps the row green without touching the shed path.
 *
 * `''`/unknown states fall through to `activate` (fail toward the write, never
 * silently skip a row that isn't demonstrably active).
 */
export function activationDecision(currentState: string | null | undefined): 'skip-active' | 'activate' | 'missing' {
  if (currentState == null) return 'missing';
  return currentState === 'active' ? 'skip-active' : 'activate';
}

/**
 * Transition every seeded custody-blob commitment to `active`, idempotently.
 *
 * Why activation at all: POST inserts `state = "proposed"` unconditionally
 * (storage lifecycle discipline — `db/rea_commitments.rs` `create_commitment`),
 * but custody is only live once active: the a2o reciprocity scenarios (and any
 * real reader) list `?action=custody-blob&state=active`. A seed row that never
 * activates satisfies the insert but is silently filtered by the view predicate
 * — the seed-row-shape lesson (history/2026-06-02-seed-row-shape-satisfies-
 * view-sql-predicates.md).
 *
 * The idempotency: read each row FIRST and only PATCH the ones that are still
 * `proposed`. An already-`active` row (a re-seed, the common case on a live
 * alpha) is recognized and skipped — the redundant PATCH that used to get shed
 * as 503 catching-up never happens (see `activationDecision`).
 *
 * `opts` is forwarded to the peer-id resolver so the flow is testable offline;
 * the live seed path passes none (real pod probes via global fetch).
 */
export async function activateCustodyCommitments(
  client: CommitmentClient,
  pairs: CustodyPair[],
  opts: ResolvePeerIdOptions = {},
): Promise<void> {
  console.log(`[seed-commitments] Activating ${pairs.length} custody-blob commitments...`);

  let activated = 0;
  let alreadyActive = 0;
  let missing = 0;

  for (const pair of pairs) {
    // Same resolver as the seed phase — the per-host cache (peer-id.ts)
    // guarantees an identical id even if a pod flapped between phases.
    const body = buildCustodyCommitmentBody(pair, await resolveCustodyPeerIds(pair, opts));
    const label = `${pair.providerHumanId.replace(/^human-/, '')}→${pair.receiverHumanId.replace(/^human-/, '')}`;

    // Read-first idempotency check: recognize the same commitment already active
    // and treat that as a valid success — no redundant, sheddable write.
    const getResp = await client.getCommitment(body.id);
    const decision =
      getResp.status === 404
        ? 'missing'
        : getResp.ok
          ? activationDecision(((await getResp.json()) as { state?: string }).state ?? null)
          : 'activate'; // non-404 read failure: fall through to the write (which retries/reports)

    if (decision === 'missing') {
      // The POST for this pair never landed — loud but non-fatal so one
      // missing row doesn't mask the activation of the rest.
      console.warn(`  [?] ${label}: commitment ${body.id} not found — POST never landed?`);
      missing += 1;
      continue;
    }

    if (decision === 'skip-active') {
      console.log(`  [=] ${label}: already active (idempotent — no write)`);
      alreadyActive += 1;
      continue;
    }

    const response = await client.patchCommitmentState(body.id, 'active', body.metadata);

    if (response.ok) {
      console.log(`  [+] ${label}: activated`);
      activated += 1;
      continue;
    }

    if (response.status === 404) {
      console.warn(`  [?] ${label}: commitment ${body.id} not found — POST never landed?`);
      missing += 1;
      continue;
    }

    // ANY other failure is a shape mismatch or doorway issue — fail fast.
    const text = await response.text();
    console.error(`  [X] ${label}: PATCH state=active HTTP ${response.status}`);
    console.error(`      Body: ${text.slice(0, 500)}`);
    process.exit(1);
  }

  console.log(
    `[seed-commitments] Activation done. activated=${activated} already-active=${alreadyActive} missing=${missing} total=${pairs.length}`,
  );
}

// =============================================================================
// Standalone execution
// =============================================================================

const isMain = import.meta.url === `file://${process.argv[1]}`;
if (isMain) {
  const doorwayUrl = process.env.DOORWAY_URL || 'http://localhost:8888';
  const apiKey = process.env.DOORWAY_API_KEY;

  const pairsJsonPath = process.env.CUSTODY_PAIRS_JSON;
  const pairs: CustodyPair[] = pairsJsonPath
    ? (JSON.parse(readFileSync(pairsJsonPath, 'utf-8')) as CustodyPair[])
    : defaultCustodyPairs();

  // The terrance-drift flag: never seed custody for suspended personas.
  assertPairsNotSuspended(pairs, loadSuspendedHumanPrefixes());

  const client = new CommitmentClient({ baseUrl: doorwayUrl, apiKey });

  console.log('='.repeat(60));
  console.log('REA Custody-Blob Commitment Seeder');
  console.log(`  Target: ${doorwayUrl}`);
  console.log(`  Pairs:  ${pairs.length}`);
  console.log('='.repeat(60));
  console.log();

  const health = await client.checkHealth();
  if (!health.healthy) {
    console.error(`ERROR: Doorway not healthy — ${health.error}`);
    process.exit(1);
  }

  await seedCustodyCommitments(client, pairs);
  process.exit(0);
}
