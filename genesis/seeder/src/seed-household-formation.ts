#!/usr/bin/env npx tsx
/**
 * Seed: household formation ceremony (Stage 1, rung-3 realism).
 * Spec: genesis/docs/superpowers/specs/2026-06-04-household-formation-ceremony-design.md §3, §6.
 *
 * REALISM RUNG: 3 (conductor-zome-as-agent). Every act below is authored by the
 * persona's OWN conductor agent. Genesis data = "this family ran the ceremony."
 * Ordering: AFTER seed-conductor-identities + seed-agent-bindings.
 *
 * Choreography (spec §3):
 *   1. matthew (founder) `create_collective` (household charter) on HIS conductor.
 *   2. for each non-founder member: matthew `issue_household_invite` on HIS
 *      conductor; the member `affirm_membership` on THEIR conductor.
 *   3. if james (minor) affirmed: matthew `create_stewardship_grant` over james
 *      on HIS conductor (parental authority, age-bounded device policy).
 *   4. each affirmed member authors a `custody-blob` commitment toward every
 *      other affirmed member via `content_store.create_rea_commitment` on the
 *      PROVIDER's own conductor (lamad cell). Self-skips when M1 blob env unset.
 *
 * Env: CONDUCTOR_URLS (comma-separated app WS urls), INSTALLED_APP_ID prefix
 * (default 'elohim'), CONTENT_BLOB_HASH + CONTENT_BLOB_SIZE_BYTES (custody payload;
 * custody layer self-skips when absent), HOUSEHOLD_SALT (32 hex; deterministic
 * default), HOUSEHOLD_NONCE_PREFIX (default 'genesis').
 *
 * Exit codes (partial-readiness aware, mirrors the sibling seeders):
 *   0 — complete: triad affirmed, no custody failures
 *   2 — partial: < 3 affirmed OR at least one custody write failed
 *   1 — fatal: no conductors / founder collective creation failed
 */

import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';

// Canonical artifact filename from build-artifacts.json — single source of
// truth across Groovy + TypeScript + JS. Resolved once at module load so
// the writeFileSync below cannot drift from what genesis/Jenkinsfile reads.
const ARTIFACTS_MANIFEST_PATH = resolve(
  dirname(fileURLToPath(import.meta.url)),
  '..', '..', 'orchestrator', 'build-artifacts.json',
);
const ARTIFACTS = JSON.parse(readFileSync(ARTIFACTS_MANIFEST_PATH, 'utf8'));
const SEED_RESULTS_FILE: string =
  ARTIFACTS.genesis.seedResultsHouseholdFormation ?? 'seed-results-household-formation.json';
import {
  AdminWebsocket,
  AppWebsocket,
  encodeHashToBase64,
  type AppInfo,
} from '@holochain/client';
import { deterministicPeerId, resolvePeerId, type Archetype } from './peer-id.js';
import type { CustodyPeerIds } from './seed-commitments.js';

// =============================================================================
// Canonical household triad
// =============================================================================

export interface HouseholdMember {
  humanId: string;
  archetype: Archetype;
  role: 'steward' | 'contributor';
  minor: boolean;
}

/**
 * The canonical Dowell-family triad, founder first. Order is load-bearing:
 * the founder (index 0) creates the collective; everyone else affirms against
 * the founder's invite.
 */
export const HOUSEHOLD_MEMBERS: HouseholdMember[] = [
  { humanId: 'human-matthew-manager', archetype: 'desktop', role: 'steward', minor: false },
  { humanId: 'human-jessica-spouse', archetype: 'desktop', role: 'steward', minor: false },
  { humanId: 'human-james-son', archetype: 'mobile', role: 'contributor', minor: true },
];

// =============================================================================
// Pure builders (exported for unit tests)
// =============================================================================

/**
 * The household charter JSON. `kind`/`rubric` select the recognition-of-given
 * membership-acquisition flow; `slugAlias` ties the collective to the
 * `family-dowell` story (validation scenarios + collectives.json).
 */
export function buildHouseholdCharter(): string {
  return JSON.stringify({ kind: 'household', rubric: 'recognition-of-given', slugAlias: 'family-dowell' });
}

/**
 * Elect the founder: the first HOUSEHOLD_MEMBERS entry (in declared, deterministic
 * order) whose humanId has a bound conductor session. HOUSEHOLD_MEMBERS[0] is the
 * preferred/nominal founder, but a bound session isn't guaranteed for the nominal
 * first member — e.g. a doorway-registered human whose conductor embodies a
 * UUID-minted Human (auth_routes.rs `generated_human_id`) rather than the canonical
 * household id can never match (findMemberSessions binds by exact humanId). Pure
 * over anything with a `.has(id)` (a `Set<string>` or the sessions `Map` itself) so
 * it's unit-testable without standing up conductors. Returns `undefined` when NO
 * member bound a session — the caller keeps that case FATAL.
 */
export function electFounder(
  members: readonly HouseholdMember[],
  boundHumanIds: { has(id: string): boolean },
): HouseholdMember | undefined {
  return members.find(m => boundHumanIds.has(m.humanId));
}

export interface CeremonyCustodyParams {
  providerHumanId: string;
  providerArchetype: Archetype;
  receiverHumanId: string;
  receiverArchetype: Archetype;
  blobHash: string;
  blobSizeBytes: number;
  collectiveCid: string;
}

