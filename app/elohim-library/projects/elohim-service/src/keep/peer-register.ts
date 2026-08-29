/**
 * The peer register — which doorways exist, and how much each one is worth trusting.
 *
 * This is the `registration` adapter the resolver seam reserved for itself:
 * `doorway-address-resolver.ts` declares `DoorwayResolutionSource` as
 * `'config' | 'registration' | 'pkarr'` and says in its own header that "Config
 * is the first adapter; a registration/pkarr adapter can replace it without
 * changing request retry or stickiness semantics." This is that adapter, and it
 * changes no call site: the interceptor already asks a `DoorwayAddressResolver`
 * and already feeds the answer to `gatewayCandidates`.
 *
 * # Two provenances, and only one of them is signed
 *
 * `GET /api/v1/federation/doorways` deliberately serves rows of two kinds
 * (`doorway-service/src/routes/federation.rs`): rows sourced from the
 * infrastructure DHT carry `identity_root` / `signing_key` / `record_serial` /
 * `record_signature`, and rows merged in from HTTP gossip set all four to
 * `None` with `tier: "Federated"`. That distinction is real and load-bearing,
 * so it is preserved here as a `trust` label rather than flattened away — the
 * existing mapper in doorway-app collapses gossip rows to `source: 'config'`,
 * which reads as "an operator configured this" when nobody did.
 *
 * The label is meant to be USED. An unsigned-gossip row is fine as one more
 * address to try a plain GET against; it must never become an identity claim or
 * a credential destination. (It should not be a JWT trust anchor either — that
 * it currently can is filed as
 * `security-unsigned-gossip-peer-becomes-jwt-trust-anchor`, and it is a
 * server-side defect this client cannot mitigate.)
 *
 * # Three properties this must have, because the naive version takes the app down
 *
 * 1. **It never throws.** `ConfiguredDoorwayResolver.resolve` throws on a miss,
 *    and the interceptor rethrows into the request stream — so a resolver that
 *    throws does not degrade, it kills every doorway-bound request in the app.
 *    Every path here ends at the fallback resolver instead.
 * 2. **It never blocks a request.** `resolve` is allowed to return a promise and
 *    the interceptor handles it, but awaiting a network round trip on the hot
 *    path would put federation availability in front of every `/api/`, `/db/`,
 *    `/blob/`, `/apps/` and `/health` call. A cold register answers from the
 *    fallback IMMEDIATELY and warms in the background, so the worst case is
 *    byte-identical to today and the register is live from the next request on.
 * 3. **It is indexed by every alias a caller might hold.** The interceptor asks
 *    with `doorwayIdentity ?? doorwayUrl ?? effectivePrimary`, and
 *    `doorwayIdentity` is set in no environment file — so in practice it asks
 *    with a URL, while the federation row is keyed by `identity_root`. Indexing
 *    only the canonical identity would miss on every real request.
 */

import { normalizeDoorwayUrl } from '../client/doorway-address-resolver.js';

import type {
  DoorwayAddressResolver,
  DoorwayEndpoint,
  DoorwayResolution,
} from '../client/doorway-address-resolver.js';

/**
 * One row of `GET /api/v1/federation/doorways`.
 *
 * Declared here rather than imported: the Rust `DoorwaySummary`
 * (`doorway-service/src/routes/federation.rs`) carries no `#[derive(TS)]`, and
 * the only existing TypeScript shape for it lives in doorway-app — importing
 * that would point elohim-library at an application. Every field is optional on
 * the wire except `id` and `url`, because the gossip merge path emits `None`
 * for the whole signature quartet.
 */
export interface FederationDoorwayRow {
  id: string;
  url: string;
  identity_root?: string | null;
  signing_key?: string | null;
  record_serial?: number | null;
  record_signature?: number[] | null;
  endpoints?:
    | readonly {
        service: string;
        url: string;
        priority: number;
        ttl_secs?: number | null;
      }[]
    | null;
  region?: string | null;
  tier?: string | null;
  capabilities?: readonly string[] | null;
  status?: string | null;
}

export interface FederationDoorwaysResponseShape {
  doorways?: readonly FederationDoorwayRow[] | null;
  self_id?: string | null;
  total?: number | null;
}

/**
 * WHERE a row came from — provenance, not verification.
 *
 * `dht-sourced` means the row arrived carrying the full quartet (signature, key,
 * serial) that a DHT read path populates and that the gossip merge path leaves
 * as None (`doorway-service/src/routes/federation.rs:112-131`). That difference
 * is real and worth reading.
 *
 * It is NOT a verification result, and the name must not imply one. Nothing in
 * doorway-service verifies `record_signature`: it is constructed or serialized
 * in five places and checked in zero — one of those five
 * (`projection/epr_router.rs:971`) fills it with `vec![1; 64]`. This label is
 * read off an unverified HTTP body, so ANY origin answering
 * `/api/v1/federation/doorways` can mint a `dht-sourced` row by filling three
 * fields with arbitrary bytes.
 *
 * Consequently nothing may be GATED on this value — no admit-list, no content
 * candidate selection, no authority. It is a hint for display and for ordering
 * a fallback, and that is all. The honest fix lives on the server: a
 * `provenance` field on DoorwaySummary, set where the read path is known.
 */
