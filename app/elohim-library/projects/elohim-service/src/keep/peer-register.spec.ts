/**
 * The register's job is to be strictly additive. Each test below pins one way
 * the naive version of this change would have been WORSE than not making it.
 */

import { describe, expect, it, vi } from 'vitest';

import {
  ConfiguredDoorwayResolver,
  gatewayCandidates,
} from '../client/doorway-address-resolver.js';

import { FederationPeerResolver, peerOf, trustOf } from './peer-register.js';

import type { FederationDoorwayRow } from './peer-register.js';

const SEED = 'https://doorway-alpha.elohim.host';

/** A DHT-sourced row: the whole signature quartet present. */
function signedRow(over: Partial<FederationDoorwayRow> = {}): FederationDoorwayRow {
  return {
    id: 'alpha-elohim-host',
    url: 'https://doorway-alpha.elohim.host',
    identity_root: 'idroot-alpha',
    signing_key: 'signkey-alpha',
    record_serial: 7,
    record_signature: [1, 2, 3],
    endpoints: [
      { service: 'gateway', url: 'https://doorway-alpha.elohim.host', priority: 0, ttl_secs: 300 },
    ],
    region: 'us-east',
    tier: 'Anchor',
    capabilities: ['gateway'],
    status: 'online',
    ...over,
  };
}

/** A gossip-merged row: the merge path sets the whole quartet to None. */
function gossipRow(over: Partial<FederationDoorwayRow> = {}): FederationDoorwayRow {
  return {
    id: 'beta-elohim-host',
    url: 'https://doorway-beta.elohim.host',
    identity_root: null,
    signing_key: null,
    record_serial: null,
    record_signature: null,
    endpoints: null,
    region: null,
    tier: 'Federated',
    capabilities: [],
    status: 'online',
    ...over,
  };
}

function respondWith(rows: readonly FederationDoorwayRow[]): typeof fetch {
  return vi.fn(
    async () =>
      // eslint-disable-next-line @typescript-eslint/no-unsafe-type-assertion -- minimal Response stand-in
      ({
        ok: true,
        status: 200,
        json: async () => ({ doorways: rows, self_id: 'alpha-elohim-host', total: rows.length }),
      }) as unknown as Response
  ) as unknown as typeof fetch;
}

function configured(): ConfiguredDoorwayResolver {
  return new ConfiguredDoorwayResolver([
    { identity: SEED, primaryUrl: SEED, fallbackUrls: ['https://fallback.elohim.host'] },
  ]);
}

function build(rows: readonly FederationDoorwayRow[], fetchImpl?: typeof fetch) {
  return new FederationPeerResolver({
    seedUrl: SEED,
    fallback: configured(),
    fetchImpl: fetchImpl ?? respondWith(rows),
  });
}

describe('trust labelling', () => {
  it('requires the whole quartet, not just a signature', () => {
    expect(trustOf(signedRow())).toBe('dht-notarized');
    expect(trustOf(gossipRow())).toBe('unsigned-gossip');
    // A signature with no key to check it against is not a weaker proof.
    expect(trustOf(signedRow({ signing_key: null }))).toBe('unsigned-gossip');
    // ...nor is one with no serial to order it.
    expect(trustOf(signedRow({ record_serial: null }))).toBe('unsigned-gossip');
    // ...nor an empty signature that is merely PRESENT.
    expect(trustOf(signedRow({ record_signature: [] }))).toBe('unsigned-gossip');
  });

  it('never claims a peer accepts our session', () => {
    // The bearer provably does not cross doorways (HS256 default + the
    // foreign-kid EdDSA refusal), so this is false even for a signed anchor.
    expect(peerOf(signedRow()).acceptsMySession).toBe(false);
    expect(peerOf(gossipRow()).acceptsMySession).toBe(false);
  });
});