/**
 * Snake_case `shefa_types::CreateReaCommitmentInput` for a single ceremony
 * custody-blob pair (conductor boundary — MessagePack maps, NOT camelCase HTTP).
 *
 * `id` is a deterministic content-address of (provider_peer, receiver_peer,
 * blob_hash) so re-runs produce identical ids (DHT-level idempotent). The pair
 * is scoped to the household collective via `in_scope_of`, and carries
 * `seedGeneration: 'ceremony'` provenance so views can distinguish ceremony
 * output from the retiring interim fixtures (spec §7).
 *
 * `peerIds` carries the resolved REAL peer ids (Stage 2 — peer-id.ts); when
 * omitted the body falls back to Stage-1 deterministic ids — isolated
 * shape-tests only. The live ceremony path always resolves first: the storage
 * custody sweep kicks a blob fetch iff commitment.provider == the node's REAL
 * peer id (and observes placement gaps iff receiver matches), which a Stage-1
 * fake id can never equal. Same contract as buildCustodyCommitmentBody.
 *
 * NOTE: `resource_conforms_to` is intentionally absent — it is not a field on
 * the DNA wire struct (`shefa_types::CreateReaCommitmentInput`); the zome sets
 * it to `None` itself.
 */
export function buildCeremonyCustodyInput(p: CeremonyCustodyParams, peerIds?: CustodyPeerIds) {
  const provider = peerIds?.provider ?? deterministicPeerId(p.providerHumanId, p.providerArchetype);
  const receiver = peerIds?.receiver ?? deterministicPeerId(p.receiverHumanId, p.receiverArchetype);
  const idDigest = createHash('sha256')
    .update(`${provider}|${receiver}|${p.blobHash}`)
    .digest('hex')
    .slice(0, 16);
  return {
    id: `custody-blob-${idDigest}`,
    action: 'custody-blob',
    provider,
    receiver,
    resource_classified_as: [p.blobHash],
    resource_quantity_value: p.blobSizeBytes,
    resource_quantity_unit: 'B',
    in_scope_of: [p.collectiveCid],
    note: `household custody: ${p.providerHumanId} -> ${p.receiverHumanId}`,
    metadata_json: JSON.stringify({
      seedGeneration: 'ceremony',
      blobHash: p.blobHash,
      providerHumanId: p.providerHumanId,
      receiverHumanId: p.receiverHumanId,
    }),
  };
}

// =============================================================================
// Holochain plumbing (copied verbatim from seed-conductor-identities.ts,
// parameterized over the cell ROLE so this script can resolve both the
// imagodei cell — formation calls — and the lamad cell — custody zome calls).
// =============================================================================

const CONNECT_TIMEOUT_MS = parseInt(process.env.CONDUCTOR_CONNECT_TIMEOUT_MS ?? '10000', 10);

async function withTimeout<T>(promise: Promise<T>, timeoutMs: number, label: string): Promise<T> {
  let timer: NodeJS.Timeout | undefined;
  const timeout = new Promise<never>((_, reject) => {
    timer = setTimeout(() => reject(new Error(`${label} timed out after ${timeoutMs}ms`)), timeoutMs);
  });
  try {
    return await Promise.race([promise, timeout]);
  } finally {
    if (timer) clearTimeout(timer);
  }
}

/**
 * Derive admin WebSocket URL from app WebSocket URL.
 * Convention (socat in K8s): admin port = app port - 1 (4445 app → 4444 admin).
 */
function toAdminUrl(appUrl: string): string {
  const u = new URL(appUrl);
  const appPort = parseInt(u.port, 10);
  if (isNaN(appPort)) {
    throw new Error(`Cannot derive admin port from app URL: ${appUrl}`);
  }
  u.port = String(appPort - 1);
  return u.toString();
}

type CellId = [Uint8Array, Uint8Array];

/**
 * Resolve a role's provisioned cell_id from an AppInfo. Returns null when the
 * role is absent (no cells array) OR when the role key is present but no cell
 * is provisioned yet — both cases let the caller decide (imagodei null → fatal
 * throw in connectToConductor; lamad null → one-line warn + continue).
 *
 * @holochain/client returns two possible cell shapes depending on version:
 *   { type: "provisioned", value: { cell_id: [...] } }  — newer
 *   { provisioned: { cell_id: [...] } }                 — older
 */
function cellForRole(matchingApp: AppInfo, role: string): CellId | null {
  const cells = matchingApp.cell_info[role];
  if (!cells || cells.length === 0) {
    return null;
  }
  for (const cell of cells as unknown[]) {
    const c = cell as Record<string, unknown>;
    if (c['type'] === 'provisioned' && c['value']) {
      return (c['value'] as Record<string, unknown>)['cell_id'] as CellId;
    } else if (c['provisioned']) {
      return (c['provisioned'] as Record<string, unknown>)['cell_id'] as CellId;
    }
  }
  // Role key present but no cell is provisioned — return null so the caller decides.
  return null;
}

interface ConductorSession {
  appWs: AppWebsocket;
  imagodeiCell: CellId;
  lamadCell: CellId | null;
  appInfo: AppInfo;
}

/**
 * Connect to a conductor and find an installed app starting with the prefix.
 * Returns null if no matching app is found (this conductor isn't for us).
 * Resolves BOTH the imagodei cell (required — formation calls) and the lamad
 * cell (optional — custody zome calls; null → custody soft-skips for that peer).
 */
