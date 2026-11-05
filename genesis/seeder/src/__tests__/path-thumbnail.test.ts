import { describe, it, expect } from 'vitest';
import { applyPathThumbnail } from '../path-thumbnail.js';
import type { CreateContentInput } from '../generated/create-content-input.js';
import { CID_A } from './fixture-cids.js';

const basePathInput = (): CreateContentInput => ({
  id: 'elohim-protocol',
  title: 'Elohim Protocol',
  contentType: 'path',
  contentFormat: 'epr-composite',
  contentBody: JSON.stringify({ sections: [] }),
  metadata: { difficulty: 'beginner' },
});

describe('applyPathThumbnail', () => {
  it('returns input unchanged when thumbnailCid is undefined', () => {
    const input = basePathInput();
    const result = applyPathThumbnail(input, undefined);
    expect(result.blobCid).toBeUndefined();
    expect((result.metadata as Record<string, unknown>).thumbnailUrl).toBeUndefined();
  });

  it('sets blobCid on the path content row so the thumbnail blob is linked via the first-class column', () => {
    const input = basePathInput();
    const hash = CID_A;
    const result = applyPathThumbnail(input, hash);
    // This is the regression under test: without setting blobCid, the uploaded
    // thumbnail blob is orphaned from any blob_cid-column reconciliation.
    expect(result.blobCid).toBe(hash);
  });

  it('also sets metadata.thumbnailUrl to /blob/<hash> for UI rendering', () => {
    const input = basePathInput();
    const hash = CID_A;
    const result = applyPathThumbnail(input, hash);
    const meta = result.metadata as Record<string, unknown>;
    expect(meta.thumbnailUrl).toBe(`/blob/${hash}`);
  });

  it('preserves other metadata fields when injecting the thumbnail reference', () => {
    const input = basePathInput();
    const hash = CID_A;
    const result = applyPathThumbnail(input, hash);
    const meta = result.metadata as Record<string, unknown>;
    expect(meta.difficulty).toBe('beginner');
  });

  it('does not mutate the input object', () => {
    const input = basePathInput();
    const hash = CID_A;
    applyPathThumbnail(input, hash);
    expect(input.blobCid).toBeUndefined();
    expect((input.metadata as Record<string, unknown>).thumbnailUrl).toBeUndefined();
  });

  it('handles input with no metadata by creating a fresh metadata object', () => {
    const input: CreateContentInput = {
      id: 'x',
      title: 'X',
      contentType: 'path',
      contentFormat: 'epr-composite',
    };
    const hash = CID_A;
    const result = applyPathThumbnail(input, hash);
    expect(result.blobCid).toBe(hash);
    expect((result.metadata as Record<string, unknown>).thumbnailUrl).toBe(`/blob/${hash}`);
  });
});
