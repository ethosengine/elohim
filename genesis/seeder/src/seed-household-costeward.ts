/**
 * Seed the Household Co-Steward Agreement (Act I Prologue leg)
 *
 * The Act I saga's last two chapters read ONE surface —
 * `GET /api/v1/resilience/<commons EPR>/household`
 * (`elohim-storage/src/services/household_resilience.rs::snapshot_with_staleness_secs`):
 *
 *   ch09 (`concern:saga-09-projectors-carry`)
 *     commitmentBackedReplication.commonsCommitments >= 1
 *     commitmentBackedReplication.totalPledgedBytes   >= 1
 *   ch10 (`concern:saga-10-card-tells-truth`)
 *     stewardingCollectives is the SAME non-zero value on both doorways
 *
 * On alpha the co-steward is adam and the agreement is authored at RUN time by
 * chapter 5's provide-pin → notarize → mirror chain, for chapter 5's own e2e
 * content. On the household mesh nothing plays that part, so Act I must cast
 * its own co-steward: **jessica**, co-stewarding matthew's landing EPR.
 *
 * ## The two folds, and what each one actually joins on
 *
 * Both counts are scoped by the SAME set — `steward_households`, the distinct
 * non-null `humans.household_id` over
 *
 *     shard_locations ⋈ humans ON humans.agent_pub_key = shard_locations.peer_id
 *
 * for the shard hashes named by the content's `shard_manifests` row. So:
 *
 * - **No manifest ⇒ empty relation ⇒ BOTH counts are zero**, regardless of how
 *   many commitments exist. `distributionState: "unmeasured"` is the tell, and
 *   it is an honest non-measurement, not a measured zero.
 * - A `replicates-commons` commitment only counts when its `provider` joins a
 *   human whose `household_id` is IN that steward set
 *   (`elohim_facings::folds::replication_commitment::commitment_backed_replication_for_households`)
 *   — a commitment from a household that holds none of the bytes is deliberately
 *   NOT attributed.
 * - `totalPledgedBytes` comes from the commitment payload's `commons_bytes`
 *   (the `capacity` variant). The `content` variant names a head_ref and
 *   pledges no bytes, so the two legs below are BOTH needed — one for the
 *   count, one for the budget.
 *
 * Which is why this seeder runs three legs in that order, and why leg 1 is a
 * hard precondition for legs 2-3 counting at all.
 *
 * ## Leg 0 — the household consent pin (ch05 station A)
 *
 * Runs BEFORE the ladder because it is the CONSENT the whole downstream
 * pipeline (provide tick → mishpat notarization → rea mirror) hangs off:
 * saga ch05 station A reads `GET /api/v1/pins` on doorway alpha-A for an
 * active item pin whose head references the commons EPR, and ch11's first
 * scenario reads `GET /api/v1/pins/<id>/pull` — pull state only materializes
 * from an active pin. `POST /api/v1/pins` is the explicit stewardship-
 * acceptance act (`{headRef, kind:"item", provide:true}` — the same own-node
 * surface acquisition-pins.feature exercises); the handler upserts on
 * (agent, head_ref, kind), so a re-run is a no-op by construction, and this
 * leg additionally GETs-before-POSTs so an existing active pin is reported
 * `[=]` rather than re-written. The pin is device-keyed on the wire (the
 * create handler's slice-1 identity placeholder), so the household member's
 * live agent key is resolved as a WITNESS of "this is household-dowell's
 * node", never sent in the body — no identity is fabricated.
 *
 * ## Leg 1 — distribution ladder (the ch10 enabler)
 *
 * Tried in honesty order, first rung that answers wins:
 *
 *   (a) already measured with >= 1 stewarding collective → nothing to do
 *       (idempotent re-run, the common case).
 *   (b) `POST /db/content/bulk` re-declaring the SAME row with its existing
 *       `blobHash`. The bulk handler builds its `distribution_items` from the
 *       INPUT views before the "row exists → skip" branch, so an existing row
 *       is not touched (verified live: `{"inserted":0,"skipped":1}`) while the
 *       shard-manifest recorder and `distribute_shards` still run. This is the
 *       GENUINE path — real bytes, real self-held evidence — and it is the rung
 *       that should normally answer.
 *   (c) `PUT /admin/seed/shard-manifest` (honesty-gated behind
 *       `ALLOW_SEED_SHARD_MANIFEST=1`), one steward agent key per distinct
 *       household. Mirrors a2o's own `ensureStewardedByHouseholds` helper in
 *       `genesis/a2o/steps/resilience.steps.ts`. A 403 is a REFUSAL to
 *       fabricate distribution, not a failure — reported, never worked around.
 *
 * KNOWN CEILING of rung (b), measured on the household mesh 2026-08-21: every
 * manifest producer (`POST /db/content`, `/db/content/bulk`, `distribute_shards`,
 * `services::shard_manifest_backfill`) reads the blob via `blob_store.get(blob_hash)`
 * — the COMPOSITE object — but `PUT /admin/seed/blob` stores a blob larger than
 * `sharding::SINGLE_SHARD_MAX` as `encoding: "chunked"` shards with NO composite
 * under the blob hash. Only the serve path (`read_local_blob`) understands both
 * shapes. So for an app bundle above that threshold (the landing browser bundle
 * is ~18 MB / 18 shards) rung (b) runs, logs "Bulk shard distribution complete",
 * and silently records nothing. On such a mesh rung (c) is the only lever, and
 * when it is disabled this seeder reports the blocker rather than inventing one.
 *
 * ## Leg 2 — the co-steward agreement (ch09 count)
 *
 * A `replicates-commons` commitment, `content` variant: jessica as provider,
 * the commons EPR id as receiver (matching
 * `mishpat_projection::parse_replicates_commons` → `recipient: head_ref`),
 * `resourceClassifiedAs: ["content:<reach>"]` so it ALSO satisfies the
 * separate `commitment_backed_collectives` scope test, and `inScopeOf` the
 * household. Idempotent by GET-before-POST on a content-addressed id, then
 * PATCH `state=active` (POST inserts `proposed` unconditionally — the
 * seed-row-shape lesson).
 *
 * IDENTITY RULE (the defect this guards): the provider MUST be the agent key
 * the co-steward's pod reports at `GET /auth/me` RIGHT NOW, never the
 * `agentPubKey` cached on her `humans` row. A mesh whose `content.db` outlives
 * its conductor keeps a STALE key on the canonical `human-*` row while the live
 * key lands on a separate `agent:<key>` row — measured on the household mesh
 * 2026-08-21 for jessica and james. `shard_locations.peer_id` carries the LIVE
 * key, so a commitment authored against the cached one joins nothing.
 *
 * ## Leg 3 — the commons byte budget (ch09 bytes)
 *
 * `POST /api/v1/commitments/capacity` through the CO-STEWARD's doorway, so the
 * pledge's provider (injected server-side from that node's own conductor cell)
 * is the co-steward's agent — the same identity leg 2 named. Reuses
 * `seedCapacityPledge` from seed-commitments.ts verbatim; that seeder already
 * pledges through doorway A (matthew).
 *
 * Usage:
 *   DOORWAY_URL=http://localhost:8888 DOORWAY_B_URL=http://localhost:8889 \
 *     PEER_STORAGE_URLS=matthew=localhost:8090,jessica=localhost:8091 \
 *     npx tsx src/seed-household-costeward.ts
 *
 * Exit codes mirror the Jenkinsfile `runProbedSeeder` contract: 0 = clean,
 * 2 = partial (a leg was honestly blocked), 1 = total failure.
 */