async function connectToConductor(
  appUrl: string,
  appIdPrefix: string,
): Promise<ConductorSession | null> {
  const adminUrl = toAdminUrl(appUrl);

  let adminWs: AdminWebsocket;
  try {
    adminWs = await withTimeout(
      AdminWebsocket.connect({
        url: new URL(adminUrl),
        wsClientOptions: { origin: 'http://localhost' },
      }),
      CONNECT_TIMEOUT_MS,
      `Admin connect ${adminUrl}`,
    );
  } catch (err) {
    throw new Error(
      `Admin connect failed (${adminUrl}): ${err instanceof Error ? err.message : err}`,
    );
  }

  try {
    const apps = await adminWs.listApps({});
    const matchingApp = apps.find(a => a.installed_app_id.startsWith(appIdPrefix));

    if (!matchingApp) {
      await adminWs.client.close();
      return null;
    }

    const imagodeiCell = cellForRole(matchingApp, 'imagodei');
    if (!imagodeiCell) {
      await adminWs.client.close();
      throw new Error(`App '${matchingApp.installed_app_id}' imagodei cell is not provisioned`);
    }
    // lamad cell is optional — custody calls soft-skip when it's absent.
    const lamadCell = cellForRole(matchingApp, 'lamad');
    if (!lamadCell) {
      console.warn(`  [!] ${matchingApp.installed_app_id}: lamad cell not provisioned — custody will be skipped for this peer`);
    }

    // Authorize signing credentials for whatever cells we resolved.
    await adminWs.authorizeSigningCredentials(imagodeiCell);
    if (lamadCell) {
      await adminWs.authorizeSigningCredentials(lamadCell);
    }

    const tokenResult = await adminWs.issueAppAuthenticationToken({
      installed_app_id: matchingApp.installed_app_id,
      single_use: true,
      expiry_seconds: 300,
    });

    await adminWs.client.close();

    const appWs = await withTimeout(
      AppWebsocket.connect({
        url: new URL(appUrl),
        token: tokenResult.token,
        wsClientOptions: { origin: 'http://localhost' },
      }),
      CONNECT_TIMEOUT_MS,
      `App connect ${appUrl}`,
    );

    return { appWs, imagodeiCell, lamadCell, appInfo: matchingApp };
  } catch (err) {
    try {
      await adminWs.client.close();
    } catch {
      // ignore close errors
    }
    throw err;
  }
}

// =============================================================================
// Session discovery — match conductors to household members by their Human id
// =============================================================================

interface MemberSession {
  member: HouseholdMember;
  conductorUrl: string;
  session: ConductorSession;
}

interface GetMyHumanResult {
  human?: { id?: string } | null;
}

/**
 * Walk every conductor URL, read its agent's Human profile (imagodei.get_my_human),
 * and bind the first conductor whose Human id matches each household member.
 * Sessions that match no member are closed; matched sessions stay open for the
 * ceremony.
 */
async function findMemberSessions(
  conductorUrls: string[],
  appIdPrefix: string,
): Promise<Map<string, MemberSession>> {
  const found = new Map<string, MemberSession>();
  const wantById = new Map(HOUSEHOLD_MEMBERS.map(m => [m.humanId, m]));

  for (const conductorUrl of conductorUrls) {
    let session: ConductorSession | null = null;
    // DHT-settle retry: right after Seed Conductor Identities creates fresh
    // Human entries, the conductor's signing-credential commit can fail with
    // "Source chain error … DepMissingFromDht" — the deps are seconds-old and
    // not yet integrated. The conductor itself says "may be retried" (genesis
    // #1123 killed formation on exactly this). Retry with a settle delay;
    // only this transient class re-attempts, real errors fail fast.
    const CONNECT_ATTEMPTS = 4;
    const SETTLE_MS = 15_000;
    for (let attempt = 1; attempt <= CONNECT_ATTEMPTS; attempt++) {
      try {
        session = await connectToConductor(conductorUrl, appIdPrefix);
        break;
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        const retriable =
          msg.includes('DepMissingFromDht') || msg.includes('may be retried');
        if (retriable && attempt < CONNECT_ATTEMPTS) {
          console.warn(
            `  [~] connect (${conductorUrl}): DHT deps not yet integrated (attempt ${attempt}/${CONNECT_ATTEMPTS}) — retrying in ${SETTLE_MS / 1000}s`,
          );
          await new Promise(r => setTimeout(r, SETTLE_MS));
          continue;
        }
        console.error(`  [X] connect (${conductorUrl}): ${msg}`);
        break;
      }
    }
    if (!session) {
      continue;
    }

    let humanId: string | undefined;
    try {
      const result = (await session.appWs.callZome({
        cell_id: session.imagodeiCell,
        zome_name: 'imagodei',
        fn_name: 'get_my_human',
        payload: null,
      })) as GetMyHumanResult | null;
      humanId = result?.human?.id ?? undefined;
    } catch (err) {
      console.error(
        `  [X] get_my_human (${conductorUrl}): ${err instanceof Error ? err.message : err}`,
      );
    }

    const member = humanId ? wantById.get(humanId) : undefined;
    if (member && !found.has(member.humanId)) {
      found.set(member.humanId, { member, conductorUrl, session });
      console.log(`  [=] ${member.humanId.padEnd(22)} ${conductorUrl}`);
    } else {
      // Not a member we need (or already bound) — release the session.
      try {
        await session.appWs.client.close();
      } catch {
        // ignore close errors
      }
    }
  }

  return found;
}

