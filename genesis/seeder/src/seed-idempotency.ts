/**
 * Idempotent seeding — decide, per row, whether a seed run writes anything.
 *
 * A seed run must recognise content that is already current and leave it
 * untouched: no overwrite, no re-signing. Only a genuinely changed row is
 * updated, and only a genuinely changed reach is re-notarized. The identity both
 * writers (pipeline seeder, steward publish) agree on is `canonicalSeedHash`
 * (seed-hash.ts), stored under `metadata.seedHash`.
 *
 * Pure: the caller does the HTTP (one GET per row, then the writes these
 * decisions call for). Everything here is testable without a peer.
 */
import type { CreateContentInput } from './generated/create-content-input.js';
import { SEED_HASH_KEY, canonicalSeedHash, type SeedHashInput } from './seed-hash.js';

/** The fields of a stored row (GET /db/content/{id}, ContentView) the decision reads. */
export interface StoredContentRow {
  id: string;
  title: string;
  description?: string | null;
  contentType: string;
  contentFormat?: string | null;
  contentBody?: string | null;
  blobHash?: string | null;
  reach?: string | null;
  tags?: string[] | null;
  metadata?: unknown;
}

/** What one read of the peer said about a row. */
export type StoredLookup =
  | { kind: 'found'; row: StoredContentRow }
  /** 404 — absent, or present but below the external serving floor (no provenance). */
  | { kind: 'missing' }
  /**
   * Present but not readable without an identity (403: reach gate above
   * public, device policy, prerequisite gate). The seeder has no identity to
   * read it with, so it cannot know whether the row is current — it leaves it.
   */
  | { kind: 'unreadable'; status: number; requiredReach?: string }
  /** Read failed (network, 5xx after retries, malformed body). */
  | { kind: 'error'; message: string };

export type SeedDecision =
  | { kind: 'insert'; id: string; seedHash: string }
  | { kind: 'unchanged'; id: string; seedHash: string; via: 'seedHash' | 'legacy' }
  | {
      kind: 'update';
      id: string;
      seedHash: string;
      /** Why the stored row is not current. */
      via: 'seedHash' | 'legacy';
      /** Non-notarized PATCH body (diesel path — never re-signs). */
      patch: ContentFieldPatch;
      /** Authored reach, set ONLY when it differs from the stored reach (conductor PATCH — re-signs). */
      reachPatch?: string;
      /** Authored differences a PATCH cannot carry (logged, never written). */
      unpatchable: string[];
    }
  | { kind: 'unverified'; id: string; seedHash: string; status: number; requiredReach?: string }
  /**
   * The repository differs from the row, but the row's canonical head was EARNED:
   * a steward's agent declared it. A seed write can never outrank that (storage
   * refuses to let the staging scaffold override an earned canonical and heals
   * back), so writing would only leave orphan versions on the chain and loop on
   * every run. The change belongs to the steward: publish it with steward-publish.
   */
  | { kind: 'stewarded'; id: string; seedHash: string; earnedBy?: string }
  | { kind: 'failed'; id: string; seedHash: string; message: string };

/** What `GET /db/content/{id}/head` says about who holds the row's canonical head. */
export interface StoredHead {
  earned?: boolean;
  earnedBy?: string;
}

/**
 * A changed row whose head a steward earned is left to the steward. Every
 * other decision passes through unchanged.
 */
export function deferToSteward(decision: SeedDecision, head: StoredHead | undefined): SeedDecision {
  if (decision.kind !== 'update' || head?.earned !== true) return decision;
  return { kind: 'stewarded', id: decision.id, seedHash: decision.seedHash, earnedBy: head.earnedBy };
}

/**
 * PATCH /db/content/{id} body for the authored fields that storage writes
 * diesel-direct. It must NEVER carry `reach`, `blobHash` or `serverBlobHash`:
 * any of those routes the whole PATCH through the conductor
 * (`patch_needs_conductor`), which re-notarizes the entry — and on that path the
 * body and tags are not written at all.
 */
export interface ContentFieldPatch {
  title: string;
  description?: string;
  contentBody?: string;
  contentFormat?: string;
  tags: string[];
  /** Shallow-merged by storage: only these keys are written. */
  metadata: Record<string, unknown>;
}

/** Metadata keys a deploy (not the seeder) owns; never written by a seed PATCH. */
const DEPLOY_OWNED_METADATA_KEYS = new Set(['serverBlobHash']);

function asObject(value: unknown): Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}

function emptyToNull(value: string | null | undefined): string | null {
  return value === undefined || value === null || value === '' ? null : value;
}

/** The hash-relevant fields of the input the seeder would write (before stamping). */
export function seedHashInputOf(input: CreateContentInput): SeedHashInput {
  return {
    id: input.id,
    title: input.title,
    description: input.description ?? null,
    contentType: input.contentType ?? 'concept',
    contentFormat: input.contentFormat ?? null,
    contentBody: input.contentBody ?? null,
    blobHash: input.blobHash ?? null,
    reach: input.reach ?? null,
    tags: input.tags ?? [],
    metadata: asObject(input.metadata),
  };
}