import { createHash } from 'node:crypto';
import { CommitmentClient, seedCapacityPledge } from './seed-commitments.js';
import { parseNamedCsv, storageUrlForHuman } from './peer-id.js';

// =============================================================================
// Configuration
// =============================================================================

/** The commons EPR the Act I saga's last chapters read. */
export const DEFAULT_COMMONS_EPR_ID = 'elohim-host-landing';

/** The household whose members cast the Act I co-steward story. */
export const DEFAULT_HOUSEHOLD_ID = 'household-dowell';

/** The Act I co-steward — jessica co-stewards matthew's landing EPR. */
export const DEFAULT_CO_STEWARD_HUMAN_ID = 'human-jessica-spouse';

/**
 * The household member whose node declares the leg-0 consent pin — matthew,
 * because doorway alpha-A (the surface ch05 station A reads) proxies HIS
 * storage's pins list.
 */
export const DEFAULT_PIN_STEWARD_HUMAN_ID = 'human-matthew-manager';

/**
 * Bytes the co-steward's `content`-variant pledge records for legibility.
 * NOT the ch09 `totalPledgedBytes` source — that comes from leg 3's capacity
 * pledge, whose `commons_bytes` the replication fold reads. This value is the
 * measured size of the bytes actually being co-stewarded, resolved from the
 * content row when it declares one and omitted otherwise ("absent beats
 * fabricated").
 */
export type ResolvedBytes = number | undefined;

// =============================================================================
// Wire shapes
// =============================================================================

/** `CreateReaCommitmentInputView` subset this seeder authors (camelCase wire). */
export interface CoStewardCommitmentBody {
  id: string;
  action: 'replicates-commons';
  provider: string;
  receiver: string;
  resourceConformsTo: 'epr';
  resourceClassifiedAs: string[];
  inScopeOf: string[];
  note: string;
  metadata: Record<string, unknown>;
}

/** The fields of the household resilience card this seeder measures against. */
export interface HouseholdCard {
  distributionState?: string;
  stewardingCollectives?: number;
  commitmentBackedReplication?: {
    commonsCommitments?: number;
    totalPledgedBytes?: number;
  };
  details?: {
    stewardingCollectives?: { id: string; kind: string }[];
  };
}

// =============================================================================
// Pure helpers (unit-tested without a mesh)
// =============================================================================