describe('FederationPeerResolver — degradation', () => {
  it('NEVER throws for an unknown identity, where the configured resolver does', () => {
    // The behaviour being replaced: a miss is an exception, and the interceptor
    // rethrows it into the request stream.
    expect(() => configured().resolve('who-is-this')).toThrow();

    const resolver = build([signedRow()]);
    const resolution = resolver.resolve('who-is-this');
    expect(resolution.identity).toBe('who-is-this');
    expect(gatewayCandidates(resolution)).toEqual([SEED]);
  });

  it('degrades to the configured resolver when the fetch fails', async () => {
    const failing = vi.fn(async () => {
      throw new Error('offline');
    }) as unknown as typeof fetch;
    const resolver = build([], failing);
    await resolver.warm();

    expect(resolver.isWarm).toBe(false);
    // Byte-identical to today: the configured resolution, fallbacks and all.
    expect(gatewayCandidates(resolver.resolve(SEED))).toEqual([
      SEED,
      'https://fallback.elohim.host',
    ]);
  });

  it('does not cache an empty federation list over the fallback', async () => {
    const resolver = build([]);
    await resolver.warm();
    expect(resolver.isWarm).toBe(false);
    expect(resolver.peers()).toEqual([]);
  });

  it('does not block a request on the network', () => {
    let settle: (() => void) | undefined;
    const hanging = vi.fn(
      async () =>
        new Promise<Response>(resolve => {
          settle = () => {
            resolve({ ok: false, status: 503 } as unknown as Response);
          };
        })
    ) as unknown as typeof fetch;

    const resolver = build([], hanging);
    // Answers while the load is still in flight — no await, no latency added.
    expect(gatewayCandidates(resolver.resolve(SEED))[0]).toBe(SEED);
    expect(settle).toBeTypeOf('function');
    settle?.();
  });
});

describe('FederationPeerResolver — the alias problem', () => {
  it('resolves by raw URL, by id, and by identity_root alike', async () => {
    const resolver = build([signedRow()]);
    await resolver.warm();

    // The interceptor asks with `doorwayIdentity ?? doorwayUrl ?? origin`, and
    // doorwayIdentity is set in no environment file — so in practice it asks
    // with a URL while the row keys itself by identity_root.
    for (const alias of [
      'idroot-alpha',
      'signkey-alpha',
      'alpha-elohim-host',
      'https://doorway-alpha.elohim.host',
      'https://doorway-alpha.elohim.host/',
    ]) {
      const resolution = resolver.resolve(alias);
      expect(resolution.source, `alias ${alias} missed the register`).toBe('registration');
      expect(resolution.identity).toBe('idroot-alpha');
    }
  });

  it('labels a gossip row as gossip and still offers it as an address', async () => {
    const resolver = build([signedRow(), gossipRow()]);
    await resolver.warm();

    const resolution = resolver.resolve('https://doorway-beta.elohim.host');
    expect(resolution.source).toBe('federation-gossip');
    // No declared endpoints on a gossip row — the url becomes the gateway.
    expect(gatewayCandidates(resolution)).toEqual(['https://doorway-beta.elohim.host']);

    const beta = resolver.peers().find(p => p.id === 'beta-elohim-host');
    expect(beta?.trust).toBe('unsigned-gossip');
    expect(resolver.peers().find(p => p.id === 'alpha-elohim-host')?.trust).toBe('dht-notarized');
  });

  it('does not let a later gossip row displace a signed row on a shared alias', async () => {
    // Same URL claimed twice: signed first, then gossiped.
    const resolver = build([
      signedRow(),
      gossipRow({ id: 'impostor', url: 'https://doorway-alpha.elohim.host' }),
    ]);
    await resolver.warm();

    expect(resolver.resolve('https://doorway-alpha.elohim.host').source).toBe('registration');
  });

  it('skips malformed rows rather than indexing them', async () => {
    const resolver = build([{ id: '', url: '' } as FederationDoorwayRow, signedRow()]);
    await resolver.warm();
    expect(resolver.peers()).toHaveLength(1);
  });
});

describe('FederationPeerResolver — load discipline', () => {
  it('shares one request across concurrent warms', async () => {
    const impl = respondWith([signedRow()]);
    const resolver = build([], impl);
    await Promise.all([resolver.warm(), resolver.warm(), resolver.warm()]);
    expect(impl).toHaveBeenCalledTimes(1);
  });

  it('reuses a warm register instead of refetching per request', async () => {
    const impl = respondWith([signedRow()]);
    const resolver = build([], impl);
    await resolver.warm();
    resolver.resolve('idroot-alpha');
    resolver.resolve('idroot-alpha');
    expect(impl).toHaveBeenCalledTimes(1);
  });
});
