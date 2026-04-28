/**
 * Blob-upload outcome bookkeeping.
 *
 * The seeder pre-uploads blob bytes (HTML5 apps, path thumbnails) before
 * stamping content rows with `blobHash` references. If we record the hash
 * mapping for an upload that didn't actually land, downstream rows get
 * stamped with a dangling `blobHash` — the UI then tries to fetch
 * `/blob/<hash>` and 404s.
 *
 * The rule is simple: record the mapping only when the bytes are confirmed
 * present on the server (either pre-existing or just-uploaded). On failure,
 * skip the mapping so consumers fall back to "no thumbnail" instead of
 * publishing a broken reference.
 */

export type BlobUploadOutcome =
  | { kind: 'existed'; hash: string }
  | { kind: 'uploaded'; hash: string }
  | { kind: 'failed' };

/**
 * Apply the upload outcome to the lookup map, recording the mapping only on
 * confirmed-present outcomes. Returns true if the mapping was recorded.
 */
export function recordBlobUploadOutcome(
  map: Map<string, string>,
  key: string,
  outcome: BlobUploadOutcome
): boolean {
  if (outcome.kind === 'failed') {
    return false;
  }
  map.set(key, outcome.hash);
  return true;
}
