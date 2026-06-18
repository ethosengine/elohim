import { describe, it, expect } from 'vitest';
import { deriveContentAnchor } from '../seed-sqlite.js';

describe('deriveContentAnchor', () => {
  it('produces a CIDv1 starting with bafk for content with a body', async () => {
    const anchor = await deriveContentAnchor({
      id: 'test-content',
      title: 'Test Content',
      contentBody: '# Hello\n\nThis is content.',
    });
    expect(anchor).toMatch(/^bafk/);
  });

  it('produces a CIDv1 starting with bafk for content with a blobCid fallback', async () => {
    const anchor = await deriveContentAnchor({
      id: 'blob-content',
      title: 'Blob Content',
      blobCid: 'bafkreiabcdef1234',
    });
    expect(anchor).toMatch(/^bafk/);
  });

  it('produces a CIDv1 starting with bafk for content with a blobHash fallback', async () => {
    const anchor = await deriveContentAnchor({
      id: 'blob-hash-content',
      title: 'Blob Hash Content',
      blobHash: 'sha256-abcdef1234',
    });
    expect(anchor).toMatch(/^bafk/);
  });

  it('produces a CIDv1 starting with bafk for placeholder content (id+title only)', async () => {
    const anchor = await deriveContentAnchor({
      id: 'placeholder',
      title: 'Placeholder',
    });
    expect(anchor).toMatch(/^bafk/);
  });

  it('is deterministic — same input produces same anchor', async () => {
    const input = { id: 'stable', title: 'Stable', contentBody: 'content text' };
    const a = await deriveContentAnchor(input);
    const b = await deriveContentAnchor(input);
    expect(a).toBe(b);
  });

  it('produces distinct anchors for distinct content bodies', async () => {
    const a = await deriveContentAnchor({ id: 'x', title: 'X', contentBody: 'alpha' });
    const b = await deriveContentAnchor({ id: 'x', title: 'X', contentBody: 'beta' });
    expect(a).not.toBe(b);
  });

  it('prefers contentBody over blobCid when both present', async () => {
    const withBody = await deriveContentAnchor({
      id: 'x',
      title: 'X',
      contentBody: 'body bytes',
      blobCid: 'bafkreiabcdef1234',
    });
    const bodyOnly = await deriveContentAnchor({
      id: 'x',
      title: 'X',
      contentBody: 'body bytes',
    });
    expect(withBody).toBe(bodyOnly);
  });

  it('prefers blobCid over blobHash when contentBody absent', async () => {
    const withCid = await deriveContentAnchor({
      id: 'x',
      title: 'X',
      blobCid: 'bafkreiabcdef1234',
      blobHash: 'sha256-deadbeef',
    });
    const cidOnly = await deriveContentAnchor({
      id: 'x',
      title: 'X',
      blobCid: 'bafkreiabcdef1234',
    });
    expect(withCid).toBe(cidOnly);
  });
});