export type PeerTrust = 'dht-sourced' | 'unsigned-gossip';

/** A doorway as the register knows it. */
export interface KeepPeer {
  /** Canonical identity: `identity_root ?? signing_key ?? id`. */
  readonly identity: string;
  readonly id: string;
  readonly url: string;
  readonly trust: PeerTrust;
  readonly region?: string;
  readonly tier?: string;
  readonly capabilities: readonly string[];
  readonly status?: string;
  readonly endpoints: readonly DoorwayEndpoint[];
  /**
   * Whether a session minted at the custodian may be presented here. Always
   * false for a non-custodian: doorways mint HS256 by default and the
   * foreign-kid verification path hard-refuses any non-EdDSA algorithm
   * (`doorway-service/src/auth/jwt.rs`), so a bearer provably does not cross
   * peers. Carried as a field so an app reads the posture instead of assuming
   * one.
   */
  readonly acceptsMySession: boolean;
}

export function trustOf(row: FederationDoorwayRow): PeerTrust {
  const signed =
    !!row.record_signature &&
    row.record_signature.length > 0 &&
    !!row.signing_key &&
    row.record_serial !== null &&
    row.record_serial !== undefined;
  return signed ? 'dht-sourced' : 'unsigned-gossip';
}

/** `identity_root ?? signing_key ?? id`, matching how the rows key themselves. */
export function identityOf(row: FederationDoorwayRow): string {
  return row.identity_root ?? row.signing_key ?? row.id;
}

function endpointsOf(row: FederationDoorwayRow): DoorwayEndpoint[] {
  const declared = row.endpoints ?? [];
  if (declared.length > 0) {
    return declared.map(e => ({
      service: e.service,
      url: e.url,
      priority: e.priority,
      ...(e.ttl_secs === null || e.ttl_secs === undefined ? {} : { ttlSecs: e.ttl_secs }),
    }));
  }
  return [{ service: 'gateway', url: row.url, priority: 0 }];
}

export function peerOf(row: FederationDoorwayRow): KeepPeer {
  return {
    identity: identityOf(row),
    id: row.id,
    url: normalizeDoorwayUrl(row.url),
    trust: trustOf(row),
    ...(row.region ? { region: row.region } : {}),
    ...(row.tier ? { tier: row.tier } : {}),
    capabilities: row.capabilities ?? [],
    ...(row.status ? { status: row.status } : {}),
    endpoints: endpointsOf(row),
    acceptsMySession: false,
  };
}

export function resolutionOf(row: FederationDoorwayRow): DoorwayResolution {
  return {
    identity: identityOf(row),
    // 'registration' is the seam's reserved word for a signed record. An
    // unsigned row is NOT 'config' — nobody configured it — so it is named for
    // what it is.
    source: trustOf(row) === 'dht-sourced' ? 'registration' : 'federation-gossip',
    endpoints: endpointsOf(row),
  };
}

/** Every string a caller could plausibly hold for this row. */
function aliasesOf(row: FederationDoorwayRow): string[] {
  const aliases = [
    row.identity_root ?? undefined,
    row.signing_key ?? undefined,
    row.id,
    row.url,
    ...endpointsOf(row).map(e => e.url),
  ];
  const out = new Set<string>();
  for (const alias of aliases) {
    if (!alias) continue;
    out.add(alias);
    const normalized = normalizeDoorwayUrl(alias);
    if (normalized) out.add(normalized);
  }
  return [...out];
}

export interface FederationPeerResolverOptions {
  /**
   * Where to ask. Normally the origin the page was served from — the one fact a
   * page always knows without being told.
   */
  seedUrl: string;
  /**
   * Answer for anything the register does not know, and for every failure. This
   * is what makes the resolver strictly additive: its worst case IS this.
   */
  fallback: DoorwayAddressResolver;
  /** Defaults to `globalThis.fetch`. */
  fetchImpl?: typeof fetch;
  /** How long a loaded register is reused. Default 5 minutes. */
  ttlMs?: number;
  /** Milliseconds before a warm attempt is abandoned. Default 5 seconds. */
  timeoutMs?: number;
}

const DEFAULT_TTL_MS = 5 * 60 * 1000;
const DEFAULT_TIMEOUT_MS = 5000;
const FEDERATION_PATH = '/api/v1/federation/doorways';