/**
 * Build the co-steward `replicates-commons` commitment body.
 *
 * `id` is a content-addressed digest of (provider, receiver, scope) so re-runs
 * produce the SAME id — the GET-before-POST check below then recognizes the
 * existing row instead of minting a duplicate. A genuinely different co-steward
 * or a different EPR yields a different id, so real drift surfaces as a new row
 * rather than hiding inside an old one.
 */
export function buildCoStewardCommitmentBody(input: {
  providerAgentKey: string;
  contentId: string;
  reach: string;
  householdId: string;
  coStewardHumanId: string;
  bytes?: ResolvedBytes;
}): CoStewardCommitmentBody {
  const { providerAgentKey, contentId, reach, householdId, coStewardHumanId, bytes } = input;
  const scope = `content:${reach}`;
  const digest = createHash('sha256')
    .update(`${providerAgentKey}|${contentId}|${scope}`, 'utf8')
    .digest('hex')
    .slice(0, 16);

  return {
    id: `replicates-commons-${digest}`,
    action: 'replicates-commons',
    provider: providerAgentKey,
    // The commons CONTENT variant's counterparty IS the EPR head_ref — the same
    // shape mishpat_projection::parse_replicates_commons mirrors into
    // rea_commitments.receiver. `commitment_refs_covering` scopes per-CID on
    // exactly this field, so naming the EPR here is what makes the pledge
    // legible as covering THIS content rather than a bare scope.
    receiver: contentId,
    resourceConformsTo: 'epr',
    // A JSON list by contract (non-commons spec §11.2, Option A) — the
    // `commitment_backed_collectives` scope test membership-checks this list in
    // Rust, so a scalar would be silently missed.
    resourceClassifiedAs: [scope],
    inScopeOf: [householdId],
    note: `${coStewardHumanId} co-stewards ${contentId} for the ${householdId} commons`,
    metadata: {
      seedGeneration: 'genesis',
      variant: 'content',
      // snake_case AND camelCase are both read by the fold's payload_* helpers;
      // head_ref is the DHT payload spelling parse_replicates_commons uses.
      head_ref: contentId,
      coStewardHumanId,
      householdId,
      ...(typeof bytes === 'number' && bytes > 0 ? { coStewardedBytes: bytes } : {}),
    },
  };
}

/**
 * Which rung of the distribution ladder this card calls for.
 *
 * `measured` alone is NOT enough: a manifest with no household-joinable
 * `shard_locations` still folds to zero stewards (that is exactly the shape a
 * mesh whose conductor identities were regenerated leaves behind — the
 * locations name the PREVIOUS generation's agent keys). Only a non-zero
 * steward count is "done".
 */
export function distributionLadderRung(card: HouseholdCard): 'satisfied' | 'redistribute' {
  const stewards = card.stewardingCollectives ?? 0;
  return stewards >= 1 ? 'satisfied' : 'redistribute';
}

/**
 * One steward agent key per distinct household, for the seed-shard-manifest
 * lever's `stewards` list. Mirrors a2o's `ensureStewardedByHouseholds`: a
 * household is represented once, so the lever cannot inflate the fault-domain
 * count by naming two members of the same home.
 *
 * `liveKeys` (human id → the key that human's pod reports NOW) overrides the
 * `agentPubKey` cached on the row — see the IDENTITY RULE in the module docs.
 */
export function stewardAgentKeysByHousehold(
  humans: { id?: string; agentPubKey?: string | null; householdId?: string | null }[],
  liveKeys: Map<string, string> = new Map(),
): string[] {
  const byHousehold = new Map<string, string>();
  for (const human of humans) {
    const household = human.householdId ?? undefined;
    const key = (human.id ? liveKeys.get(human.id) : undefined) ?? human.agentPubKey ?? undefined;
    if (household && key && !byHousehold.has(household)) {
      byHousehold.set(household, key);
    }
  }
  return [...byHousehold.values()];
}

/**
 * Idempotency rule for the activation phase — identical to
 * seed-commitments.ts's `activationDecision`, restated here so this leg does
 * not depend on that module's internals. An already-`active` row is recognized
 * from a plain READ and never re-PATCHed: a redundant active→active PATCH still
 * routes through the conductor/projection write path and gets shed under seed
 * back-pressure with 503 {"status":"catching-up"}.
 */
export function activationDecision(
  currentState: string | null | undefined,
): 'skip-active' | 'activate' | 'missing' {
  if (currentState == null) return 'missing';
  return currentState === 'active' ? 'skip-active' : 'activate';
}

// =============================================================================
// Live-mesh probes
// =============================================================================

/** Every peer storage base URL declared in PEER_STORAGE_URLS. */
function peerStorageUrlsFromEnv(): string[] {
  return parseNamedCsv(process.env.PEER_STORAGE_URLS ?? '').map(entry =>
    (entry.value.includes('://') ? entry.value : `http://${entry.value}`).replace(/\/+$/, ''),
  );
}

/** Storage base URL for a human, honoring PEER_STORAGE_URLS. */
function storageBase(humanId: string): string {
  return storageUrlForHuman(humanId).replace(/\/+$/, '');
}

