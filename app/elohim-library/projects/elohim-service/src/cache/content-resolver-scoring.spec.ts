/**
 * Delivery Peer Scoring Tests
 *
 * Tests for multi-peer scoring logic used to rank delivery peers
 * by network proximity, capability, cache warmth, and recency.
 */

import {
  type DeliveryPeer,
  scorePeer,
  scorePeersForContent,
  extractIpFromMultiaddr,
} from './content-resolver';

// ============================================================================
// Test Fixtures
// ============================================================================

function createPeer(overrides?: Partial<DeliveryPeer>): DeliveryPeer {
  return {
    peerId: 'peer-1',
    multiaddrs: ['/ip4/192.168.1.50/tcp/9876'],
    network: 'wan',
    capabilities: ['serves_extracted', 'serves_compressed'],
    lastSeen: Date.now(),
    httpPort: 8090,
    ...overrides,
  };
}

// ============================================================================
// extractIpFromMultiaddr
// ============================================================================

describe('extractIpFromMultiaddr', () => {
  it('should parse IPv4 from standard multiaddr', () => {
    expect(extractIpFromMultiaddr('/ip4/192.168.1.50/tcp/9876')).toBe('192.168.1.50');
  });

  it('should parse IPv4 from multiaddr with udp', () => {
    expect(extractIpFromMultiaddr('/ip4/10.0.0.1/udp/4001/quic-v1')).toBe('10.0.0.1');
  });

  it('should return empty string for IPv6 multiaddr', () => {
    expect(extractIpFromMultiaddr('/ip6/::1/tcp/9876')).toBe('');
  });

  it('should return empty string for empty string', () => {
    expect(extractIpFromMultiaddr('')).toBe('');
  });

  it('should return empty string for malformed multiaddr', () => {
    expect(extractIpFromMultiaddr('/dns4/example.com/tcp/443')).toBe('');
  });

  it('should parse first IPv4 segment only', () => {
    expect(extractIpFromMultiaddr('/ip4/127.0.0.1/tcp/4001/p2p/QmFoo')).toBe('127.0.0.1');
  });
});

// ============================================================================
// scorePeer
// ============================================================================

describe('scorePeer', () => {
  it('should give LAN peers the highest network score', () => {
    const lan = scorePeer(createPeer({ network: 'lan' }), 'hash-1');
    const wan = scorePeer(createPeer({ network: 'wan' }), 'hash-1');
    const relay = scorePeer(createPeer({ network: 'relay' }), 'hash-1');

    expect(lan.score).toBeGreaterThan(wan.score);
    expect(wan.score).toBeGreaterThan(relay.score);
  });

  it('should give LAN + warm higher score than WAN + warm', () => {
    const lanWarm = scorePeer(
      createPeer({ network: 'lan', capabilities: ['serves_extracted', 'warm:hash-1'] }),
      'hash-1',
    );
    const wanWarm = scorePeer(
      createPeer({ network: 'wan', capabilities: ['serves_extracted', 'warm:hash-1'] }),
      'hash-1',
    );

    expect(lanWarm.score).toBeGreaterThan(wanWarm.score);
  });

  it('should give WAN + extracted higher score than WAN + compressed only', () => {
    const wanExtracted = scorePeer(
      createPeer({ network: 'wan', capabilities: ['serves_extracted'] }),
      'hash-1',
    );
    const wanCompressed = scorePeer(
      createPeer({ network: 'wan', capabilities: ['serves_compressed'] }),
      'hash-1',
    );

    expect(wanExtracted.score).toBeGreaterThan(wanCompressed.score);
  });

  it('should add warm bonus only for matching content hash', () => {
    const warm = scorePeer(
      createPeer({ capabilities: ['serves_extracted', 'warm:hash-1'] }),
      'hash-1',
    );
    const notWarm = scorePeer(
      createPeer({ capabilities: ['serves_extracted', 'warm:hash-other'] }),
      'hash-1',
    );

    expect(warm.warm).toBe(true);
    expect(notWarm.warm).toBe(false);
    expect(warm.score).toBeGreaterThan(notWarm.score);
  });

  it('should give stale peers (>90s) no recency bonus', () => {
    const fresh = scorePeer(
      createPeer({ lastSeen: Date.now() - 5000, capabilities: [] }),
      'hash-1',
    );
    const stale = scorePeer(
      createPeer({ lastSeen: Date.now() - 100000, capabilities: [] }),
      'hash-1',
    );

    // Fresh gets +100 recency, stale gets +0
    expect(fresh.score - stale.score).toBe(100);
  });

  it('should give medium recency bonus for 30-90s age', () => {
    const medium = scorePeer(
      createPeer({ lastSeen: Date.now() - 60000, capabilities: [] }),
      'hash-1',
    );
    const stale = scorePeer(
      createPeer({ lastSeen: Date.now() - 100000, capabilities: [] }),
      'hash-1',
    );

    // Medium gets +50, stale gets +0
    expect(medium.score - stale.score).toBe(50);
  });

  it('should construct correct baseUrl from multiaddr + httpPort', () => {
    const result = scorePeer(
      createPeer({
        multiaddrs: ['/ip4/192.168.1.100/tcp/9876'],
        httpPort: 8090,
      }),
      'hash-1',
    );

    expect(result.baseUrl).toBe('http://192.168.1.100:8090');
  });

  it('should return empty baseUrl when no IPv4 multiaddr', () => {
    const result = scorePeer(
      createPeer({
        multiaddrs: ['/ip6/::1/tcp/9876'],
      }),
      'hash-1',
    );

    expect(result.baseUrl).toBe('');
  });

  it('should return empty baseUrl when multiaddrs is empty', () => {
    const result = scorePeer(createPeer({ multiaddrs: [] }), 'hash-1');
    expect(result.baseUrl).toBe('');
  });

  it('should correctly set servesExtracted and servesCompressed flags', () => {
    const both = scorePeer(
      createPeer({ capabilities: ['serves_extracted', 'serves_compressed'] }),
      'hash-1',
    );
    expect(both.servesExtracted).toBe(true);
    expect(both.servesCompressed).toBe(true);

    const neither = scorePeer(createPeer({ capabilities: [] }), 'hash-1');
    expect(neither.servesExtracted).toBe(false);
    expect(neither.servesCompressed).toBe(false);
  });

  it('should include peerId and network in result', () => {
    const result = scorePeer(createPeer({ peerId: 'my-peer', network: 'lan' }), 'hash-1');
    expect(result.peerId).toBe('my-peer');
    expect(result.network).toBe('lan');
  });
});

