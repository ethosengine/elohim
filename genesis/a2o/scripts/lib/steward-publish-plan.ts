/**
 * Steward publish — the pure decisions, kept apart from the I/O so they can be tested
 * without a mesh.
 *
 * A steward publishes commons content from their own peer (scripts/steward-publish.ts).
 * Three rules decide what happens to each item, and all three live here:
 *
 *   1. Commons fence. Only an item whose AUTHORED reach is `commons` may be published.
 *      Anything else (an intimate love map, an ungraded row) is refused before any write.
 *   2. One builder, one hash. The input comes from the seeder's own builders
 *      (genesis/seeder/src/content-input.ts) and the row carries
 *      `metadata.seedHash = seedHashFor(input)` (genesis/seeder/src/seed-idempotency.ts),
 *      so a later seed run recognises the publish as current. Neither is re-implemented.
 *   3. Carry or refuse. The storage PATCH re-notarizes through the conductor, and the
 *      conductor's `update_content` carries only title, description, metadata (merged),
 *      reach and the blob pointer. Stamping the seed hash onto a row whose body, type,
 *      format or tags did NOT move would make the row claim bytes it does not hold — and
 *      a later seed run would then trust that claim and leave the stale row alone — so
 *      such an item is `blocked` and nothing is written.
 */
import { SEED_HASH_KEY } from '../../../seeder/src/seed-hash.js';
import { seedHashFor } from '../../../seeder/src/seed-idempotency.js';

import type { CreateContentInput } from '../../../seeder/src/generated/create-content-input.js';

/** The reach a steward publish is allowed to carry. */
export const COMMONS = 'commons';

/** A content row as `GET /db/content/{id}` returns it (only the fields read here). */
export interface ExistingRow {
  id: string;
  title?: string | null;
  description?: string | null;
  contentType?: string | null;
  contentFormat?: string | null;
  contentBody?: string | null;
  blobHash?: string | null;
  reach?: string | null;
  tags?: string[] | null;
  metadata?: unknown;
  dhtAnchorHash?: string | null;
}

/** The authored reach of a repo JSON: `reach`, else the legacy `visibility` key. */
export function authoredReach(json: { reach?: unknown; visibility?: unknown }): string | undefined {
  const r = json.reach ?? json.visibility;
  return typeof r === 'string' && r.trim() !== '' ? r : undefined;
}

/** Why an item may not be published at all, or undefined when it may. */
export function commonsFenceRefusal(
  id: string,
  json: { reach?: unknown; visibility?: unknown }
): string | undefined {
  const reach = authoredReach(json);
  if (reach === COMMONS) return undefined;
  return reach
    ? `${id} is authored at reach "${reach}", not "${COMMONS}"`
    : `${id} carries no authored reach (an ungraded row is not commons)`;
}

function asObject(value: unknown): Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}

/** '' and null/undefined are the same absence (the conductor bootstrap stores '' for none). */
function norm(value: string | null | undefined): string | null {
  return value === undefined || value === null || value === '' ? null : value;
}

function sameTags(a: string[] | null | undefined, b: string[] | null | undefined): boolean {
  const sa = [...new Set(a ?? [])].sort();
  const sb = [...new Set(b ?? [])].sort();
  return sa.length === sb.length && sa.every((t, i) => t === sb[i]);
}

/**
 * Authored fields the publish would need to move but the substrate cannot carry onto
 * this existing row. Empty means every authored field either already matches or rides
 * the PATCH.
 *
 * - An ANCHORED row re-notarizes through the zome's `update_content`, which carries the
 *   body and format (and every peer adopts them from the verified entry), but not the
 *   content type. Tags reach the entry and this peer's row, but other peers do not adopt
 *   them yet, so a tag change is still refused rather than landing on one peer only.
 * - An UNANCHORED row is bootstrapped through `create_content`, which does take the
 *   body, format and tags from the PATCH, but still takes the content type from the
 *   existing row.
 * - A description can be replaced but never cleared (a null PATCH field means "leave").
 */