/**
 * Retry a transiently-failing probe a few times with a short backoff.
 *
 * Exists because conductor-backed key resolution (`GET /auth/me`) was seen to
 * time out transiently on the household mesh (wave-2b) while the pod was
 * otherwise healthy. Three attempts, short backoff — this is a flake
 * absorber, never a wait-for-deploy loop.
 */
export async function withRetry<T>(
  fn: () => Promise<T>,
  attempts = 3,
  backoffMs = 1500,
): Promise<T> {
  let lastError: unknown;
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      return await fn();
    } catch (error) {
      lastError = error;
      if (attempt < attempts) {
        await new Promise(resolve => setTimeout(resolve, backoffMs * attempt));
      }
    }
  }
  throw lastError;
}

/**
 * The agent key a human's pod reports RIGHT NOW.
 *
 * No fallback to the `humans` row on purpose (same posture as
 * seed-commitments' `resolveCustodyPeerIds`): an unreachable pod is an honest
 * seed failure, never authority to author a commitment in a stale identity
 * namespace.
 *
 * `storageUrlOverride` bypasses the PEER_STORAGE_URLS/template lookup when the
 * caller already knows the pod (leg 0 probes the SAME node it pins on).
 */
export async function resolveLiveAgentKey(
  humanId: string,
  fetchImpl: typeof fetch = fetch,
  storageUrlOverride?: string,
): Promise<string> {
  const base = (storageUrlOverride ?? storageBase(humanId)).replace(/\/+$/, '');
  const url = `${base}/auth/me`;
  const response = await fetchImpl(url, { signal: AbortSignal.timeout(5000) });
  if (!response.ok) {
    throw new Error(`${humanId}: GET ${url} returned HTTP ${response.status}`);
  }
  const body = (await response.json()) as { agentPubKey?: unknown };
  if (typeof body.agentPubKey !== 'string' || !body.agentPubKey.startsWith('uhCAk')) {
    throw new Error(`${humanId}: ${url} did not return a Holochain agentPubKey`);
  }
  return body.agentPubKey;
}

/** Read the household resilience card through a doorway. */
async function readCard(
  doorwayUrl: string,
  contentId: string,
  fetchImpl: typeof fetch = fetch,
): Promise<HouseholdCard> {
  const url = `${doorwayUrl.replace(/\/+$/, '')}/api/v1/resilience/${encodeURIComponent(contentId)}/household`;
  const response = await fetchImpl(url, { signal: AbortSignal.timeout(10_000) });
  if (!response.ok) {
    throw new Error(`GET ${url} returned HTTP ${response.status}`);
  }
  return (await response.json()) as HouseholdCard;
}

/** Read one content row (the source of the blobHash + reach leg 1 re-declares). */
async function readContentRow(
  doorwayUrl: string,
  contentId: string,
  fetchImpl: typeof fetch = fetch,
): Promise<Record<string, unknown>> {
  const url = `${doorwayUrl.replace(/\/+$/, '')}/db/content/${encodeURIComponent(contentId)}`;
  const response = await fetchImpl(url, { signal: AbortSignal.timeout(10_000) });
  if (!response.ok) {
    throw new Error(`GET ${url} returned HTTP ${response.status}`);
  }
  return (await response.json()) as Record<string, unknown>;
}

// =============================================================================
// Leg 0 — household consent pin (ch05 station A)
// =============================================================================

export interface ConsentPinLegResult {
  status: 'already-pinned' | 'created' | 'failed';
  detail: string;
}

/**
 * Ensure the household's stewardship consent pin exists on the pin node's own
 * `/api/v1/pins` surface — the explicit-consent leg saga ch05 station A reads.
 *
 * GET-before-POST: an existing ACTIVE item pin whose headRef matches either
 * spelling (`elohim-host-landing` / `epr:elohim-host-landing` — the namespace
 * rule station A guards) is recognized and left alone. Otherwise POST
 * `{headRef, kind:"item", provide:true}`; the handler upserts on
 * (agent, head_ref, kind), so even a raced double-create converges on one row.
 * The bare spelling is posted deliberately: current handlers strip `epr:` at
 * the boundary, and on older binaries a bare headRef still groups correctly
 * on the `/pull` route's exact-match lookup.
 */