/**
 * The seed hash of the input a writer is about to write — the value stored as
 * `metadata.seedHash`. Both writers (pipeline seeder, steward publish) call this
 * on the input built by content-input.ts, before stamping.
 */
export function seedHashFor(input: CreateContentInput): string {
  return canonicalSeedHash(seedHashInputOf(input));
}

/**
 * Project a stored row onto what the seeder could have written, for a row that
 * carries no seedHash (seeded before the hash existed). Normalizations — each
 * one is a difference the seeder could not have authored and cannot repair:
 *
 * - `description` / `contentBody`: '' ⇄ null. A row re-published through the
 *   conductor bootstrap stores `description: ""` for an absent one.
 * - `blobHash`: when the repository authors none, a stored hash was linked by a
 *   deploy (stageSpaBlob) and is not the seeder's to compare.
 * - `metadata`: only the keys the repository authors are compared. A PATCH
 *   shallow-merges, so the seeder could never remove a key another writer
 *   (deploy, P2P resolution) added; comparing them would call the row changed
 *   forever.
 */
export function legacyHashInputOf(stored: StoredContentRow, authored: CreateContentInput): SeedHashInput {
  const storedMeta = asObject(stored.metadata);
  const authoredMeta = asObject(authored.metadata);
  const metadata: Record<string, unknown> = {};
  for (const key of Object.keys(authoredMeta)) {
    if (key in storedMeta) metadata[key] = storedMeta[key];
  }
  return {
    id: stored.id,
    title: stored.title,
    description: emptyToNull(stored.description),
    contentType: stored.contentType,
    contentFormat: stored.contentFormat ?? null,
    contentBody: emptyToNull(stored.contentBody),
    blobHash: authored.blobHash ? stored.blobHash ?? null : null,
    reach: stored.reach ?? null,
    tags: stored.tags ?? [],
    metadata,
  };
}

/** The input with `metadata.seedHash` stamped — what an INSERT writes. */
export function withSeedHash(input: CreateContentInput, seedHash: string): CreateContentInput {
  return { ...input, metadata: { ...asObject(input.metadata), [SEED_HASH_KEY]: seedHash } };
}

function fieldPatchFor(input: CreateContentInput, seedHash: string): ContentFieldPatch {
  const metadata: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(asObject(input.metadata))) {
    if (!DEPLOY_OWNED_METADATA_KEYS.has(key)) metadata[key] = value;
  }
  metadata[SEED_HASH_KEY] = seedHash;
  const patch: ContentFieldPatch = { title: input.title, tags: input.tags ?? [], metadata };
  if (input.description !== undefined) patch.description = input.description;
  if (input.contentBody !== undefined) patch.contentBody = input.contentBody;
  if (input.contentFormat !== undefined) patch.contentFormat = input.contentFormat;
  return patch;
}

/** Authored differences no PATCH can write (contentType is not patchable; blobHash is deploy/conductor-owned). */
function unpatchableDiffs(input: CreateContentInput, stored: StoredContentRow): string[] {
  const diffs: string[] = [];
  if ((input.contentType ?? 'concept') !== stored.contentType) diffs.push('contentType');
  if (input.blobHash && input.blobHash !== (stored.blobHash ?? undefined)) diffs.push('blobHash');
  return diffs;
}

/**
 * Decide what a seed run does with one row.
 *
 *   missing                      → insert (stamped with seedHash)
 *   found, seedHash === h        → unchanged — no write of any kind
 *   found, seedHash !== h        → update
 *   found, no seedHash, legacy
 *     projection hashes to h     → unchanged — NOT stamped (stamping is a write)
 *   found, no seedHash, differs  → update
 *   unreadable (403…)            → unverified — left untouched
 *   read error                   → failed — nothing written
 */
export function decideSeedAction(input: CreateContentInput, lookup: StoredLookup): SeedDecision {
  const seedHash = seedHashFor(input);
  const id = input.id;

  switch (lookup.kind) {
    case 'missing':
      return { kind: 'insert', id, seedHash };
    case 'unreadable':
      return { kind: 'unverified', id, seedHash, status: lookup.status, requiredReach: lookup.requiredReach };
    case 'error':
      return { kind: 'failed', id, seedHash, message: lookup.message };
    case 'found': {
      const stored = lookup.row;
      const storedHash = asObject(stored.metadata)[SEED_HASH_KEY];
      let via: 'seedHash' | 'legacy';
      if (typeof storedHash === 'string') {
        if (storedHash === seedHash) return { kind: 'unchanged', id, seedHash, via: 'seedHash' };
        via = 'seedHash';
      } else {
        if (canonicalSeedHash(legacyHashInputOf(stored, input)) === seedHash) {
          return { kind: 'unchanged', id, seedHash, via: 'legacy' };
        }
        via = 'legacy';
      }
      const authoredReach = input.reach ?? undefined;
      const reachPatch = authoredReach && authoredReach !== (stored.reach ?? undefined) ? authoredReach : undefined;
      return {
        kind: 'update',
        id,
        seedHash,
        via,
        patch: fieldPatchFor(input, seedHash),
        reachPatch,
        unpatchable: unpatchableDiffs(input, stored),
      };
    }
  }
}

