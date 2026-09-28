/**
 * Canonical seed hash — the one identity both writers of a content row agree on.
 *
 * Two writers put the same content rows onto a peer: the pipeline seeder (from the
 * repository's JSON) and a steward publishing from their own peer. For a later seed
 * run to recognise a steward's publish as already current — and leave it alone
 * instead of overwriting or re-signing it — both must compute the SAME hash over the
 * SAME authored fields. This module is that contract; neither writer may hash on its
 * own.
 *
 * Hashed: the authored meaning of the row. Not hashed: anything derived or volatile
 * (timestamps, schemaVersion, contentSizeBytes, anchors, the stored hash itself).
 */
import { createHash } from 'node:crypto';

/** Metadata key under which a writer records the hash of the row it authored. */
export const SEED_HASH_KEY = 'seedHash';

/** The authored fields of a content row, as either writer builds them. */
export interface SeedHashInput {
  id: string;
  title: string;
  description?: string | null;
  contentType: string;
  contentFormat?: string | null;
  contentBody?: string | null;
  blobHash?: string | null;
  reach?: string | null;
  tags?: string[] | null;
  metadata?: Record<string, unknown> | null;
}

/** JSON with object keys sorted at every depth, so equal meaning serializes equally. */
export function stableStringify(value: unknown): string {
  if (value === null || typeof value !== 'object') return JSON.stringify(value ?? null);
  if (Array.isArray(value)) return `[${value.map(stableStringify).join(',')}]`;
  const entries = Object.entries(value as Record<string, unknown>)
    .filter(([, v]) => v !== undefined)
    .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
  return `{${entries.map(([k, v]) => `${JSON.stringify(k)}:${stableStringify(v)}`).join(',')}}`;
}

/**
 * A structured body (a quiz, a path's section tree) is JSON carried as a string. Two writers can
 * serialize the same structure with different key order, so it is hashed by its parsed value.
 * Anything that is not a JSON object or array is hashed as the text it is.
 */
function canonicalBody(body: string | null | undefined): unknown {
  if (body == null) return null;
  const head = body.trimStart()[0];
  if (head !== '{' && head !== '[') return body;
  try {
    return { json: JSON.parse(body) as unknown };
  } catch {
    return body;
  }
}

/** `sha256-<hex>` over the authored fields. Tags are a set; metadata excludes the hash itself. */
export function canonicalSeedHash(input: SeedHashInput): string {
  const { [SEED_HASH_KEY]: _ignored, ...metadata } = input.metadata ?? {};
  const authored = {
    id: input.id,
    title: input.title,
    description: input.description ?? null,
    contentType: input.contentType,
    contentFormat: input.contentFormat ?? null,
    contentBody: canonicalBody(input.contentBody),
    blobHash: input.blobHash ?? null,
    reach: input.reach ?? null,
    tags: [...new Set(input.tags ?? [])].sort(),
    metadata,
  };
  return 'sha256-' + createHash('sha256').update(stableStringify(authored), 'utf8').digest('hex');
}