export async function seedConsentPin(opts: {
  storageUrl: string;
  contentId: string;
  fetchImpl?: typeof fetch;
}): Promise<ConsentPinLegResult> {
  const fetchImpl = opts.fetchImpl ?? fetch;
  const base = opts.storageUrl.replace(/\/+$/, '');
  const accepted = new Set([opts.contentId, `epr:${opts.contentId}`]);

  const list = await fetchImpl(`${base}/api/v1/pins`, { signal: AbortSignal.timeout(10_000) });
  if (!list.ok) {
    return {
      status: 'failed',
      detail: `GET ${base}/api/v1/pins → HTTP ${list.status}`,
    };
  }
  const pins = (((await list.json()) as { pins?: unknown }).pins ?? []) as Record<
    string,
    unknown
  >[];
  const existing = pins.find(
    p => accepted.has(String(p['headRef'])) && p['status'] === 'active' && p['kind'] === 'item',
  );
  if (existing) {
    return {
      status: 'already-pinned',
      detail: `pin #${String(existing['id'])} already active for ${String(existing['headRef'])} (idempotent — no write)`,
    };
  }

  const created = await fetchImpl(`${base}/api/v1/pins`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ headRef: opts.contentId, kind: 'item', provide: true }),
    signal: AbortSignal.timeout(15_000),
  });
  const text = await created.text();
  if (!created.ok) {
    return {
      status: 'failed',
      detail: `POST ${base}/api/v1/pins → HTTP ${created.status} ${text.slice(0, 200)}`,
    };
  }
  let pinId = '?';
  try {
    pinId = String((JSON.parse(text) as { id?: unknown }).id ?? '?');
  } catch {
    // 2xx with an unparseable body still created the pin; id stays unknown.
  }
  return {
    status: 'created',
    detail: `pin #${pinId} created (headRef=${opts.contentId}, kind=item, provide=true) → HTTP ${created.status}`,
  };
}

// =============================================================================
// Leg 1 — distribution ladder
// =============================================================================

export interface DistributionLegResult {
  rung: 'already-measured' | 'redistributed' | 'seed-lever' | 'blocked';
  stewardingCollectives: number;
  detail: string;
}

/**
 * Re-declare the content row through `POST /db/content/bulk` so the storage
 * node records its shard manifest and runs `distribute_shards`.
 *
 * Deliberately re-declares only the fields the bulk handler reads for
 * distribution (`id`, `blobHash`, `blobCid`, `contentFormat`, `reach`) plus the
 * NOT-NULL columns its insert path requires. The existing row is skipped, never
 * updated, so no field — notably the deploy-time `serverBlobHash`, which
 * `CreateContentInputView` cannot even express — can be clobbered by this call.
 */
async function redeclareForDistribution(
  storageUrl: string,
  row: Record<string, unknown>,
  apiKey: string | undefined,
  fetchImpl: typeof fetch = fetch,
): Promise<{ ok: boolean; detail: string }> {
  const item: Record<string, unknown> = {
    id: row['id'],
    title: row['title'] ?? String(row['id']),
    contentType: row['contentType'],
    contentFormat: row['contentFormat'],
    blobHash: row['blobHash'],
    blobCid: row['blobCid'],
    reach: row['reach'] ?? 'commons',
  };
  for (const key of Object.keys(item)) {
    if (item[key] === null || item[key] === undefined) delete item[key];
  }
  if (!item['blobHash']) {
    return { ok: false, detail: 'content row declares no blobHash — nothing to distribute' };
  }

  const response = await fetchImpl(`${storageUrl.replace(/\/+$/, '')}/db/content/bulk`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Schema-Version': '1',
      ...(apiKey ? { 'X-Api-Key': apiKey } : {}),
    },
    body: JSON.stringify([item]),
    signal: AbortSignal.timeout(30_000),
  });
  const text = await response.text();
  return {
    ok: response.ok,
    detail: `POST /db/content/bulk → HTTP ${response.status} ${text.slice(0, 160)}`,
  };
}

/** Drive the honesty-gated seed lever. 403 is a refusal, reported as such. */
async function driveSeedShardManifestLever(
  storageUrl: string,
  contentId: string,
  reach: string,
  blobHash: string,
  stewards: string[],
  totalSizeBytes: number,
  apiKey: string | undefined,
  fetchImpl: typeof fetch = fetch,
): Promise<{ ok: boolean; disabled: boolean; detail: string }> {
  const response = await fetchImpl(`${storageUrl.replace(/\/+$/, '')}/admin/seed/shard-manifest`, {
    method: 'PUT',
    headers: {
      'Content-Type': 'application/json',
      ...(apiKey ? { 'X-Api-Key': apiKey } : {}),
    },
    body: JSON.stringify({ contentId, reach, blobHash, stewards, totalSizeBytes }),
    signal: AbortSignal.timeout(30_000),
  });
  const text = await response.text();
  return {
    ok: response.ok,
    disabled: response.status === 403,
    detail: `PUT /admin/seed/shard-manifest → HTTP ${response.status} ${text.slice(0, 200)}`,
  };
}