/**
 * Rows whose reach a seed run re-notarizes: only UPDATED rows whose stored
 * reach differs from the authored one. An inserted row was written with the
 * authored reach; an unchanged or unverified row gets nothing.
 */
export function reachReconcileTargets(decisions: SeedDecision[]): Array<{ id: string; reach: string }> {
  const targets: Array<{ id: string; reach: string }> = [];
  for (const d of decisions) {
    if (d.kind === 'update' && d.reachPatch) targets.push({ id: d.id, reach: d.reachPatch });
  }
  return targets;
}

/** Per-run counts for one phase (content or paths). */
export interface SeedTally {
  inserted: number;
  updated: number;
  unchanged: number;
  /** Present but unreadable, or present-but-invisible (bulk insert skipped it). Never written. */
  unverified: number;
  /** Changed in the repository, but a steward earned the head: left for steward-publish. */
  stewarded: number;
  failed: number;
  /** Every PATCH request issued (field updates + reach re-notarizations). */
  patches: number;
}

export function emptyTally(): SeedTally {
  return { inserted: 0, updated: 0, unchanged: 0, unverified: 0, stewarded: 0, failed: 0, patches: 0 };
}

export function addTally(a: SeedTally, b: SeedTally): SeedTally {
  return {
    inserted: a.inserted + b.inserted,
    updated: a.updated + b.updated,
    unchanged: a.unchanged + b.unchanged,
    unverified: a.unverified + b.unverified,
    stewarded: a.stewarded + b.stewarded,
    failed: a.failed + b.failed,
    patches: a.patches + b.patches,
  };
}

/** The one summary line a run reports per phase — what a reader greps for. */
export function formatTally(label: string, t: SeedTally): string {
  return (
    `Seed summary [${label}]: inserted=${t.inserted} updated=${t.updated} ` +
    `unchanged=${t.unchanged} unverified=${t.unverified} stewarded=${t.stewarded} failed=${t.failed} ` +
    `patches=${t.patches}`
  );
}

// ============================================================================
// Content-graph edges
// ============================================================================

/** The fields of a stored edge (GET /db/relationships, RelationshipView) the decision reads. */
export interface StoredRelationshipRow {
  id: string;
  sourceId: string;
  targetId: string;
  relationshipType: string;
  confidence: number;
  inferenceSource: string;
  reach: string;
  metadata?: unknown;
}

export type RelationshipDecision =
  | { kind: 'insert' }
  /** `reachDiffers`: the stored edge's reach is not the source atom's — the HTTP route cannot carry reach. */
  | { kind: 'unchanged'; reachDiffers: boolean }
  | { kind: 'update'; reachDiffers: boolean; changed: string[] };

/** Key-order-independent JSON for comparing edge metadata. */
function stableJson(v: unknown): string {
  if (v === undefined || v === null) return 'null';
  if (Array.isArray(v)) return `[${v.map(stableJson).join(',')}]`;
  if (typeof v === 'object') {
    const o = v as Record<string, unknown>;
    return `{${Object.keys(o).sort().map(k => `${JSON.stringify(k)}:${stableJson(o[k])}`).join(',')}}`;
  }
  return JSON.stringify(v);
}

/**
 * Decide whether an authored edge needs a write. Storage holds one edge per
 * (source, target, type) and its bulk upsert rewrites confidence,
 * inference_source and metadata_json — so exactly those decide `update`.
 * Reach is compared and REPORTED, never a write trigger: the bulk route can
 * neither set nor update it, so rewriting on a reach difference would write on
 * every run and change nothing.
 */
export function decideRelationshipAction(
  desired: { confidence: number; inferenceSource: string; reach: string; metadata?: unknown },
  stored: StoredRelationshipRow | undefined,
): RelationshipDecision {
  if (!stored) return { kind: 'insert' };
  const changed: string[] = [];
  if (Math.abs((stored.confidence ?? 1) - desired.confidence) > 1e-6) changed.push('confidence');
  if (stored.inferenceSource !== desired.inferenceSource) changed.push('inferenceSource');
  if (stableJson(stored.metadata) !== stableJson(desired.metadata)) changed.push('metadata');
  const reachDiffers = stored.reach !== desired.reach;
  return changed.length === 0 ? { kind: 'unchanged', reachDiffers } : { kind: 'update', reachDiffers, changed };
}