// =============================================================================
// Hash → canonical display-string encoding finding
// =============================================================================
//
// callZome for create_collective returns the Collective's ActionHash. The
// @holochain/client (0.20) decoder returns a HoloHash as a Uint8Array, NOT the
// canonical "uhCkk..." base64 display string. The Rust side encodes the cid as
// `format!("collective:{hash}")` where `{hash}` is the ActionHash's Display impl
// (canonical base64). To make our `collective:<cid>` match the Rust display form
// (so `in_scope_of` scoping + anchor lookups line up), we MUST run the returned
// bytes through `encodeHashToBase64` — concatenating the raw Uint8Array would
// yield a comma-joined byte string and silently break scope matching.
function encodeActionHash(returned: unknown): string {
  if (returned instanceof Uint8Array) {
    return encodeHashToBase64(returned);
  }
  // Some decoders hand back { hash: Uint8Array } or already-encoded strings.
  if (typeof returned === 'string') {
    return returned;
  }
  if (returned && typeof returned === 'object') {
    const h = (returned as Record<string, unknown>)['hash'];
    if (h instanceof Uint8Array) {
      return encodeHashToBase64(h);
    }
  }
  throw new Error(`create_collective returned an un-encodable hash: ${typeof returned}`);
}

// =============================================================================
// Projection probe — check if the household collective already exists
// =============================================================================

const PROBE_TIMEOUT_MS = 5000;

/**
 * Query the storage/doorway projection to find an existing `family-dowell`
 * collective cid. Returns the cid string (e.g. `collective:uhCkk...`) if the
 * collective is already present, or `null` on probe miss / error (caller then
 * falls through to create_collective as normal).
 *
 * Soft-fails on all network / JSON errors — a probe failure never aborts seeding.
 */
export async function resolveExistingCollectiveCid(baseUrl: string): Promise<string | null> {
  const url = `${baseUrl.replace(/\/$/, '')}/db/collectives/family-dowell`;
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), PROBE_TIMEOUT_MS);
  try {
    const headers: Record<string, string> = {};
    if (process.env.DOORWAY_API_KEY) {
      headers['Authorization'] = `Bearer ${process.env.DOORWAY_API_KEY}`;
    }
    const res = await fetch(url, { signal: controller.signal, headers });
    if (!res.ok) {
      return null;
    }
    const json = (await res.json()) as Record<string, unknown>;
    const cid = json['collectiveCid'] ?? json['collective_cid'];
    if (typeof cid === 'string' && cid.startsWith('collective:')) {
      return cid;
    }
    return null;
  } catch {
    return null;
  } finally {
    clearTimeout(timer);
  }
}

/**
 * Founder-chain fallback for `resolveExistingCollectiveCid`: ask the
 * founder's OWN conductor (their source chain, via
 * `imagodei.get_my_household_collective_cids`) whether a household
 * collective already exists, bypassing the storage/doorway projection
 * entirely.
 *
 * This matters because the projection probe above can MISS even when a
 * collective genuinely exists: its SQL stamp lags cross-conductor DHT
 * gossip/reconcile — the same class of lag that races `affirm_membership`
 * (see the settle-retry loop in the invite/affirm loop below). A genesis run
 * that targets one doorway while the founder's OWN conductor authored the
 * entry can outrun that settle window every time, so re-running never
 * converges without a probe that stays on the founder's own chain.
 *
 * `get_my_household_collective_cids` already filters to Person memberships
 * whose collective charter declares `{"kind":"household"}` (qahal_coordinator.rs).
 * This ceremony is the sole author of `family-dowell` household charters for
 * this founder identity in any one genesis environment, so every returned
 * cid IS this household by construction — no further per-cid charter/
 * slugAlias fetch-and-decode is needed (that would require a second msgpack
 * decode pass over the raw DHT entry bytes, which the seeder carries no
 * dependency for today).
 *
 * More than one cid means an earlier partial run minted an orphan collective
 * before this reuse path existed (or before its own projection settled).
 * `get_my_household_collective_cids` reads the founder's own source chain via
 * `query()`, which preserves creation (action-sequence) order, so the LAST
 * entry is the most recently authored — that one is reused; the rest are
 * reported back as an orphan count for operator visibility.
 *
 * Returns `null` on any zome-call failure (no session, DNA fn not deployed
 * yet, network hiccup) or an empty result — the caller falls through to
 * `create_collective` exactly as before.
 */
export async function resolveFounderChainCollectiveCid(
  founderSession: MemberSession,
): Promise<{ cid: string; orphanCount: number } | null> {
  try {
    const cids = (await founderSession.session.appWs.callZome({
      cell_id: founderSession.session.imagodeiCell,
      zome_name: 'imagodei',
      fn_name: 'get_my_household_collective_cids',
      payload: null,
    })) as string[];
    if (!Array.isArray(cids) || cids.length === 0) {
      return null;
    }
    return { cid: cids[cids.length - 1], orphanCount: cids.length - 1 };
  } catch (err) {
    console.warn(
      `[!] get_my_household_collective_cids probe (non-fatal): ${err instanceof Error ? err.message : err}`,
    );
    return null;
  }
}