export async function ensureDistributionMeasured(opts: {
  doorwayUrl: string;
  storageUrl: string;
  contentId: string;
  apiKey?: string;
  fetchImpl?: typeof fetch;
  settleMs?: number;
}): Promise<DistributionLegResult> {
  const fetchImpl = opts.fetchImpl ?? fetch;
  const settleMs = opts.settleMs ?? 12_000;

  let card = await readCard(opts.doorwayUrl, opts.contentId, fetchImpl);
  if (distributionLadderRung(card) === 'satisfied') {
    return {
      rung: 'already-measured',
      stewardingCollectives: card.stewardingCollectives ?? 0,
      detail: 'already measured with at least one stewarding collective (idempotent re-run)',
    };
  }

  // Rung (b) — the genuine path.
  const row = await readContentRow(opts.doorwayUrl, opts.contentId, fetchImpl);
  const redeclare = await redeclareForDistribution(opts.storageUrl, row, opts.apiKey, fetchImpl);
  console.log(`  [~] redistribute: ${redeclare.detail}`);
  if (redeclare.ok) {
    await new Promise(resolve => setTimeout(resolve, settleMs));
    card = await readCard(opts.doorwayUrl, opts.contentId, fetchImpl);
    if (distributionLadderRung(card) === 'satisfied') {
      return {
        rung: 'redistributed',
        stewardingCollectives: card.stewardingCollectives ?? 0,
        detail: 'genuine distribute_shards round recorded the manifest and self-held evidence',
      };
    }
  }

  // Rung (c) — the honesty-gated operator lever.
  const humansUrl = `${opts.doorwayUrl.replace(/\/+$/, '')}/db/humans`;
  const humansResponse = await fetchImpl(humansUrl, { signal: AbortSignal.timeout(10_000) });
  const humans = humansResponse.ok
    ? (((await humansResponse.json()) as { items?: unknown[] }).items ?? [])
    : [];
  // Live keys override the cached ones. Without this the lever would name the
  // PREVIOUS conductor generation's agent keys — a manifest that joins nothing,
  // which is the same measured zero it was called to cure (see the IDENTITY
  // RULE in the module docs).
  const liveKeys = new Map<string, string>();
  for (const url of peerStorageUrlsFromEnv()) {
    try {
      const response = await fetchImpl(`${url}/auth/me`, { signal: AbortSignal.timeout(5000) });
      if (!response.ok) continue;
      const body = (await response.json()) as { humanId?: unknown; agentPubKey?: unknown };
      if (typeof body.humanId === 'string' && typeof body.agentPubKey === 'string') {
        liveKeys.set(body.humanId, body.agentPubKey);
      }
    } catch {
      // An unreachable peer contributes no override — the cached key stands,
      // and a stale one shows up as a still-zero steward count, reported.
    }
  }
  const stewards = stewardAgentKeysByHousehold(
    humans as { id?: string; agentPubKey?: string | null; householdId?: string | null }[],
    liveKeys,
  );
  const blobHash = String(row['blobHash'] ?? '');
  const lever = await driveSeedShardManifestLever(
    opts.storageUrl,
    opts.contentId,
    String(row['reach'] ?? 'commons'),
    blobHash,
    stewards,
    Number(row['contentSizeBytes'] ?? 0) || 0,
    opts.apiKey,
    fetchImpl,
  );
  console.log(`  [~] seed lever: ${lever.detail}`);
  if (lever.ok) {
    await new Promise(resolve => setTimeout(resolve, 2000));
    card = await readCard(opts.doorwayUrl, opts.contentId, fetchImpl);
    if (distributionLadderRung(card) === 'satisfied') {
      return {
        rung: 'seed-lever',
        stewardingCollectives: card.stewardingCollectives ?? 0,
        detail: `operator seed lever recorded a manifest for ${stewards.length} household steward(s)`,
      };
    }
  }

  return {
    rung: 'blocked',
    stewardingCollectives: card.stewardingCollectives ?? 0,
    detail: lever.disabled
      ? `"${opts.contentId}" has no household-joinable shard manifest, the genuine ` +
        `distribute_shards round recorded nothing (an app bundle above ` +
        `sharding::SINGLE_SHARD_MAX is stored CHUNKED, and every manifest producer reads the ` +
        `absent COMPOSITE via blob_store.get), and the deterministic lever is disabled. ` +
        `Enable it on the storage peers with ALLOW_SEED_SHARD_MANIFEST=1, or teach the ` +
        `manifest recorder to read chunked blobs the way read_local_blob already does. ` +
        `Until then stewardingCollectives is an honest 0 — this seeder will not fabricate it.`
      : `"${opts.contentId}" still folds to ${card.stewardingCollectives ?? 0} stewarding ` +
        `collectives after both rungs. ${lever.detail}`,
  };
}

// =============================================================================
// Leg 2 — the co-steward agreement
// =============================================================================

export interface CoStewardLegResult {
  status: 'created' | 'already-active' | 'activated' | 'failed';
  commitmentId: string;
  detail: string;
}

/**
 * `CommitmentClient` widened for the `replicates-commons` action.
 *
 * Its `createCommitment` is typed to the `custody-blob` body on purpose (that
 * seeder's whole contract is one action). Subclassing rather than loosening
 * that type keeps the custody seeder's fail-fast shape guarantee intact while
 * this leg authors a different action through the SAME route.
 */
