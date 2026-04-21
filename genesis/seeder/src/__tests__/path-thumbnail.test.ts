import { describe, it, expect } from 'vitest';
import { applyPathThumbnail } from '../path-thumbnail.js';
import type { CreateContentInput } from '../generated/create-content-input.js';

const basePathInput = (): CreateContentInput => ({
  id: 'elohim-protocol',
  title: 'Elohim Protocol',
  contentType: 'path',
  contentFormat: 'epr-composite',
  contentBody: JSON.stringify({ sections: [] }),
  metadata: { difficulty: 'beginner' },
});

describe('applyPathThumbnail', () => {
  it('returns input unchanged when thumbnailHash is undefined', () => {
    const input = basePathInput();
    const result = applyPathThumbnail(input, undefined);
    expect(result.blobHash).toBeUndefined();
    expect((result.metadata as Record<string, unknown>).thumbnailUrl).toBeUndefined();
  });

  it('sets blobHash on the path content row so the thumbnail blob is linked via the first-class column', () => {
    const input = basePathInput();
    const hash = 'sha256-c5dcc240abcdef1234567890';
    const result = applyPathThumbnail(input, hash);
    // This is the regression under test: without setting blobHash, the uploaded
    // thumbnail blob is orphaned from any blob_hash-column reconciliation.
    expect(result.blobHash).toBe(hash);
  });

  it('also sets metadata.thumbnailUrl to /blob/<hash> for UI rendering', () => {
    const input = basePathInput();
    const hash = 'sha256-c5dcc240abcdef1234567890';
    const result = applyPathThumbnail(input, hash);
    const meta = result.metadata as Record<string, unknown>;
    expect(meta.thumbnailUrl).toBe(`/blob/${hash}`);
  });

  it('preserves other metadata fields when injecting the thumbnail reference', () => {
    const input = basePathInput();
    const hash = 'sha256-c5dcc240abcdef1234567890';
    const result = applyPathThumbnail(input, hash);
    const meta = result.metadata as Record<string, unknown>;
    expect(meta.difficulty).toBe('beginner');
  });

  it('does not mutate the input object', () => {
    const input = basePathInput();
    const hash = 'sha256-c5dcc240abcdef1234567890';
    applyPathThumbnail(input, hash);
    expect(input.blobHash).toBeUndefined();
    expect((input.metadata as Record<string, unknown>).thumbnailUrl).toBeUndefined();
  });

  it('handles input with no metadata by creating a fresh metadata object', () => {
    const input: CreateContentInput = {
      id: 'x',
      title: 'X',
      contentType: 'path',
      contentFormat: 'epr-composite',
    };
    const hash = 'sha256-c5dcc240abcdef1234567890';
    const result = applyPathThumbnail(input, hash);
    expect(result.blobHash).toBe(hash);
    expect((result.metadata as Record<string, unknown>).thumbnailUrl).toBe(`/blob/${hash}`);
  });
});
