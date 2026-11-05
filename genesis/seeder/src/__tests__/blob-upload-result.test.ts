import { describe, it, expect } from 'vitest';
import { recordBlobUploadOutcome } from '../blob-upload-result.js';
import { CID_A, CID_B, CID_C } from './fixture-cids.js';

describe('recordBlobUploadOutcome', () => {
  it('records when blob already existed on server', () => {
    const map = new Map<string, string>();
    const recorded = recordBlobUploadOutcome(map, '/images/logo.png', {
      kind: 'existed',
      cid: CID_A,
    });
    expect(recorded).toBe(true);
    expect(map.get('/images/logo.png')).toBe(CID_A);
  });

  it('records when blob was just uploaded successfully', () => {
    const map = new Map<string, string>();
    const recorded = recordBlobUploadOutcome(map, 'app-x', {
      kind: 'uploaded',
      cid: CID_B,
    });
    expect(recorded).toBe(true);
    expect(map.get('app-x')).toBe(CID_B);
  });

  it('does NOT record when upload failed — prevents dangling blobCid', () => {
    // Regression: before this fix, the seeder unconditionally stamped the hash
    // mapping, so a failed thumbnail upload still produced a path content row
    // with `blobCid: bafkr4i…` pointing at bytes that never landed.
    // The browser's <img> request to `/blob/<cid>` then 404'd against alpha.
    const map = new Map<string, string>();
    const recorded = recordBlobUploadOutcome(map, '/images/logo.png', {
      kind: 'failed',
    });
    expect(recorded).toBe(false);
    expect(map.has('/images/logo.png')).toBe(false);
  });

  it('does not overwrite a prior mapping when current outcome is failure', () => {
    const map = new Map<string, string>();
    map.set('/images/logo.png', CID_C);
    recordBlobUploadOutcome(map, '/images/logo.png', { kind: 'failed' });
    expect(map.get('/images/logo.png')).toBe(CID_C);
  });
});
