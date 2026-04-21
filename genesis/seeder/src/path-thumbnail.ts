import type { CreateContentInput } from './generated/create-content-input.js';

/**
 * Link an uploaded thumbnail blob to a path's content input.
 *
 * Sets two references on the returned (copy of the) input:
 *   - `blobHash` — the first-class column that storage-level reconciliation
 *     scans. Without this, the uploaded blob has no row-column reference and
 *     looks like an orphan to blob-reachability queries and compaction.
 *   - `metadata.thumbnailUrl = "/blob/<hash>"` — the shape the Angular UI
 *     resolves via resolveBlobUrl to produce an <img src> URL.
 *
 * Both references point at the same blob. The metadata URL keeps UI behavior
 * unchanged; the column reference restores storage-level linkage.
 *
 * Paths store their structured body inline in `contentBody` (as stringified
 * epr-composite sections), so `blobHash` is otherwise unused on path rows —
 * no downstream consumer interprets it as the path body.
 *
 * Returns the input unchanged if `thumbnailHash` is undefined.
 */
export function applyPathThumbnail(
  input: CreateContentInput,
  thumbnailHash: string | undefined
): CreateContentInput {
  if (!thumbnailHash) {
    return input;
  }

  const existingMetadata =
    typeof input.metadata === 'object' && input.metadata !== null
      ? (input.metadata as Record<string, unknown>)
      : {};

  return {
    ...input,
    blobHash: thumbnailHash,
    metadata: {
      ...existingMetadata,
      thumbnailUrl: `/blob/${thumbnailHash}`,
    },
  };
}