/**
 * A `DoorwayAddressResolver` backed by the DHT-known doorway set, with the
 * configured resolver underneath it.
 *
 * Deliberately NOT an Angular service: this file is part of the framework-free
 * half of elohim-service and is asserted to be so by `keep-boundary.spec.ts`.
 * It uses `fetch` rather than `HttpClient` for a second reason too — the
 * interceptor rewrites `/api/` requests, so routing the register's own load
 * through Angular's HTTP stack would make it re-enter the thing it exists to
 * answer for.
 */
export class FederationPeerResolver implements DoorwayAddressResolver {
  private readonly seedUrl: string;
  private readonly fallback: DoorwayAddressResolver;
  private readonly fetchImpl?: typeof fetch;
  private readonly ttlMs: number;
  private readonly timeoutMs: number;

  private index = new Map<string, DoorwayResolution>();
  private known: KeepPeer[] = [];
  private loadedAt = 0;
  private inFlight: Promise<void> | null = null;

  constructor(options: FederationPeerResolverOptions) {
    this.seedUrl = normalizeDoorwayUrl(options.seedUrl);
    this.fallback = options.fallback;
    this.fetchImpl = options.fetchImpl;
    this.ttlMs = options.ttlMs ?? DEFAULT_TTL_MS;
    this.timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  }

  /** Everything the register currently knows. Empty until the first warm lands. */
  peers(): readonly KeepPeer[] {
    return this.known;
  }

  /** True once a load has succeeded and has not aged out. */
  get isWarm(): boolean {
    return this.loadedAt > 0 && Date.now() - this.loadedAt < this.ttlMs;
  }

  /**
   * Resolve an identity, alias, or URL.
   *
   * Synchronous by construction. A cold or stale register answers from the
   * fallback and warms in the background, so no request ever waits on
   * federation and a federation outage is invisible.
   */
  resolve(identity: string): DoorwayResolution {
    if (!this.isWarm) void this.warm();
    const hit = this.index.get(identity) ?? this.index.get(normalizeDoorwayUrl(identity));
    return hit ?? this.fallbackFor(identity);
  }

  /**
   * The fallback, with its throw contained.
   *
   * `ConfiguredDoorwayResolver` throws for an unknown identity. Letting that
   * escape would mean an unrecognised doorway takes down every request in the
   * app, which is strictly worse than the behaviour this replaces.
   */
  private fallbackFor(identity: string): DoorwayResolution {
    try {
      const resolution = this.fallback.resolve(identity);
      if (resolution instanceof Promise) {
        // The configured adapter is synchronous. An async fallback cannot be
        // awaited here without making resolve() async for everyone, so this
        // degrades to the seed rather than changing the contract.
        return this.seedResolution(identity);
      }
      return resolution;
    } catch {
      return this.seedResolution(identity);
    }
  }

  private seedResolution(identity: string): DoorwayResolution {
    return {
      identity,
      source: 'config',
      endpoints: [{ service: 'gateway', url: this.seedUrl, priority: 0 }],
    };
  }

  /**
   * Load the register. Never rejects; concurrent callers share one request.
   *
   * Awaiting this is how a caller gets determinism (a spec, or an app that
   * wants the set before it draws a doorway picker). Nothing on the request
   * path awaits it.
   */
  async warm(): Promise<void> {
    if (this.inFlight) return this.inFlight;
    this.inFlight = this.load().finally(() => {
      this.inFlight = null;
    });
    return this.inFlight;
  }

  private async load(): Promise<void> {
    const doFetch = this.fetchImpl ?? globalThis.fetch?.bind(globalThis);
    if (!doFetch) return;

    const controller = new AbortController();
    const timer = setTimeout(() => {
      controller.abort();
    }, this.timeoutMs);
    try {
      const response = await doFetch(`${this.seedUrl}${FEDERATION_PATH}`, {
        method: 'GET',
        signal: controller.signal,
      });
      if (!response.ok) return;
      const body = (await response.json()) as FederationDoorwaysResponseShape;
      const rows = body.doorways ?? [];
      // An empty list is not an answer worth caching over the fallback: it
      // would swap every request onto the seed for a whole TTL on the strength
      // of a doorway that simply has not discovered anyone yet.
      if (rows.length === 0) return;

      const index = new Map<string, DoorwayResolution>();
      const known: KeepPeer[] = [];
      for (const row of rows) {
        if (!row?.id || !row?.url) continue;
        const resolution = resolutionOf(row);
        for (const alias of aliasesOf(row)) {
          // First writer wins per alias, so a later gossip row cannot displace
          // a signed row that already claimed the same address.
          if (!index.has(alias)) index.set(alias, resolution);
        }
        known.push(peerOf(row));
      }
      if (index.size === 0) return;

      this.index = index;
      this.known = known;
      this.loadedAt = Date.now();
    } catch {
      // Every failure — offline, abort, non-JSON, malformed — leaves the
      // previous register (or the cold state) in place. There is no path here
      // that makes the app worse than not having tried.
    } finally {
      clearTimeout(timer);
    }
  }
}
