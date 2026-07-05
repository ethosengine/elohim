import { deriveSyncProgress } from './sync-status.service';

import type { P2PStatusView } from '../../generated/p2p-status-view';

/** Minimal P2PStatusView factory — only the fields deriveSyncProgress reads matter. */
function status(overrides: Partial<P2PStatusView>): P2PStatusView {
  return {
    peerId: 'test',
    listenAddresses: [],
    connectedPeers: 0,
    bootstrapNodes: [],
    syncDocuments: 0,
    natStatus: 'unknown',
    relayReservations: 0,
    announceAddresses: [],
    relayMode: 'disabled',
    replication: { pending: 0, completed: 0, failed: 0, caughtUp: false },
    syncPaused: false,
    dedupUniqueLen: 0,
    dedupTotalSeen: 0,
    reconcilePassesTotal: 0,
    kicksFiredTotal: 0,
    placementGapsEmittedTotal: 0,
    ...overrides,
  } as P2PStatusView;
}

describe('deriveSyncProgress', () => {
  it('reports unreachable (never "done") when the snapshot is null', () => {
    const p = deriveSyncProgress(null);
    expect(p.phase).toBe('unreachable');
    expect(p.percent).toBeNull();
  });

  it('reports waiting when nothing is discovered and no peers are connected', () => {
    const p = deriveSyncProgress(status({ connectedPeers: 0 }));
    expect(p.phase).toBe('waiting');
    expect(p.label).toMatch(/waiting/i);
  });

  it('prefers the pull rollup and computes a have/total percent while syncing', () => {
    const p = deriveSyncProgress(
      status({
        connectedPeers: 3,
        pull: { total: 210, fetched: 142, pending: 60, failed: 8, caughtUp: false },
      })
    );
    expect(p.phase).toBe('syncing');
    expect(p.fetched).toBe(142);
    expect(p.total).toBe(210);
    expect(p.percent).toBe(68); // round(142/210*100)
    expect(p.peers).toBe(3);
    expect(p.label).toContain('142/210');
    expect(p.label).toContain('3 peers');
  });

  it('reports ready with 100% when the pull stream is caught up', () => {
    const p = deriveSyncProgress(
      status({ connectedPeers: 4, pull: { total: 50, fetched: 50, pending: 0, failed: 0, caughtUp: true } })
    );
    expect(p.phase).toBe('ready');
    expect(p.percent).toBe(100);
  });

  it('falls back to replication (total = completed+pending+failed) when pull is absent', () => {
    const p = deriveSyncProgress(
      status({
        connectedPeers: 2,
        pull: null,
        replication: { completed: 30, pending: 10, failed: 0, caughtUp: false },
      })
    );
    expect(p.phase).toBe('syncing');
    expect(p.fetched).toBe(30);
    expect(p.total).toBe(40);
    expect(p.percent).toBe(75);
  });

  it('a null pull never reads as caught up on its own — replication drives readiness', () => {
    // pull absent + replication caughtUp ⇒ ready (the present stream is authoritative)
    const ready = deriveSyncProgress(
      status({ connectedPeers: 2, pull: null, replication: { completed: 5, pending: 0, failed: 0, caughtUp: true } })
    );
    expect(ready.phase).toBe('ready');

    // pull absent + replication NOT caughtUp with peers ⇒ syncing, not ready
    const syncing = deriveSyncProgress(
      status({ connectedPeers: 2, pull: null, replication: { completed: 5, pending: 3, failed: 0, caughtUp: false } })
    );
    expect(syncing.phase).toBe('syncing');
  });

  it('is indeterminate (null percent) when syncing with an unknown total but live peers', () => {
    const p = deriveSyncProgress(
      status({ connectedPeers: 5, pull: { total: 0, fetched: 0, pending: 0, failed: 0, caughtUp: false } })
    );
    expect(p.phase).toBe('syncing');
    expect(p.percent).toBeNull();
  });
});
