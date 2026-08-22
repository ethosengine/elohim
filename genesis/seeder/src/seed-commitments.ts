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
 *
 * CONTENT_BLOB_HASH / CONTENT_BLOB_SIZE_BYTES are optional on a live mesh:
 * in CI, genesis/Jenkinsfile's "Upload Blob-Backed Content" stage always
 * exports them (a real manifesto-blob upload) before this stage's `when{}`
 * lets it run. A standalone/local-mesh seed chain has no such stage — when
 * both are unset (and CUSTODY_PAIRS_JSON isn't given either), this script
 * uploads a small deterministic dev-fixture blob directly via the doorway's
 * `PUT /admin/seed/blob` (same path substrate-verify.ts's upload step uses)
 * so the default pairs have a REAL, locally-resolvable blob instead of a
 * hard requirement the caller has to satisfy by hand. Passing either env
 * explicitly always takes precedence over the fallback.
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
  /**
   * Exact blob-store address — raw hex (no prefix), or full "sha256-<hex>".
   * Mutually exclusive with `contentId`: declare exactly one. This is the
   * canonical custody join key when the caller already knows the artifact.
   */
  blobHash?: string;
  /**
   * Content id to resolve at seed time against the PROVIDER's storage pod.
   * `artifactRole` is mandatory with this field because a content row may
   * intentionally name two DIFFERENT artifacts: `blobHash` is the browser /
   * primary content blob; `serverBlobHash` is the optional Angular SSR server
   * bundle. Neither is an alias for the other.
   *
   * DRIFT GUARD: the seeded commitment id is a content-addressed hash of
   * (provider_peer, receiver_peer, RESOLVED blob hash) — see
   * `buildCustodyCommitmentBody`. A content re-upload that mints a new
   * `serverBlobHash` therefore mints a NEW commitment id on the next seed
   * run; it does not update the old one in place. The old pledge is not
   * superseded automatically — that reconcile pass (recognize + retire the
   * stale commitment id for a re-uploaded contentId) is out of scope here.
   */
  contentId?: string;
  artifactRole?: 'content' | 'ssr-server';
  /**
   * Optional when `contentId` is set (resolved from the content row's
   * `contentSizeBytes`); required when `blobHash` is set.
   */
  blobSizeBytes?: number;
  /** Formation provenance marker — present only on interim fixtures. */
  fixture?: string;
}

/** A `CustodyPair` with `blobHash`/`blobSizeBytes` resolved to concrete values. */
export type ResolvedCustodyPair = CustodyPair & {
  blobHash: string;
  blobSizeBytes: number;
};

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

/** Wire shape of `GET /api/v1/commitments/capacity/ratios` — camelCase,
 * matches `elohim_views::replicates_commons::CommonsRatioAttestation`. */
export interface CapacityRatioAttestation {
  commonsPct: number;
  dwellingPct: number;
  collectivePct: number;
  freePct: number;
  effectiveRatioCid: string;
}

/** Wire body for `POST /api/v1/commitments/capacity` (the `ReplicatesContentPayload::Capacity`
 * variant). `provider` is injected server-side from the storage node's own
 * conductor cell — this seeder never declares it. */
export interface CapacityPledgeBody {
  variant: 'capacity';
  commonsBytes: number;
  bounds: { ratePerMinute: number; reachCeiling: string };
  ratioAttestation: CapacityRatioAttestation;
}

// =============================================================================
// Body builder (testable in isolation)
// =============================================================================

/**
 * Apply the legacy `sha256-` marker ONLY to bare 64-hex input. CID-form
 * (`baf…`) and already-marked addresses pass through untouched: prefixing a
 * CID-form blob hash mints a double-wrapped `sha256-<cid>` address that NO
 * peer accepts — the responder's strict parse rejects it (T21) on every
 * fetch retry forever, so the blob can never replicate (backlog
 * blob-fetch-sha256-prefixed-cid-rejection; live rows minted by the
 * pre-fix version of this exact function).
 */
function normalizeBlobHash(input: string): string {
  return /^[0-9a-fA-F]{64}$/.test(input) ? `sha256-${input}` : input;
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
 * Resolve a `CustodyPair`'s `blobHash`/`blobSizeBytes` to concrete values,
 * following the pair's declared source:
 *
 * - `blobHash` declared: pass through unchanged (back-compat fixture path).
 *   `blobSizeBytes` must already be declared — there is no row to fall back
 *   to for size.
 * - `contentId` declared: `artifactRole` MUST select the exact row field:
 *   `content` → `blobHash`; `ssr-server` → `serverBlobHash`. There is no
 *   fallback between them because they address different bytes. The
 *   custody-facing join key is whichever exact artifact the manifest and
 *   `shard_locations` describe, not a preferred content-row column.
 *   `blobSizeBytes` resolves from the pair's declared value, else the row's
 *   `contentSizeBytes` (callers pledging an SSR server bundle should declare
 *   its exact size when it differs from the content row's primary artifact).
 *
 * Exactly one of `blobHash`/`contentId` must be declared — declaring both or
 * neither is a fail-fast authoring error, same posture as the no-fallback
 * rule in `resolveCustodyPeerIds`: an unresolvable content row is an honest
 * seed failure, not a decision this function papers over.
 */
export async function resolveCustodyBlobDescriptor(
  pair: CustodyPair,
  opts: ResolvePeerIdOptions = {},
): Promise<{ blobHash: string; blobSizeBytes: number }> {
  const label = `${pair.providerHumanId}→${pair.receiverHumanId}`;
  const hasBlobHash = typeof pair.blobHash === 'string' && pair.blobHash.length > 0;
  const hasContentId = typeof pair.contentId === 'string' && pair.contentId.length > 0;

  if (hasBlobHash && hasContentId) {
    throw new Error(`${label}: pair declares BOTH blobHash and contentId — declare exactly one.`);
  }
  if (!hasBlobHash && !hasContentId) {
    throw new Error(`${label}: pair declares NEITHER blobHash nor contentId — declare exactly one.`);
  }
  if (hasBlobHash && pair.artifactRole) {
    throw new Error(`${label}: artifactRole is only valid with contentId.`);
  }

  if (hasBlobHash) {
    if (typeof pair.blobSizeBytes !== 'number' || pair.blobSizeBytes <= 0) {
      throw new Error(`${label}: blobHash pair requires a positive blobSizeBytes.`);
    }
    return {
      blobHash: normalizeBlobHash(pair.blobHash as string),
      blobSizeBytes: pair.blobSizeBytes,
    };
  }

  if (!pair.artifactRole) {
    throw new Error(`${label}: contentId pair requires artifactRole ('content' or 'ssr-server').`);
  }

  const fetchImpl = opts.fetchImpl ?? fetch;
  const storageUrl = (opts.storageUrl ?? storageUrlForHuman(pair.providerHumanId)).replace(/\/+$/, '');
  const endpoint = `${storageUrl}/db/content/${pair.contentId}`;

  let response: Response;
  try {
    response = await fetchImpl(endpoint, {
      signal: AbortSignal.timeout(opts.timeoutMs ?? 5000),
    });
  } catch (err) {
    throw new Error(`${label}: GET ${endpoint} failed — ${err instanceof Error ? err.message : String(err)}`);
  }
  if (!response.ok) {
    throw new Error(
      `${label}: GET ${endpoint} returned HTTP ${response.status} — cannot resolve contentId "${pair.contentId}".`,
    );
  }

  const row = (await response.json()) as {
    serverBlobHash?: unknown;
    blobHash?: unknown;
    contentSizeBytes?: unknown;
  };

  const field = pair.artifactRole === 'ssr-server' ? 'serverBlobHash' : 'blobHash';
  const candidate = row[field];
  const resolvedHash = typeof candidate === 'string' && candidate.length > 0 ? candidate : undefined;
  if (!resolvedHash) {
    throw new Error(
      `${label}: content row "${pair.contentId}" at ${endpoint} has no ${field} for artifactRole "${pair.artifactRole}".`,
    );
  }
  const normalizedResolvedHash = normalizeBlobHash(resolvedHash);

  // blobSizeBytes resolution order: declared pair value → the content row's
  // own contentSizeBytes → CONTENT_BLOB_SIZE_BYTES when CONTENT_BLOB_HASH
  // names this SAME resolved blob (the mesh/CI env this pair's hash was
  // staged under) → GET /blob/<hash> and read its Content-Length (GET, never
  // HEAD — content-addressed blob routes 404 on HEAD even when GET 200s,
  // matching DoorwayClient.blobExists()'s existing note; no Range, for the
  // same reason). A deploy-time-only field like serverBlobHash routinely
  // lands on the content row with contentSizeBytes still null (a PATCH sets
  // only the hash field), so this mesh needs a real fallback, not just a
  // row-field read.
  let blobSizeBytes: number | undefined =
    pair.blobSizeBytes ?? (typeof row.contentSizeBytes === 'number' ? row.contentSizeBytes : undefined);

  if (blobSizeBytes === undefined) {
    const envHash = process.env.CONTENT_BLOB_HASH;
    const envSize = parseInt(process.env.CONTENT_BLOB_SIZE_BYTES || '', 10);
    if (envHash && Number.isFinite(envSize) && envSize > 0 && normalizeBlobHash(envHash) === normalizedResolvedHash) {
      blobSizeBytes = envSize;
    }
  }

  if (blobSizeBytes === undefined) {
    const blobEndpoint = `${storageUrl}/blob/${normalizedResolvedHash}`;
    try {
      const blobResponse = await fetchImpl(blobEndpoint, {
        signal: AbortSignal.timeout(opts.timeoutMs ?? 5000),
      });
      if (blobResponse.ok) {
        const contentLength = blobResponse.headers.get('content-length');
        const parsed = contentLength ? parseInt(contentLength, 10) : NaN;
        if (Number.isFinite(parsed) && parsed > 0) {
          blobSizeBytes = parsed;
        }
      }
    } catch {
      // A network failure on the blob GET isn't itself fatal — fall through
      // to the "cannot resolve" error below, which names every tier tried.
    }
  }

  if (typeof blobSizeBytes !== 'number' || blobSizeBytes <= 0) {
    throw new Error(
      `${label}: cannot resolve blobSizeBytes for contentId "${pair.contentId}" — pair didn't declare it, the ` +
        `row's contentSizeBytes is missing/invalid, CONTENT_BLOB_SIZE_BYTES doesn't name this blob, and GET ` +
        `${storageUrl}/blob/${normalizedResolvedHash} had no usable Content-Length.`,
    );
  }

  return { blobHash: normalizedResolvedHash, blobSizeBytes };
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
export function buildCustodyCommitmentBody(pair: ResolvedCustodyPair, peerIds: CustodyPeerIds): CommitmentBody {
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
      ...(pair.contentId ? { contentId: pair.contentId, artifactRole: pair.artifactRole } : {}),
      ...(pair.fixture ? { fixture: pair.fixture, retireAt: 'ceremony-landing' } : {}),
    },
  };
}

// =============================================================================
// Default M1 pair set
// =============================================================================

/**
 * Self-custody pledge for the saga ch07 carrier content (2026-07-30):
 * resolved via contentId + the explicit `ssr-server` artifact role, not a
 * fixture blobHash. The live custody evidence for this saga carrier names
 * the SSR server-bundle manifest; the browser bundle is a different
 * artifact and must never be selected by fallback. The deterministic id
 * derivation from (provider|receiver|resolved-blob) keeps re-runs
 * idempotent. DRIFT GUARD: a server-bundle re-upload mints a new
 * serverBlobHash and therefore a NEW commitment id; the old pledge is not
 * superseded automatically (that reconcile pass remains out of scope).
 *
 * Hoisted out of `defaultCustodyPairs()` so it can be read WITHOUT the
 * CONTENT_BLOB_HASH env (that function hard-exits when the blob env is
 * missing). seed-epr-atom.ts reads it to assert the landing atom's custody
 * claim — the claim describes the pledge, never the deploy-time blob hash.
 */
export const LANDING_SELF_CUSTODY_PLEDGE: CustodyPair = {
  providerHumanId: 'human-matthew-manager',
  providerArchetype: 'desktop',
  receiverHumanId: 'human-matthew-manager',
  receiverArchetype: 'desktop',
  contentId: 'elohim-host-landing',
  artifactRole: 'ssr-server',
  // The SSR deployment artifact is distinct from the browser bundle;
  // keep its measured byte count explicit when production supplies it.
};

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
    // Self-custody pledge for the saga ch07 carrier content — declared as
    // LANDING_SELF_CUSTODY_PLEDGE above so seed-epr-atom.ts can read it
    // without the CONTENT_BLOB_HASH env this function requires.
    LANDING_SELF_CUSTODY_PLEDGE,
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

  /** Consent-legibility read: the server's effective_ratios(), which a
   * capacity-pledge ratioAttestation must exact-match. Never throws on a
   * non-2xx — the caller (seedCapacityPledge) inspects the status itself so
   * a storage build that predates this endpoint (404/501) can be skipped. */
  async getCapacityRatios(): Promise<Response> {
    return this.fetch('/api/v1/commitments/capacity/ratios', { method: 'GET' });
  }

  async createCapacityPledge(body: CapacityPledgeBody): Promise<Response> {
    return this.fetch('/api/v1/commitments/capacity', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
  }
}

// =============================================================================
// Seeding (fail-fast on non-409 errors)
// =============================================================================

/** Summary counts from {@link seedCustodyCommitments}, used by the standalone
 * runner to pick the right exit code (Jenkinsfile's runProbedSeeder contract:
 * 0=clean, 1=total, 2=partial). */
export interface SeedCustodyCommitmentsResult {
  created: number;
  alreadyExists: number;
  skipped: number;
  total: number;
}

export async function seedCustodyCommitments(
  client: CommitmentClient,
  pairs: CustodyPair[],
  opts: ResolvePeerIdOptions = {},
): Promise<SeedCustodyCommitmentsResult> {
  console.log(`[seed-commitments] Seeding ${pairs.length} custody-blob commitments...`);

  let created = 0;
  let alreadyExists = 0;
  let skipped = 0;
  const resolvedPairs: CustodyPair[] = [];

  for (const pair of pairs) {
    const label = `${pair.providerHumanId.replace(/^human-/, '')}→${pair.receiverHumanId.replace(/^human-/, '')}`;

    // Resolve the blob descriptor (blobHash pass-through, or contentId +
    // explicit artifactRole → exact row field) and the real peer ids from the
    // live storage pods — per-host cached, so the activate phase below
    // recomputes the SAME content-addressed body.id. A pair unresolvable on
    // THIS mesh (e.g. LANDING_SELF_CUSTODY_PLEDGE's serverBlobHash, which is
    // only minted at deploy time — "absent beats fabricated", same posture
    // seed-epr-atom.ts already takes on this exact artifact) is skipped
    // loudly rather than crashing the whole leg: one pair's resolution gap
    // must never cost the rest of the household its custody commitments.
    let blob: { blobHash: string; blobSizeBytes: number };
    let peerIds: CustodyPeerIds;
    try {
      blob = await resolveCustodyBlobDescriptor(pair, opts);
      peerIds = await resolveCustodyPeerIds(pair, opts);
    } catch (err) {
      console.warn(
        `  [?] ${label}: SKIPPED (unresolvable on this mesh) — ${err instanceof Error ? err.message : String(err)}`,
      );
      skipped += 1;
      continue;
    }

    resolvedPairs.push(pair);
    const resolvedPair: ResolvedCustodyPair = { ...pair, ...blob };
    const body = buildCustodyCommitmentBody(resolvedPair, peerIds);

    // GET-before-POST idempotency check (mirrors activateCustodyCommitments'
    // existing read-first rule below): the storage/doorway create handler
    // upserts on a duplicate id and returns 200/201 with the EXISTING row —
    // it never answers 409. Trusting `response.ok` on the POST alone silently
    // double-counted "created" on every re-run with the same (provider,
    // receiver, blob) triple (verified live: re-POSTing an existing id
    // returns HTTP 201 carrying the ORIGINAL row's note/metadata/createdAt,
    // unchanged). Recognize the existing row from a plain read FIRST — same
    // idempotency posture as the activation phase, applied to the create
    // phase too. A non-404 read failure falls through to the write (fail
    // toward the write, never silently skip a resolvable pair).
    const existing = await client.getCommitment(body.id);
    if (existing.ok) {
      console.log(`  [=] ${label} (idempotent re-run)`);
      alreadyExists += 1;
      continue;
    }

    const response = await client.createCommitment(body);

    if (response.ok) {
      console.log(`  [+] ${label} ${blob.blobHash.slice(0, 16)}...`);
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

  console.log(
    `[seed-commitments] Done. created=${created} already-exists=${alreadyExists} skipped=${skipped} total=${pairs.length}`,
  );

  if (resolvedPairs.length > 0) {
    await activateCustodyCommitments(client, resolvedPairs, opts);
  }

  return { created, alreadyExists, skipped, total: pairs.length };
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
    // Same resolvers as the seed phase — the per-host peer-id cache
    // (peer-id.ts) guarantees an identical id even if a pod flapped between
    // phases; the blob descriptor is re-resolved from the same content row.
    const blob = await resolveCustodyBlobDescriptor(pair, opts);
    const resolvedPair: ResolvedCustodyPair = { ...pair, ...blob };
    const body = buildCustodyCommitmentBody(resolvedPair, await resolveCustodyPeerIds(pair, opts));
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
// Default capacity pledge (matthew-household dwelling pledges commons capacity)
// =============================================================================

/**
 * 25 GiB — the default commons byte-budget pledge seeded for the
 * matthew-household dwelling's storage node. Matches
 * `capacity_pledge_author.rs`'s `valid_capacity()` test fixture magnitude
 * (64 MiB there is a unit-test amount; this is the seeded default).
 */
const DEFAULT_CAPACITY_COMMONS_BYTES = 25 * 1024 * 1024 * 1024; // 26_843_545_600

/**
 * Seed the default commons capacity pledge: the matthew-household dwelling's
 * storage node explicitly consents to host `DEFAULT_CAPACITY_COMMONS_BYTES`
 * for the commons tier.
 *
 * Rollout-compatible: `GET /api/v1/commitments/capacity/ratios` is the
 * consent-legibility read this pledge needs to construct a `ratioAttestation`
 * that exact-matches `constitutional_ratio_registry::effective_ratios()`
 * (`replicates_commons_validator`'s donut check — see `handle_get_effective_ratios`
 * doc comment in `elohim/elohim-storage/src/api/rea_commitments.rs`). A
 * storage build that predates the endpoint (404/501) is a valid deployment
 * state, not an error — this pledge is skipped, loudly, without failing the
 * seeder.
 *
 * Fail-soft throughout: any failure (network error, non-2xx GET, non-2xx
 * POST) is logged and returns without throwing. Custody-commitment seeding's
 * outcome is never affected by a capacity-pledge failure — this MUST run
 * after `seedCustodyCommitments`, never gate it.
 *
 * Idempotent by construction: the POST handler
 * (`capacity_pledge_author::author_capacity_pledge`) recognizes an
 * already-live exact pledge (`is_exact_live_pledge`) and returns it instead
 * of minting a duplicate, so re-running this seeder is safe.
 */
export async function seedCapacityPledge(client: CommitmentClient): Promise<void> {
  console.log('[seed-commitments] Seeding matthew-household commons capacity pledge...');

  let ratiosResponse: Response;
  try {
    ratiosResponse = await client.getCapacityRatios();
  } catch (err) {
    console.warn(
      `[seed-commitments] capacity pledge: GET /api/v1/commitments/capacity/ratios failed — ${
        err instanceof Error ? err.message : String(err)
      } — skipping.`,
    );
    return;
  }

  if (ratiosResponse.status === 404 || ratiosResponse.status === 501) {
    console.warn('[seed-commitments] storage predates capacity ratios endpoint — skipping capacity pledge');
    return;
  }

  if (!ratiosResponse.ok) {
    const text = await ratiosResponse.text();
    console.warn(
      `[seed-commitments] capacity pledge: GET ratios returned HTTP ${ratiosResponse.status} — skipping. Body: ${text.slice(0, 300)}`,
    );
    return;
  }

  const ratioAttestation = (await ratiosResponse.json()) as CapacityRatioAttestation;

  const body: CapacityPledgeBody = {
    variant: 'capacity',
    commonsBytes: DEFAULT_CAPACITY_COMMONS_BYTES,
    bounds: { ratePerMinute: 12, reachCeiling: 'commons' },
    ratioAttestation,
  };

  let response: Response;
  try {
    response = await client.createCapacityPledge(body);
  } catch (err) {
    console.warn(
      `[seed-commitments] capacity pledge: POST /api/v1/commitments/capacity failed — ${
        err instanceof Error ? err.message : String(err)
      } — continuing (custody seeding outcome is unaffected).`,
    );
    return;
  }

  if (response.ok) {
    console.log('[seed-commitments] capacity pledge: ok');
    return;
  }

  const text = await response.text();
  console.warn(
    `[seed-commitments] capacity pledge: POST returned HTTP ${response.status} — continuing (custody seeding outcome is unaffected). Body: ${text.slice(0, 300)}`,
  );
}

// =============================================================================
// Mesh-local dev-fixture blob (fallback when CONTENT_BLOB_HASH is unset)
// =============================================================================

/**
 * Upload a tiny deterministic dev-fixture blob via the SAME admin seed path
 * substrate-verify.ts's `upload` subcommand uses (`PUT /admin/seed/blob`,
 * unauthenticated on a dev-mode doorway) and stamp CONTENT_BLOB_HASH /
 * CONTENT_BLOB_SIZE_BYTES from the result. Only called when the caller gave
 * neither CUSTODY_PAIRS_JSON nor a real CONTENT_BLOB_HASH/SIZE — CI always
 * supplies the real blob env first, so this path never runs there.
 *
 * Fail-fast on upload failure (loud, single line) — never silently invent a
 * hash the custody sweep could never resolve.
 */
async function provisionDevFixtureBlob(client: CommitmentClient): Promise<void> {
  const data = Buffer.from(
    'elohim household custody dev fixture\n' +
      'source: genesis/seeder/src/seed-commitments.ts (mesh-local fallback — ' +
      'no CONTENT_BLOB_HASH env, no substrate-verify upload stage)\n',
    'utf8',
  );
  const blobHash = `sha256-${createHash('sha256').update(data).digest('hex')}`;

  const push = await client.pushBlob(blobHash, data, { hash: blobHash, sizeBytes: data.length, mimeType: 'text/plain' });
  if (!push.success) {
    console.error(
      `ERROR: CONTENT_BLOB_HASH/CONTENT_BLOB_SIZE_BYTES unset and the mesh-local dev-fixture ` +
        `blob upload failed: ${push.error}. Set CONTENT_BLOB_HASH + CONTENT_BLOB_SIZE_BYTES ` +
        `explicitly (or CUSTODY_PAIRS_JSON) to bypass.`,
    );
    process.exit(1);
  }

  process.env.CONTENT_BLOB_HASH = blobHash;
  process.env.CONTENT_BLOB_SIZE_BYTES = String(data.length);
  console.log(
    `[seed-commitments] CONTENT_BLOB_HASH/SIZE unset — uploaded mesh-local dev-fixture blob ` +
      `${blobHash} (${data.length} bytes)`,
  );
}

// =============================================================================
// Standalone execution
// =============================================================================

const isMain = import.meta.url === `file://${process.argv[1]}`;
if (isMain) {
  const doorwayUrl = process.env.DOORWAY_URL || 'http://localhost:8888';
  const apiKey = process.env.DOORWAY_API_KEY;

  const client = new CommitmentClient({ baseUrl: doorwayUrl, apiKey });

  console.log('='.repeat(60));
  console.log('REA Custody-Blob Commitment Seeder');
  console.log(`  Target: ${doorwayUrl}`);
  console.log('='.repeat(60));
  console.log();

  const health = await client.checkHealth();
  if (!health.healthy) {
    console.error(`ERROR: Doorway not healthy — ${health.error}`);
    process.exit(1);
  }

  const pairsJsonPath = process.env.CUSTODY_PAIRS_JSON;
  const hasBlobEnv =
    !!process.env.CONTENT_BLOB_HASH && parseInt(process.env.CONTENT_BLOB_SIZE_BYTES || '0', 10) > 0;
  if (!pairsJsonPath && !hasBlobEnv) {
    await provisionDevFixtureBlob(client);
  }

  const pairs: CustodyPair[] = pairsJsonPath
    ? (JSON.parse(readFileSync(pairsJsonPath, 'utf-8')) as CustodyPair[])
    : defaultCustodyPairs();

  // The terrance-drift flag: never seed custody for suspended personas.
  assertPairsNotSuspended(pairs, loadSuspendedHumanPrefixes());

  console.log(`  Pairs:  ${pairs.length}`);
  console.log();

  const result = await seedCustodyCommitments(client, pairs);
  await seedCapacityPledge(client);

  // Partial-vs-total contract (Jenkinsfile's runProbedSeeder): 0=clean,
  // 2=partial (some pairs unresolvable on this mesh), 1=total (handled above
  // by the per-pair process.exit(1) on a real HTTP/shape failure).
  if (result.skipped > 0 && result.created + result.alreadyExists > 0) {
    console.error(
      `=== Results: ${result.created} created, ${result.alreadyExists} already-exists, ${result.skipped} skipped (${result.total} total) — partial ===`,
    );
    process.exit(2);
  }
  if (result.skipped > 0 && result.created + result.alreadyExists === 0) {
    console.error(
      `=== Results: 0 created, 0 already-exists, ${result.skipped} skipped (${result.total} total) — total failure ===`,
    );
    process.exit(1);
  }
  console.log(
    `=== Results: ${result.created} created, ${result.alreadyExists} already-exists, 0 skipped (${result.total} total) ===`,
  );
  process.exit(0);
}
