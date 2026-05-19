/**
 * GraphQL query documents + response types for the viewer surface.
 *
 * This module exposes the query strings and TS response interfaces for the
 * `viewer.hub` and `viewer.peers` resolvers landed in elohim-storage (Epic A,
 * tasks A2 + A3). It deliberately ships zero GraphQL client dependency —
 * consumers (Angular's HttpClient, fetch, etc.) call `POST /api/v1/graphql`
 * with `{ query, variables }` and parse the standard envelope shape themselves.
 *
 * Field selections mirror the resolver output exactly. Byte counters and other
 * `u64`/`i64` fields cross the wire as `string` to preserve precision beyond
 * JavaScript's 2^53 safe-integer ceiling — see resolvers.rs for the rationale.
 *
 * Enum string-unions match what async-graphql produces from the Rust `Enum`
 * derive: PascalCase Rust variants serialize as SCREAMING_SNAKE_CASE.
 */

// ---------------------------------------------------------------------------
// Query documents
// ---------------------------------------------------------------------------

export const VIEWER_HUB_QUERY = `
  query ViewerHub($agentCid: ID!) {
    viewer(agentCid: $agentCid) {
      agentCid
      hub {
        agentCid
        devices {
          peerId
          archetype
          displayName
          online
          freshness {
            state
            staleSinceMs
          }
          storageUsedBytes
          storageTotalBytes
          memoryUsedBytes
          memoryTotalBytes
          hostingCount
          projectingCount
          beaconAgeMs
        }
        totals {
          storageUsedBytes
          storageTotalBytes
          externalCommittedBytes
          reciprocityNetBytes
        }
        freshness {
          state
          staleSinceMs
        }
      }
    }
  }
`;

export const VIEWER_PEERS_QUERY = `
  query ViewerPeers($agentCid: ID!) {
    viewer(agentCid: $agentCid) {
      agentCid
      peers {
        agentCid
        edges {
          householdId
          displayName
          online
          lastSyncSec
          myCidsHostedByThem
          theirCidsHostedByMe
          netDiff
          isCriticalForMe
          iAmCriticalForThem
        }
        reciprocationCount
        resilienceCliffs {
          householdId
          soleReplicaCidCount
        }
        freshness {
          state
          staleSinceMs
        }
      }
    }
  }
`;

export const VIEWER_RECIPROCITY_QUERY = `
  query ViewerReciprocity($agentCid: ID!) {
    viewer(agentCid: $agentCid) {
      agentCid
      reciprocity {
        agentCid
        inflow {
          counterpartyHouseholdId
          displayName
          committedBytes
          deliveredBytes
          honoredPercent
          online
        }
        outflow {
          counterpartyHouseholdId
          displayName
          committedBytes
          deliveredBytes
          honoredPercent
          online
        }
        netHostedBytes
        capacityAvailableBytes
      }
    }
  }
`;

// ---------------------------------------------------------------------------
// Enums — string-unions matching async-graphql's SCREAMING_SNAKE_CASE
// serialization of the Rust `Enum`-derived variants in resolvers.rs.
// ---------------------------------------------------------------------------

/**
 * Freshness state bucket. Mirrors `FreshnessStateGql` in
 * `elohim/elohim-storage/src/graphql/resolvers.rs`.
 */
export type FreshnessState =
  | 'LIVE'
  | 'STALE'
  | 'OFFLINE'
  | 'CACHED_OFFLINE_UNTIL_RECONNECT'
  | 'UNVERIFIABLE'
  | 'ALL_OFFLINE';

/**
 * Hardware/deployment archetype. Mirrors `DeviceArchetypeGql` in
 * `elohim/elohim-storage/src/graphql/resolvers.rs`.
 */
export type DeviceArchetype = 'NODE' | 'DESKTOP' | 'MOBILE' | 'STEWARD';

// ---------------------------------------------------------------------------
// Component types
// ---------------------------------------------------------------------------

/** Liveness / staleness indicator. */
export interface FreshnessGql {
  state: FreshnessState;
  /** Epoch-ms (as string for precision) when data became stale; null when state is Live. */
  staleSinceMs: string | null;
}

/**
 * Per-device summary in the hub view.
 *
 * Byte counters are `string` to avoid JS integer-precision loss at 2^53.
 * Null counters indicate metrics were not yet available from the peer.
 */