/**
 * Pure settle predicate for the household-formation projection (genesis #1182
 * Cluster B). True iff the collective's `collective_cid` is stamped AND every
 * expected member id is present in the projected participants — i.e. the
 * formation projection has run and reflects the affirmed triad. The (network)
 * `waitForHouseholdProjected` loops on this; kept pure so it is unit-tested
 * DB-free. Participant rows may be plain id strings (the `Vec<String>` view
 * shape) or objects carrying `humanId`/`id`.
 */
export function householdProjectionSatisfied(
  collective: Record<string, unknown> | null,
  participants: unknown[],
  expectedMemberIds: string[],
): boolean {
  if (!collective) {
    return false;
  }
  const cid = collective['collectiveCid'] ?? collective['collective_cid'];
  if (typeof cid !== 'string' || !cid.startsWith('collective:')) {
    return false;
  }
  const present = new Set(participants.map(participantId));
  return expectedMemberIds.every(id => present.has(id));
}

/**
 * The member id one projected participant row names. Rows may be plain id
 * strings (the `Vec<String>` view shape) or objects carrying
 * `humanId`/`id`/`participant`. Shared by the settle predicate and its
 * gap description so the two can never disagree about who is present.
 */
function participantId(p: unknown): string {
  if (typeof p === 'string') {
    return p;
  }
  if (p && typeof p === 'object') {
    const o = p as Record<string, unknown>;
    return (o['humanId'] ?? o['id'] ?? o['participant'] ?? '') as string;
  }
  return '';
}

/**
 * Wait for the household-formation projection to reflect the affirmed members
 * (genesis #1182 Cluster B). Polls `/db/collectives/<slug>` (collective_cid
 * stamp) and `/db/collectives/<slug>/participants` until
 * `householdProjectionSatisfied`, or a bounded timeout. SOFT — a probe error or
 * timeout NEVER aborts seeding (the projection may still settle after exit); it
 * logs and returns false. Mirrors `resolveExistingCollectiveCid`'s soft-fail.
 */
export async function waitForHouseholdProjected(
  baseUrl: string,
  expectedMemberIds: string[],
  opts: { slug?: string; timeoutMs?: number; intervalMs?: number } = {},
): Promise<boolean> {
  const slug = opts.slug ?? 'family-dowell';
  const timeoutMs = opts.timeoutMs ?? 60_000;
  const intervalMs = opts.intervalMs ?? 3_000;
  const base = baseUrl.replace(/\/$/, '');
  const headers: Record<string, string> = {};
  if (process.env.DOORWAY_API_KEY) {
    headers['Authorization'] = `Bearer ${process.env.DOORWAY_API_KEY}`;
  }
  const fetchJson = async (path: string): Promise<Record<string, unknown> | null> => {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), PROBE_TIMEOUT_MS);
    try {
      const res = await fetch(`${base}${path}`, { signal: controller.signal, headers });
      if (!res.ok) {
        return null;
      }
      return (await res.json()) as Record<string, unknown>;
    } catch {
      return null;
    } finally {
      clearTimeout(timer);
    }
  };
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    const collective = await fetchJson(`/db/collectives/${slug}`);
    const partsRaw = await fetchJson(`/db/collectives/${slug}/participants`);
    const participants = (partsRaw?.['items'] ?? partsRaw?.['participants'] ?? []) as unknown[];
    if (householdProjectionSatisfied(collective, participants, expectedMemberIds)) {
      return true;
    }
    if (Date.now() >= deadline) {
      // Name WHICH leg is short. The old single line ("collective_cid/
      // participants may lag") could not distinguish an unstamped cid from a
      // short participant list from an absent row, so the downstream a2o red had
      // to re-derive it from scratch (genesis #1489). These are different
      // defects: an unstamped cid means the CollectiveCommitted signal landed on
      // another peer's storage and reconcile has not gap-filled it here; a
      // missing member means their conductor never affirmed.
      console.warn(
        `[!] household projection did not settle within ${timeoutMs}ms — ` +
          `${describeProjectionGap(collective, participants, expectedMemberIds)} ` +
          `(non-fatal; projection_reconcile continues after exit)`,
      );
      return false;
    }
    await new Promise(r => setTimeout(r, intervalMs));
  }
}

/**
 * Human-readable statement of exactly which settle leg is unmet, for the
 * timeout warning above. Pure over the same inputs as
 * [`householdProjectionSatisfied`], and reuses its participant-id reader, so it
 * cannot describe a state that predicate would have accepted.
 */
export function describeProjectionGap(
  collective: Record<string, unknown> | null,
  participants: unknown[],
  expectedMemberIds: string[],
): string {
  if (!collective) {
    return 'the collective row is absent from this storage peer';
  }
  const gaps: string[] = [];
  const cid = collective['collectiveCid'] ?? collective['collective_cid'];
  if (typeof cid !== 'string' || !cid.startsWith('collective:')) {
    gaps.push(`collective_cid unstamped (got ${JSON.stringify(cid ?? null)})`);
  }
  const present = participants.map(participantId).filter(Boolean);
  const missing = expectedMemberIds.filter(id => !present.includes(id));
  if (missing.length > 0) {
    gaps.push(
      `participants missing ${missing.join(', ')} (present: ${present.join(', ') || 'none'})`,
    );
  }
  return gaps.length > 0 ? gaps.join('; ') : 'no gap detected on the final poll (race with settle)';
}

