import { describe, it, expect } from 'vitest';
import { recordBlobUploadOutcome } from '../blob-upload-result.js';

describe('recordBlobUploadOutcome', () => {
  it('records when blob already existed on server', () => {
    const map = new Map<string, string>();
    const recorded = recordBlobUploadOutcome(map, '/images/logo.png', {
      kind: 'existed',
      hash: 'sha256-abc',
    });
    expect(recorded).toBe(true);
    expect(map.get('/images/logo.png')).toBe('sha256-abc');
  });

  it('records when blob was just uploaded successfully', () => {
    const map = new Map<string, string>();
    const recorded = recordBlobUploadOutcome(map, 'app-x', {
      kind: 'uploaded',
      hash: 'sha256-def',
    });
    expect(recorded).toBe(true);
    expect(map.get('app-x')).toBe('sha256-def');
  });

  it('does NOT record when upload failed — prevents dangling blobHash', () => {
    // Regression: before this fix, the seeder unconditionally stamped the hash
    // mapping, so a failed thumbnail upload still produced a path content row
    // with `blobHash: sha256-1f3ed518…` pointing at bytes that never landed.
    // The browser's <img> request to `/blob/<hash>` then 404'd against alpha.
    const map = new Map<string, string>();
    const recorded = recordBlobUploadOutcome(map, '/images/logo.png', {
      kind: 'failed',
    });
    expect(recorded).toBe(false);
    expect(map.has('/images/logo.png')).toBe(false);
  });

  it('does not overwrite a prior mapping when current outcome is failure', () => {
    const map = new Map<string, string>();
    map.set('/images/logo.png', 'sha256-prior');
    recordBlobUploadOutcome(map, '/images/logo.png', { kind: 'failed' });
    expect(map.get('/images/logo.png')).toBe('sha256-prior');
  });
});