export function uncarriedFields(input: CreateContentInput, row: ExistingRow): string[] {
  const out: string[] = [];
  const anchored = Boolean(row.dhtAnchorHash);
  if ((row.contentType ?? null) !== (input.contentType ?? 'concept')) out.push('contentType');
  if (anchored && !sameTags(row.tags, input.tags)) out.push('tags');
  // Adopting peers deliberately never take a reach change from a head (storage's
  // non-narrowing guard), so it would land on this peer only.
  if (anchored && (row.reach ?? null) !== (input.reach ?? null)) {
    out.push('reach (other peers do not adopt a reach change)');
  }
  if (norm(row.description) !== null && norm(input.description) === null) {
    out.push('description (cannot be cleared)');
  }
  return out;
}

export type ItemAction =
  /** The row already carries this seed hash and its head is the earned canonical head. */
  | 'unchanged'
  /** The row already carries this seed hash; only the earned declaration is missing. */
  | 'declare'
  /** No row on the steward's peer: insert, PATCH (bootstraps the entry), declare. */
  | 'create'
  /** A row exists with a different (or no) seed hash, and every change can be carried. */
  | 'update'
  /** A row exists and some authored change cannot be carried; nothing is written. */
  | 'blocked';

export interface ItemPlan {
  action: ItemAction;
  seedHash: string;
  /** For `blocked`: the fields the substrate cannot carry. */
  uncarried: string[];
}

/**
 * Decide what to do for one commons-fenced item.
 *
 * @param earnedHead the earned canonical head the steward's conductor elects for the
 *   id, or undefined when there is none (or the winner is not earned). Only consulted
 *   when the seed hash already matches, to tell `unchanged` from `declare`.
 */
export function planItem(
  input: CreateContentInput,
  row: ExistingRow | undefined,
  earnedHead?: string
): ItemPlan {
  const seedHash = seedHashFor(input);
  if (!row) return { action: 'create', seedHash, uncarried: [] };
  // A SQL seed marker is not a native version. Bootstrap unanchored rows
  // through the existing PATCH/create_content path before declaring a head.
  if (row.dhtAnchorHash && asObject(row.metadata)[SEED_HASH_KEY] === seedHash) {
    const declared = Boolean(row.dhtAnchorHash) && earnedHead === row.dhtAnchorHash;
    return { action: declared ? 'unchanged' : 'declare', seedHash, uncarried: [] };
  }
  const uncarried = uncarriedFields(input, row);
  return uncarried.length > 0
    ? { action: 'blocked', seedHash, uncarried }
    : { action: 'update', seedHash, uncarried };
}

/**
 * The PATCH body a publish sends. `reach` is always present, which routes the PATCH
 * through the conductor (`patch_needs_conductor`), so the change is ONE witnessed head
 * rather than a per-peer diesel write. `metadata` carries the seed hash and is merged
 * into the notarized entry's metadata.
 */
export function publishPatch(input: CreateContentInput, seedHash: string): Record<string, unknown> {
  const patch: Record<string, unknown> = {
    title: input.title,
    reach: input.reach,
    tags: input.tags ?? [],
    metadata: { ...asObject(input.metadata), [SEED_HASH_KEY]: seedHash },
  };
  if (norm(input.description) !== null) patch.description = input.description;
  if (input.contentBody !== undefined) patch.contentBody = input.contentBody;
  if (input.contentFormat !== undefined) patch.contentFormat = input.contentFormat;
  if (input.blobHash) patch.blobHash = input.blobHash;
  return patch;
}

/**
 * Fields of a freshly re-read row that disagree with what the publish meant to write.
 * Empty means the publish landed as intended on the reading peer.
 */
export function landedMismatches(
  input: CreateContentInput,
  seedHash: string,
  row: ExistingRow
): string[] {
  const out: string[] = [];
  if (asObject(row.metadata)[SEED_HASH_KEY] !== seedHash) out.push('metadata.seedHash');
  if ((row.reach ?? null) !== (input.reach ?? null)) out.push('reach');
  if ((row.title ?? null) !== input.title) out.push('title');
  if (norm(row.contentBody) !== norm(input.contentBody)) out.push('contentBody');
  if (input.blobHash && row.blobHash !== input.blobHash) out.push('blobHash');
  if (!row.dhtAnchorHash) out.push('dhtAnchorHash');
  return out;
}
