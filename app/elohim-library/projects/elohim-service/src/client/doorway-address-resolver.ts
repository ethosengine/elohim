/**
 * Framework-neutral doorway identity → candidate-address resolution.
 *
 * The resolver consumes already-verified protocol projections. It does not
 * mint identity, verify DHT authority, or own endpoint truth. Config is the
 * first adapter; a registration/pkarr adapter can replace it without changing
 * request retry or stickiness semantics.
 */

/**
 * Where an address came from — and, for a federation row, how much its own
 * claim about itself is worth.
 *
 * `registration` is a signed record: identity_root + signing_key + serial +
 * signature, sourced from the infrastructure DHT. `federation-gossip` is the
 * same endpoint's OTHER half — rows merged in over HTTP with all four of those
 * set to None (`doorway-service/src/routes/federation.rs`). They are separate
 * words because calling an unsigned row `config` claims an operator chose it,
 * and calling it `registration` claims a signature that is not there. It is a
 * fine address to try a plain GET against and a bad one to send a credential
 * to, and only a distinct label lets a caller act on the difference.
 */
export type DoorwayResolutionSource = 'config' | 'registration' | 'federation-gossip' | 'pkarr';

export interface DoorwayEndpoint {
  /** gateway, bootstrap, or signal */
  service: string;
  url: string;
  /** Lower values are preferred; ties retain signed record order. */
  priority: number;
  /** Cache freshness, not a liveness claim. */
  ttlSecs?: number;
}

export interface DoorwayResolution {
  identity: string;
  endpoints: readonly DoorwayEndpoint[];
  source: DoorwayResolutionSource;
}

export interface DoorwayAddressResolver {
  resolve(identity: string): DoorwayResolution | Promise<DoorwayResolution>;
}

export interface ConfiguredDoorwayAddress {
  identity: string;
  primaryUrl: string;
  fallbackUrls?: readonly string[];
}

/**
 * DNS/config-backed resolver used until signed registration lookup is wired.
 */
export class ConfiguredDoorwayResolver implements DoorwayAddressResolver {
  private readonly configured = new Map<string, DoorwayResolution>();

  constructor(addresses: readonly ConfiguredDoorwayAddress[]) {
    for (const address of addresses) {
      const urls = dedupeUrls([address.primaryUrl, ...(address.fallbackUrls ?? [])]);
      this.configured.set(address.identity, {
        identity: address.identity,
        source: 'config',
        endpoints: urls.map((url, priority) => ({
          service: 'gateway',
          url,
          priority,
        })),
      });
    }
  }

  resolve(identity: string): DoorwayResolution {
    const resolution = this.configured.get(identity);
    if (!resolution) {
      throw new Error(`No configured addresses for doorway identity "${identity}"`);
    }
    return resolution;
  }
}

/** Return ordered, normalized gateway candidates from a resolution. */
export function gatewayCandidates(resolution: DoorwayResolution): string[] {
  return dedupeUrls(
    resolution.endpoints
      .map((endpoint, index) => ({ endpoint, index }))
      .filter(({ endpoint }) => endpoint.service === 'gateway')
      .sort(
        (left, right) =>
          left.endpoint.priority - right.endpoint.priority || left.index - right.index
      )
      .map(({ endpoint }) => endpoint.url)
  );
}

export function normalizeDoorwayUrl(url: string): string {
  let end = url.length;
  while (end > 0 && url[end - 1] === '/') {
    end -= 1;
  }
  return url.slice(0, end);
}

function dedupeUrls(urls: readonly string[]): string[] {
  return Array.from(new Set(urls.map(normalizeDoorwayUrl).filter(Boolean)));
}