// =============================================================================
// Main
// =============================================================================

// Module-level sessions reference so the fatal catch can best-effort close.
let activeSessions: Map<string, MemberSession> = new Map();

async function main(): Promise<void> {
  const conductorUrlsRaw = process.env.CONDUCTOR_URLS ?? '';
  const appIdPrefix = process.env.INSTALLED_APP_ID ?? 'elohim';
  const salt = process.env.HOUSEHOLD_SALT ?? 'f00df00df00df00df00df00df00df00d';
  const noncePrefix = process.env.HOUSEHOLD_NONCE_PREFIX ?? 'genesis';
  const blobHash = process.env.CONTENT_BLOB_HASH ?? '';
  const blobSizeBytes = parseInt(process.env.CONTENT_BLOB_SIZE_BYTES ?? '0', 10);

  const conductorUrls = conductorUrlsRaw
    .split(',')
    .map(u => u.trim())
    .filter(Boolean);

  console.log('=== Seed Household Formation (ceremony, rung 3) ===\n');
  console.log(`App ID prefix: ${appIdPrefix}`);
  console.log(
    `Conductors:    ${conductorUrls.length > 0 ? conductorUrls.join(', ') : '(none — set CONDUCTOR_URLS)'}`,
  );
  console.log('');

  if (conductorUrls.length === 0) {
    console.error('ERROR: CONDUCTOR_URLS is not set. Set it to comma-separated conductor app WebSocket URLs.');
    console.error('  Example: CONDUCTOR_URLS=ws://elohim-adam-alpha:4445,ws://elohim-eve-alpha:4445');
    process.exit(1);
  }

  // -- Bind conductors to members ------------------------------------------
  console.log('Binding conductors to household members:');
  const sessions = await findMemberSessions(conductorUrls, appIdPrefix);
  activeSessions = sessions as typeof activeSessions;
  console.log('');

  // See electFounder() docstring for why the founder can't just be HOUSEHOLD_MEMBERS[0].
  const electedFounder = electFounder(HOUSEHOLD_MEMBERS, sessions);

  const closeAll = async () => {
    for (const s of sessions.values()) {
      try {
        await s.session.appWs.client.close();
      } catch {
        // ignore close errors
      }
    }
  };

  if (!electedFounder) {
    console.error(
      `FATAL: no conductor bound for any household member — cannot elect a founder. ` +
        `Checked: ${HOUSEHOLD_MEMBERS.map(m => m.humanId).join(', ')}.`,
    );
    await closeAll();
    process.exit(1);
  }

  const founder = electedFounder;
  const founderSession = sessions.get(founder.humanId)!;

  if (founder.humanId !== HOUSEHOLD_MEMBERS[0].humanId) {
    console.warn(
      `[!] founder ${HOUSEHOLD_MEMBERS[0].humanId} unbindable (no conductor session matched — ` +
        `likely a UUID-minted Human from doorway registration rather than the canonical id) — ` +
        `electing ${founder.humanId} as founder instead.`,
    );
  } else {
    console.log(`[=] founder elected: ${founder.humanId} (nominal default, session bound)`);
  }

  const affirmed = new Set<string>([founder.humanId]); // founder is steward-affirmed by create_collective

  // -- 1. Founder creates (or reuses) the household collective -------------
  let collectiveCid: string;

  // Projection probe: if STORAGE_URL or DOORWAY_URL is set, check whether the
  // collective already exists before minting a second one.
  const probeBase = process.env.STORAGE_URL ?? process.env.DOORWAY_URL ?? '';
  let probedCid: string | null = null;
  if (probeBase) {
    probedCid = await resolveExistingCollectiveCid(probeBase);
  }

  // Founder-chain fallback: the storage/doorway probe above can miss even
  // when a collective genuinely exists (its stamp lags cross-conductor DHT
  // gossip — see resolveFounderChainCollectiveCid docstring). Ask the
  // founder's OWN conductor before minting a fresh one.
  let founderChainOrphanCount = 0;
  if (!probedCid) {
    const founderChain = await resolveFounderChainCollectiveCid(founderSession);
    if (founderChain) {
      probedCid = founderChain.cid;
      founderChainOrphanCount = founderChain.orphanCount;
    }
  }

  if (probedCid) {
    collectiveCid = probedCid;
    console.log(`[=] reusing existing household collective ${collectiveCid}`);
    if (founderChainOrphanCount > 0) {
      console.warn(
        `[!] founder has ${founderChainOrphanCount} orphaned household collective(s) from prior partial runs — reused the newest`,
      );
    }
  } else {
    try {
      const collectiveHash = await founderSession.session.appWs.callZome({
        cell_id: founderSession.session.imagodeiCell,
        zome_name: 'imagodei',
        fn_name: 'create_collective',
        payload: {
          charter: buildHouseholdCharter(),
          display_name: 'Dowell Family',
          // NOTE: create_entry mints a fresh ActionHash per run — re-running WITHOUT the
          // projection probe creates a SECOND collective (old memberships/custody orphan
          // under the prior cid; the SQL slug-merge re-stamps to the newest). The probe
          // above is what makes CI re-runs convergent; keep it healthy.
          salt,
        },
      });
      collectiveCid = `collective:${encodeActionHash(collectiveHash)}`;
      console.log(`[+] collective created: ${collectiveCid}`);
    } catch (err) {
      console.error(`FATAL: create_collective failed: ${err instanceof Error ? err.message : err}`);
      await closeAll();
      process.exit(1);
    }
  }

  // -- 2. Invite + affirm each non-founder member -------------------------
  // Backoff schedule for the issuer-stewardship DHT-settle race below
  // (total ~4-5 min across 6 retries) — see the affirm_membership comment.
  const STEWARD_SETTLE_BACKOFFS_MS = [10_000, 20_000, 30_000, 45_000, 60_000, 90_000];
  // Filter by humanId (not slice(1)) — the elected founder isn't guaranteed to
  // be HOUSEHOLD_MEMBERS[0] (see founder-election above).
  for (const member of HOUSEHOLD_MEMBERS.filter(m => m.humanId !== founder.humanId)) {
    const memberSession = sessions.get(member.humanId);
    if (!memberSession) {
      console.error(`  [X] ${member.humanId}: no conductor bound — cannot affirm`);
      continue;
    }

    // Founder issues a single-use signed invite on HIS conductor.
    let token: unknown;
    try {
      const nonce = createHash('sha256')
        .update(`${noncePrefix}:${member.humanId}`)
        .digest('hex')
        .slice(0, 32);
      token = await founderSession.session.appWs.callZome({
        cell_id: founderSession.session.imagodeiCell,
        zome_name: 'imagodei',
        fn_name: 'issue_household_invite',
        payload: {
          collective_cid: collectiveCid,
          role: member.role,
          expires_at_micros: (Date.now() + 24 * 3600 * 1000) * 1000,
          nonce,
        },
      });
    } catch (err) {
      console.error(
        `  [X] issue_household_invite for ${member.humanId}: ${err instanceof Error ? err.message : err}`,
      );
      continue;
    }

    // Member affirms on THEIR conductor (their agent authors the Membership).
    //
    // DHT-settle retry: affirm_membership's issuer-stewardship check
    // (require_caller_is_steward_of, qahal_coordinator.rs:490-507) runs
    // list_memberships_for_collective LOCALLY on the AFFIRMING member's
    // conductor — a get_links + get pair that only sees what's already
    // gossiped there. The founder's Steward Membership is minted seconds
    // earlier on the FOUNDER's conductor, so the very first affirm can
    // legitimately race DHT propagation and fail with "caller is not a
    // current Steward of <cid>" (genesis #1382/#1383: both "1/3 affirmed").
    // Mirrors findMemberSessions' DepMissingFromDht settle-retry above —
    // only THIS error class is retried; every other failure (expired token,
    // bad signature, already-consumed nonce) fails/settles on the first try.
    let affirmSucceeded = false;
    let affirmAlreadyConsumed = false;
    let affirmLastErr: string | undefined;
    for (let attempt = 1; attempt <= STEWARD_SETTLE_BACKOFFS_MS.length + 1; attempt++) {
      try {
        await memberSession.session.appWs.callZome({
          cell_id: memberSession.session.imagodeiCell,
          zome_name: 'imagodei',
          fn_name: 'affirm_membership',
          payload: { token },
        });
        affirmSucceeded = true;
        affirmLastErr = undefined;
        break;
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        affirmLastErr = msg;
        if (msg.includes('already consumed')) {
          affirmAlreadyConsumed = true;
          break;
        }
        const backoffMs = STEWARD_SETTLE_BACKOFFS_MS[attempt - 1];
        if (msg.includes('not a current Steward of') && backoffMs !== undefined) {
          console.warn(
            `  [~] issuer stewardship not yet visible on ${member.humanId}'s conductor — DHT settle retry ${attempt}/${STEWARD_SETTLE_BACKOFFS_MS.length} (waiting ${backoffMs / 1000}s)`,
          );
          await new Promise(r => setTimeout(r, backoffMs));
          continue;
        }
        break; // not a settle race, or retries exhausted
      }
    }

    if (affirmSucceeded) {
      affirmed.add(member.humanId);
      console.log(`  [+] ${member.humanId} affirmed (${member.role})`);
    } else if (affirmAlreadyConsumed) {
      // Idempotent re-run — the nonce anchor already exists, so the prior
      // affirmation stands.
      affirmed.add(member.humanId);
      console.log(`  [=] ${member.humanId} already affirmed (idempotent re-run)`);
    } else {
      console.error(`  [X] affirm_membership for ${member.humanId}: ${affirmLastErr}`);
    }
  }
  console.log('');

  // -- 2b. Settle-wait for the formation projection -----------------------
  // The collective_cid stamp + participant list lag the conductor-side
  // affirmations via DHT → gossip → projection_reconcile. Wait until the
  // projection reflects the affirmed members so downstream reads (a2o
  // qahal-formation) don't observe an unstamped cid or a short participant
  // list (genesis #1182 Cluster B). Soft — never aborts seeding.
  if (probeBase) {
    const settled = await waitForHouseholdProjected(probeBase, [...affirmed]);
    console.log(
      settled
        ? `[+] household projection settled (${affirmed.size} member(s) reflected)`
        : `[!] household projection not yet settled — continuing (reconcile is async)`,
    );
    console.log('');
  }

  // -- 3. Parental StewardshipGrant over the minor (james) ----------------
  const minor = HOUSEHOLD_MEMBERS.find(m => m.minor);
  if (minor && affirmed.has(minor.humanId)) {
    try {
      // NOTE: grant ids embed a timestamp — a re-run mints a new grant entry (bounded noise, latest-wins semantics downstream).
      await founderSession.session.appWs.callZome({
        cell_id: founderSession.session.imagodeiCell,
        zome_name: 'imagodei',
        fn_name: 'create_stewardship_grant',
        payload: {
          subject_id: minor.humanId,
          // Must be a member of imagodei_integrity's AUTHORITY_BASES
          // (stewardship.rs) — 'parental' was never in that vocabulary; the
          // guardian-of-minor basis is 'minor_guardianship'.
          authority_basis: 'minor_guardianship',
          evidence_hash: null,
          verified_by: 'household-formation-ceremony',
          content_filtering: true,
          time_limits: true,
          feature_restrictions: true,
          activity_monitoring: true,
          policy_delegation: false,
          delegatable: false,
          expires_in_days: 365,
          review_in_days: 90,
        },
      });
      console.log(
        `[+] stewardship grant: ${founder.humanId} -> ${minor.humanId} (minor_guardianship)`
      );
    } catch (err) {
      console.warn(
        `[!] create_stewardship_grant (non-fatal): ${err instanceof Error ? err.message : err}`,
      );
    }
  }
  console.log('');

  // -- 4. Custody mesh: every affirmed provider → every other affirmed ----
  let custodyOk = 0;
  let custodyFail = 0;

  const haveBlob = blobHash.length > 0 && blobSizeBytes > 0;
  if (!haveBlob) {
    console.warn('[!] CONTENT_BLOB_HASH / CONTENT_BLOB_SIZE_BYTES unset — skipping custody mesh.');
  } else {
    const affirmedMembers = HOUSEHOLD_MEMBERS.filter(m => affirmed.has(m.humanId));
    for (const provider of affirmedMembers) {
      const providerSession = sessions.get(provider.humanId);
      if (!providerSession) {
        continue;
      }
      const lamadCell = providerSession.session.lamadCell;
      if (!lamadCell) {
        console.warn(`  [!] ${provider.humanId}: no lamad cell — custody soft-skipped`);
        continue;
      }
      for (const receiver of affirmedMembers) {
        if (receiver.humanId === provider.humanId) {
          continue;
        }
        // Stage-2 resolution (cached per host; falls back to Stage-1 with a
        // loud warning) — fake ids here silently disarm the custody sweep.
        const peerIds: CustodyPeerIds = {
          provider: await resolvePeerId(provider.humanId, provider.archetype),
          receiver: await resolvePeerId(receiver.humanId, receiver.archetype),
        };
        const input = buildCeremonyCustodyInput({
          providerHumanId: provider.humanId,
          providerArchetype: provider.archetype,
          receiverHumanId: receiver.humanId,
          receiverArchetype: receiver.archetype,
          blobHash,
          blobSizeBytes,
          collectiveCid,
        }, peerIds);
        try {
          await providerSession.session.appWs.callZome({
            cell_id: lamadCell,
            zome_name: 'content_store',
            fn_name: 'create_rea_commitment',
            payload: input,
          });
          custodyOk += 1;
          console.log(`  [+] custody ${provider.humanId} -> ${receiver.humanId}`);
        } catch (err) {
          const msg = err instanceof Error ? err.message : String(err);
          if (msg.includes('already') || msg.includes('duplicate')) {
            // NOTE: the zome has no duplicate guard today — this branch is forward-compat;
            // convergence is provided by the SQL projection upserting on the deterministic id.
            custodyOk += 1;
            console.log(`  [=] custody ${provider.humanId} -> ${receiver.humanId} (idempotent)`);
          } else {
            custodyFail += 1;
            console.error(`  [X] custody ${provider.humanId} -> ${receiver.humanId}: ${msg}`);
          }
        }
      }
    }
  }
  console.log('');

  // -- 5. Write result artifact + exit ------------------------------------
  const partial = affirmed.size < HOUSEHOLD_MEMBERS.length || custodyFail > 0;
  const report = {
    schemaVersion: 1,
    seededAt: new Date().toISOString(),
    script: 'seed-household-formation',
    collectiveCid,
    affirmed: [...affirmed],
    custodyOk,
    custodyFail,
    partial,
  };

  const resultsFile = resolve(dirname(fileURLToPath(import.meta.url)), '..', SEED_RESULTS_FILE);
  try {
    writeFileSync(resultsFile, JSON.stringify(report, null, 2));
  } catch (e) {
    console.error(`WARN: could not write ${resultsFile}:`, e);
  }

  console.log(
    `=== Results: ${affirmed.size}/${HOUSEHOLD_MEMBERS.length} affirmed, custody ok=${custodyOk} fail=${custodyFail} ===`,
  );

  await closeAll();

  process.exit(partial ? 2 : 0);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(async err => {
    console.error('FATAL:', err instanceof Error ? err.stack ?? err.message : err);
    // Best-effort close any sessions that were opened before the fatal error.
    for (const s of activeSessions.values()) {
      try {
        await s.session.appWs.client.close();
      } catch {
        // ignore close errors during fatal cleanup
      }
    }
    process.exit(1);
  });
}