// ============================================================================
// scorePeersForContent
// ============================================================================

describe('scorePeersForContent', () => {
  it('should return empty array for empty peer list', () => {
    expect(scorePeersForContent([], 'hash-1')).toEqual([]);
  });

  it('should sort peers by score descending', () => {
    const peers: DeliveryPeer[] = [
      createPeer({ peerId: 'relay-peer', network: 'relay', capabilities: [] }),
      createPeer({ peerId: 'lan-peer', network: 'lan', capabilities: ['serves_extracted'] }),
      createPeer({ peerId: 'wan-peer', network: 'wan', capabilities: ['serves_extracted'] }),
    ];

    const scored = scorePeersForContent(peers, 'hash-1');
    expect(scored[0].peerId).toBe('lan-peer');
    expect(scored[1].peerId).toBe('wan-peer');
    expect(scored[2].peerId).toBe('relay-peer');
  });

  it('should filter out peers without reachable baseUrl', () => {
    const peers: DeliveryPeer[] = [
      createPeer({ peerId: 'good', multiaddrs: ['/ip4/192.168.1.1/tcp/9876'] }),
      createPeer({ peerId: 'bad-ipv6', multiaddrs: ['/ip6/::1/tcp/9876'] }),
      createPeer({ peerId: 'bad-empty', multiaddrs: [] }),
    ];

    const scored = scorePeersForContent(peers, 'hash-1');
    expect(scored).toHaveLength(1);
    expect(scored[0].peerId).toBe('good');
  });

  it('should rank warm LAN peer above all others', () => {
    const peers: DeliveryPeer[] = [
      createPeer({
        peerId: 'warm-lan',
        network: 'lan',
        capabilities: ['serves_extracted', 'warm:hash-1'],
      }),
      createPeer({
        peerId: 'wan-extracted',
        network: 'wan',
        capabilities: ['serves_extracted'],
      }),
      createPeer({
        peerId: 'wan-warm',
        network: 'wan',
        capabilities: ['serves_extracted', 'warm:hash-1'],
      }),
    ];

    const scored = scorePeersForContent(peers, 'hash-1');
    expect(scored[0].peerId).toBe('warm-lan');
  });
});