export interface DeviceSummaryGql {
  peerId: string;
  archetype: DeviceArchetype;
  displayName: string | null;
  online: boolean;
  freshness: FreshnessGql;
  storageUsedBytes: string | null;
  storageTotalBytes: string | null;
  memoryUsedBytes: string | null;
  memoryTotalBytes: string | null;
  hostingCount: number | null;
  projectingCount: number | null;
  beaconAgeMs: string | null;
}

/** Aggregated totals across the agent's devices. */
export interface DeviceTotalsGql {
  storageUsedBytes: string;
  storageTotalBytes: string;
  externalCommittedBytes: string;
  reciprocityNetBytes: string;
}

/** Hub view — the agent's federated cluster of devices. */
export interface HubViewGql {
  agentCid: string;
  devices: DeviceSummaryGql[];
  totals: DeviceTotalsGql;
  freshness: FreshnessGql;
}

/** Per-household reciprocation edge in the peer topology view. */
export interface PeerHouseholdEdgeGql {
  householdId: string;
  displayName: string | null;
  online: boolean;
  lastSyncSec: string | null;
  myCidsHostedByThem: number;
  theirCidsHostedByMe: number;
  netDiff: string | null;
  isCriticalForMe: boolean | null;
  iAmCriticalForThem: boolean | null;
}

/**
 * Sole-replica risk row — a peer household that is the only replicator of
 * one or more CIDs this agent cares about.
 */
export interface ResilienceCliffGql {
  householdId: string;
  soleReplicaCidCount: number;
}

/** Peer topology view — reciprocation edges + resilience cliffs. */
export interface PeerTopologyGql {
  agentCid: string;
  edges: PeerHouseholdEdgeGql[];
  reciprocationCount: number;
  resilienceCliffs: ResilienceCliffGql[];
  freshness: FreshnessGql;
}

/**
 * Per-counterparty row in the reciprocity view's inflow / outflow ledger.
 *
 * Byte counters are `string` to preserve precision across the JS boundary
 * (u64 can exceed 2^53). `honoredPercent` stays `number` (f64). `online`
 * is `null` on cold-cache reads (no live swarm context).
 */
export interface ReciprocityRowGql {
  counterpartyHouseholdId: string;
  displayName: string | null;
  committedBytes: string;
  deliveredBytes: string;
  honoredPercent: number;
  online: boolean | null;
}

/** Reciprocity view — agent-scoped inflow/outflow ledger. */
export interface ReciprocityViewGql {
  agentCid: string;
  inflow: ReciprocityRowGql[];
  outflow: ReciprocityRowGql[];
  /** Signed net hosting position (stringified i64). Positive = others host more for the viewer than the viewer does for them. */
  netHostedBytes: string;
  /** Spare capacity the viewer can offer to expand reciprocity (stringified u64). */
  capacityAvailableBytes: string;
}

// ---------------------------------------------------------------------------
// Operation response shapes (the `data` payload inside the GraphQL envelope)
// ---------------------------------------------------------------------------

/** Response payload for `VIEWER_HUB_QUERY`. */
export interface ViewerHubResponse {
  viewer: {
    agentCid: string;
    hub: HubViewGql;
  };
}

/** Response payload for `VIEWER_PEERS_QUERY`. */
export interface ViewerPeersResponse {
  viewer: {
    agentCid: string;
    peers: PeerTopologyGql;
  };
}

/** Response payload for `VIEWER_RECIPROCITY_QUERY`. */
export interface ViewerReciprocityResponse {
  viewer: {
    agentCid: string;
    reciprocity: ReciprocityViewGql;
  };
}

// ---------------------------------------------------------------------------
// GraphQL envelope
// ---------------------------------------------------------------------------

/** Standard GraphQL error shape (subset that callers typically care about). */
export interface GraphQLError {
  message: string;
  path?: ReadonlyArray<string | number>;
  locations?: ReadonlyArray<{ line: number; column: number }>;
  extensions?: Record<string, unknown>;
}

/**
 * The `{ data, errors }` envelope returned from `POST /api/v1/graphql`.
 *
 * `data` is nullable per the GraphQL spec — when a request fails at
 * resolution time the server may return `errors` only.
 */
export interface GraphQLEnvelope<T> {
  data?: T | null;
  errors?: GraphQLError[];
}

/** Variables for `VIEWER_HUB_QUERY` / `VIEWER_PEERS_QUERY` (same shape). */
export interface ViewerQueryVariables {
  agentCid: string;
}
