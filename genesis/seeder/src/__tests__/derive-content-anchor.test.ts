import { describe, it, expect } from 'vitest';
import { deriveContentAnchor } from '../seed-sqlite.js';
import { CID_A } from './fixture-cids.js';

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
      blobCid: CID_A,
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
      blobCid: CID_A,
    });
    const bodyOnly = await deriveContentAnchor({
      id: 'x',
      title: 'X',
      contentBody: 'body bytes',
    });
    expect(withBody).toBe(bodyOnly);
  });

  it('derives the byte-profile anchor (bafkr4i…) for blob-backed content', async () => {
    const anchor = await deriveContentAnchor({ id: 'x', title: 'X', blobCid: CID_A });
    expect(anchor).toMatch(/^bafkr4i[a-z2-7]{52}$/);
  });
});
