import { describe, expect, it } from 'vitest';
import type { AccountPackageInputView } from '@elohim/storage-client';
import { resolveTargetUrl, stableHash } from '../seed-accounts.js';

function pkg(humanId: string): AccountPackageInputView {
  return {
    identity: { humanId, displayName: humanId } as AccountPackageInputView['identity'],
    content: [],
    relationships: [],
    stewardship: [],
    collectives: [],
  } as unknown as AccountPackageInputView;
}

describe('stableHash', () => {
  it('is deterministic across calls', () => {
    expect(stableHash('human-matthew-manager')).toBe(stableHash('human-matthew-manager'));
    expect(stableHash('human-adam-firstman')).toBe(stableHash('human-adam-firstman'));
  });

  it('differs across distinct inputs', () => {
    expect(stableHash('human-matthew-manager')).not.toBe(stableHash('human-adam-firstman'));
  });

  it('returns an unsigned 32-bit integer', () => {
    const h = stableHash('x'.repeat(64));
    expect(h).toBeGreaterThanOrEqual(0);
    expect(h).toBeLessThanOrEqual(0xffffffff);
  });
});

describe('resolveTargetUrl', () => {
  const peers = ['http://matthew:8090', 'http://adam:8090'];

  it('returns a peer from the list', () => {
    const t = resolveTargetUrl(pkg('human-matthew-manager'), peers);
    expect(peers).toContain(t);
  });

  it('is stable for the same humanId', () => {
    const a = resolveTargetUrl(pkg('human-matthew-manager'), peers);
    const b = resolveTargetUrl(pkg('human-matthew-manager'), peers);
    expect(a).toBe(b);
  });

  it('distributes the five legacy humans across two peers non-trivially', () => {
    const legacy = ['human-matthew-manager', 'human-adam-firstman', 'human-jessica-spouse', 'human-pete-pastor', 'human-frank-farmer', 'human-timothy-tutor'];
    const counts = new Map<string, number>([[peers[0], 0], [peers[1], 0]]);
    for (const id of legacy) {
      const t = resolveTargetUrl(pkg(id), peers);
      counts.set(t, (counts.get(t) ?? 0) + 1);
    }
    // Both peers should receive at least one account out of six.
    expect(counts.get(peers[0])!).toBeGreaterThan(0);
    expect(counts.get(peers[1])!).toBeGreaterThan(0);
  });

  it('collapses to single peer when only one is configured', () => {
    expect(resolveTargetUrl(pkg('human-matthew-manager'), ['http://one:8090'])).toBe('http://one:8090');
  });

  it('throws when peer list is empty', () => {
    expect(() => resolveTargetUrl(pkg('human-matthew-manager'), [])).toThrow(/must not be empty/);
  });
});