export class CoStewardClient extends CommitmentClient {
  async createCoStewardCommitment(body: CoStewardCommitmentBody): Promise<Response> {
    return this.fetch('/api/v1/commitments', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
  }
}

export async function seedCoStewardCommitment(
  client: CoStewardClient,
  body: CoStewardCommitmentBody,
): Promise<CoStewardLegResult> {
  // GET-before-POST: the create handler UPSERTS on a duplicate id and answers
  // 200/201 with the EXISTING row (it never 409s), so trusting `response.ok`
  // alone would report "created" on every re-run. Recognize the existing row
  // from a plain read first — the same posture seed-commitments.ts takes.
  const existing = await client.getCommitment(body.id);
  if (existing.ok) {
    const state = ((await existing.json()) as { state?: string }).state ?? null;
    if (activationDecision(state) === 'skip-active') {
      return {
        status: 'already-active',
        commitmentId: body.id,
        detail: 'already active (idempotent — no write)',
      };
    }
  } else if (existing.status === 404) {
    const created = await client.createCoStewardCommitment(body);
    if (!created.ok) {
      const text = await created.text();
      return {
        status: 'failed',
        commitmentId: body.id,
        detail: `POST /api/v1/commitments → HTTP ${created.status} ${text.slice(0, 300)}`,
      };
    }
  }

  // POST inserts `state = "proposed"` unconditionally (storage lifecycle
  // discipline — db/rea_commitments.rs::create_commitment). The folds filter on
  // `state = "active"`, so a row that never activates satisfies the insert and
  // is silently invisible to every reader.
  const patched = await client.patchCommitmentState(body.id, 'active', body.metadata);
  if (!patched.ok) {
    const text = await patched.text();
    return {
      status: 'failed',
      commitmentId: body.id,
      detail: `PATCH state=active → HTTP ${patched.status} ${text.slice(0, 300)}`,
    };
  }
  return {
    status: existing.ok ? 'activated' : 'created',
    commitmentId: body.id,
    detail: 'active',
  };
}

// =============================================================================
// Standalone execution
// =============================================================================

const isMain = import.meta.url === `file://${process.argv[1]}`;
if (isMain) {
  const doorwayA = (process.env.DOORWAY_URL || 'http://localhost:8888').replace(/\/+$/, '');
  const doorwayB = (process.env.DOORWAY_B_URL || doorwayA).replace(/\/+$/, '');
  const apiKey = process.env.DOORWAY_API_KEY || process.env.STORAGE_API_KEY_ADMIN;
  const contentId = process.env.COMMONS_EPR_ID || DEFAULT_COMMONS_EPR_ID;
  const householdId = process.env.HOUSEHOLD_ID || DEFAULT_HOUSEHOLD_ID;
  const coStewardHumanId = process.env.CO_STEWARD_HUMAN_ID || DEFAULT_CO_STEWARD_HUMAN_ID;

  console.log('='.repeat(64));
  console.log('Household Co-Steward Seeder (Act I Prologue)');
  console.log(`  commons EPR : ${contentId}`);
  console.log(`  co-steward  : ${coStewardHumanId} (household ${householdId})`);
  console.log(`  doorway A/B : ${doorwayA} / ${doorwayB}`);
  console.log('='.repeat(64));

  const clientA = new CoStewardClient({ baseUrl: doorwayA, apiKey });
  const clientB = new CoStewardClient({ baseUrl: doorwayB, apiKey });

  const health = await clientA.checkHealth();
  if (!health.healthy) {
    console.error(`ERROR: doorway A not healthy — ${health.error}`);
    process.exit(1);
  }

  let partial = 0;

  // --- Leg 0 -----------------------------------------------------------------
  console.log('\n-- leg 0: household consent pin (ch05 station A) --');
  const pinStewardHumanId = process.env.PIN_STEWARD_HUMAN_ID || DEFAULT_PIN_STEWARD_HUMAN_ID;
  // Doorway alpha-A reads its OWN storage's pins list, so the pin lands on the
  // pin steward's node — STORAGE_URL when set (the local-mesh spelling), else
  // the per-human resolution the other legs use.
  const pinStorageUrl = (process.env.STORAGE_URL || storageBase(pinStewardHumanId)).replace(
    /\/+$/,
    '',
  );
  // Witness the node's live member identity (this is household-dowell's
  // consent, declared from its member's device). The pin wire is device-keyed
  // (slice-1 placeholder), so the key is logged, never sent — and a transient
  // conductor key-resolution timeout (seen on wave-2b) is absorbed by retry,
  // while a persistent failure is reported without blocking the consent act.
  try {
    const pinNodeKey = await withRetry(() =>
      resolveLiveAgentKey(pinStewardHumanId, fetch, pinStorageUrl),
    );
    console.log(`  [=] ${pinStewardHumanId} live agent key: ${pinNodeKey.slice(0, 20)}...`);
  } catch (error) {
    console.warn(
      `  [?] could not witness ${pinStewardHumanId}'s live agent key at ${pinStorageUrl} — ` +
        `pin proceeds device-keyed (${String(error)})`,
    );
  }
  try {
    const pinResult = await seedConsentPin({ storageUrl: pinStorageUrl, contentId });
    const pinMark =
      pinResult.status === 'failed' ? 'X' : pinResult.status === 'already-pinned' ? '=' : '+';
    console.log(`  [${pinMark}] ${pinStorageUrl} ${pinResult.status} — ${pinResult.detail}`);
    if (pinResult.status === 'failed') partial += 1;
  } catch (error) {
    console.error(`  [X] consent pin leg failed — ${String(error)}`);
    partial += 1;
  }

  // --- Leg 1 -----------------------------------------------------------------
  console.log('\n-- leg 1: distribution ladder (ch10 enabler) --');
  // Run the ladder on EVERY peer's storage, not just the authoring one.
  // ch10 compares the card across two doorways, and each doorway reads its OWN
  // primary storage — a manifest recorded on only one of them makes the two
  // answers differ, which is the exact disagreement that chapter exists to
  // catch. Reading through storage directly (not the doorway) also keeps the
  // follow-up measurement out of the doorway's 30s route cache.
  const peerStorage = peerStorageUrlsFromEnv();
  const storageTargets =
    peerStorage.length > 0
      ? peerStorage
      : [process.env.STORAGE_URL || storageBase('human-matthew-manager')];

  const distributionResults: DistributionLegResult[] = [];
  for (const storageUrl of storageTargets) {
    let result: DistributionLegResult;
    try {
      result = await ensureDistributionMeasured({
        doorwayUrl: storageUrl,
        storageUrl,
        contentId,
        apiKey,
      });
    } catch (error) {
      result = {
        rung: 'blocked',
        stewardingCollectives: 0,
        detail: `probe failed: ${error instanceof Error ? error.message : String(error)}`,
      };
    }
    distributionResults.push(result);
    const mark = result.rung === 'blocked' ? 'X' : '+';
    console.log(`  [${mark}] ${storageUrl} ${result.rung} (stewards=${result.stewardingCollectives})`);
    if (result.rung === 'blocked') console.log(`      ${result.detail}`);
  }
  if (distributionResults.some(result => result.rung === 'blocked')) partial += 1;

  // --- Leg 2 -----------------------------------------------------------------
  console.log('\n-- leg 2: co-steward replicates-commons agreement (ch09 count) --');
  let providerAgentKey = '';
  try {
    providerAgentKey = await withRetry(() => resolveLiveAgentKey(coStewardHumanId));
    console.log(`  [=] ${coStewardHumanId} live agent key: ${providerAgentKey.slice(0, 20)}...`);
  } catch (error) {
    console.error(`  [X] cannot resolve ${coStewardHumanId}'s live agent key — ${String(error)}`);
    partial += 1;
  }

  if (providerAgentKey) {
    let reach = 'commons';
    let bytes: ResolvedBytes;
    try {
      const row = await readContentRow(doorwayA, contentId);
      reach = String(row['reach'] ?? 'commons');
      const declared = row['contentSizeBytes'];
      bytes = typeof declared === 'number' && declared > 0 ? declared : undefined;
    } catch (error) {
      console.warn(`  [?] could not read ${contentId} for reach/size — defaulting reach=commons (${String(error)})`);
    }

    const body = buildCoStewardCommitmentBody({
      providerAgentKey,
      contentId,
      reach,
      householdId,
      coStewardHumanId,
      bytes,
    });
    // Authored through BOTH doorways: the commitment projection is per-node,
    // and ch10 reads the card from each doorway's own storage. A pledge that
    // exists on only one side is precisely the asymmetry that makes two
    // doorways tell two truths.
    for (const [label, client] of [
      ['A', clientA],
      ['B', clientB],
    ] as const) {
      const legResult = await seedCoStewardCommitment(client, body);
      const mark = legResult.status === 'failed' ? 'X' : '+';
      console.log(
        `  [${mark}] doorway ${label} ${legResult.commitmentId}: ${legResult.status} — ${legResult.detail}`,
      );
      if (legResult.status === 'failed') partial += 1;
    }
  }

  // --- Leg 3 -----------------------------------------------------------------
  console.log("\n-- leg 3: co-steward commons capacity pledge (ch09 bytes) --");
  // Fail-soft by construction (see seedCapacityPledge): a storage build that
  // predates the ratios endpoint is a valid deployment state, not an error.
  // Both nodes pledge: the provider is injected server-side from the receiving
  // node's OWN conductor cell, so a pledge posted to A names matthew and one
  // posted to B names jessica — two households' byte budgets, and a
  // totalPledgedBytes that is non-zero on either doorway's card.
  await seedCapacityPledge(clientA);
  await seedCapacityPledge(clientB);

  // --- Measure ---------------------------------------------------------------
  console.log('\n-- measure --');
  for (const [label, url] of [
    ['A', doorwayA],
    ['B', doorwayB],
  ] as const) {
    try {
      const card = await readCard(url, contentId);
      console.log(
        `  doorway ${label}: distributionState=${card.distributionState} ` +
          `stewardingCollectives=${card.stewardingCollectives} ` +
          `commonsCommitments=${card.commitmentBackedReplication?.commonsCommitments} ` +
          `totalPledgedBytes=${card.commitmentBackedReplication?.totalPledgedBytes}`,
      );
    } catch (error) {
      console.log(`  doorway ${label}: unreadable — ${String(error)}`);
      partial += 1;
    }
  }

  if (partial > 0) {
    console.error(`=== Results: ${partial} leg(s) honestly blocked — partial ===`);
    process.exit(2);
  }
  console.log('=== Results: co-steward cast, all legs green ===');
  process.exit(0);
}
