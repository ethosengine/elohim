/**
 * topology-graphql.adapter.ts
 *
 * Pure functions that transform GraphQL response payloads (from
 * `@elohim/storage-client/graphql`) into the REST-compatible view shapes
 * (`MyClusterView`, `PeerTopologyView`) that the rest of the app already
 * consumes.  No Angular injectables — these are stateless data mappers.
 *
 * SCREAMING_SNAKE_CASE → lowercase  (enum fields)
 * string | null        → number?    (byte/ms counters)
 * string               → number     (required totals)
 *
 * Known precision trade-off: converting `string` counter values via
 * `Number()` loses precision for integers > 2^53 (~9 PB). This is
 * acceptable for the visual-parity sprint (A6); the REST path already
 * loses this precision at JSON parse time in the browser anyway.
 * A future migration can adopt BigInt display once the UI layer is ready.
 */

import type {
  DeviceArchetype,
  Freshness,
  Freshness1,
  MyClusterView,
} from '@app/generated/my-cluster-view';
import type { PeerHouseholdEdge, PeerTopologyView } from '@app/generated/peer-topology-view';
import type { ReciprocityRow } from '@app/generated/reciprocity-row';
import type { ReciprocityView } from '@app/generated/reciprocity-view';
import type {
  ComputeTriptychGql,
  DeviceSummaryGql,
  FreshnessGql,
  PeerHouseholdEdgeGql,
  ReciprocityRowGql,
  ViewerHubResponse,
  ViewerPeersResponse,
  ViewerReciprocityResponse,
} from '@elohim/storage-client/graphql';

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/**
 * SCREAMING_SNAKE_CASE → lowercase for freshness state enums.
 * The REST layer returns e.g. `'live'`; GraphQL returns `'LIVE'`.
 */
function adaptFreshnessState(gql: FreshnessGql['state']): Freshness['state'] {
  return gql.toLowerCase() as Freshness['state'];
}

function adaptFreshness(gql: FreshnessGql): Freshness {
  return {
    state: adaptFreshnessState(gql.state),
    ...(gql.staleSinceMs !== null && { staleSinceMs: Number(gql.staleSinceMs) }),
  };
}

function adaptFreshnessTop(gql: FreshnessGql): Freshness1 {
  // Freshness1 has the same shape as Freshness — it's just the top-level
  // cluster freshness alias from the generated types.
  return adaptFreshness(gql) as Freshness1;
}

/**
 * SCREAMING_SNAKE_CASE → lowercase for DeviceArchetype enums.
 */
function adaptArchetype(gql: DeviceSummaryGql['archetype']): DeviceArchetype {
  return gql.toLowerCase() as DeviceArchetype;
}

function adaptDevice(gql: DeviceSummaryGql): MyClusterView['devices'][number] {
  return {
    peerId: gql.peerId,
    archetype: adaptArchetype(gql.archetype),
    ...(gql.displayName !== null && { displayName: gql.displayName }),
    online: gql.online,
    freshness: adaptFreshness(gql.freshness),
    // All byte/count fields are optional — omit the key entirely when null so
    // that `'key' in obj` returns false, matching the REST shape exactly.
    ...(gql.storageUsedBytes !== null && { storageUsedBytes: Number(gql.storageUsedBytes) }),
    ...(gql.storageTotalBytes !== null && { storageTotalBytes: Number(gql.storageTotalBytes) }),
    ...(gql.memoryUsedBytes !== null && { memoryUsedBytes: Number(gql.memoryUsedBytes) }),
    ...(gql.memoryTotalBytes !== null && { memoryTotalBytes: Number(gql.memoryTotalBytes) }),
    ...(gql.hostingCount !== null && { hostingCount: gql.hostingCount }),
    ...(gql.projectingCount !== null && { projectingCount: gql.projectingCount }),
    ...(gql.beaconAgeMs !== null && { beaconAgeMs: Number(gql.beaconAgeMs) }),
    // compute: omit the key entirely when null/undefined so that `'compute' in obj`
    // returns false — matches the REST shape and the null-omission parity convention.
    // Convert the GraphQL string-typed bytes to numbers so the device-level
    // compute matches the canonical `MyClusterView['devices'][number].compute`
    // shape (`ComputeTriptych` with `number | null` fields).
    ...(gql.compute != null && { compute: adaptComputeTriptych(gql.compute) }),
  };
}

/**
 * Adapt the GraphQL `ComputeTriptychGql` (string-typed bytes for JS precision
 * safety) to the canonical `ComputeTriptych` shape consumed by the REST view
 * (number-typed). Same precision trade-off as the other byte counters above.
 */
function adaptComputeTriptych(
  gql: ComputeTriptychGql
): MyClusterView['devices'][number]['compute'] {
  return {
    free: gql.free !== null ? Number(gql.free) : null,
    used: gql.used !== null ? Number(gql.used) : null,
    stewarded: gql.stewarded !== null ? Number(gql.stewarded) : null,
  };
}

function adaptEdge(gql: PeerHouseholdEdgeGql): PeerHouseholdEdge {
  return {
    householdId: gql.householdId,
    ...(gql.displayName !== null && { displayName: gql.displayName }),
    online: gql.online,
    ...(gql.lastSyncSec !== null && { lastSyncSec: Number(gql.lastSyncSec) }),
    myCidsHostedByThem: gql.myCidsHostedByThem,
    theirCidsHostedByMe: gql.theirCidsHostedByMe,
    ...(gql.netDiff !== null && { netDiff: Number(gql.netDiff) }),
    ...(gql.isCriticalForMe !== null && { isCriticalForMe: gql.isCriticalForMe }),
    ...(gql.iAmCriticalForThem !== null && { iAmCriticalForThem: gql.iAmCriticalForThem }),
  };
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/**
 * `MyClusterView` extended with the compute triptych on each device.
 * The REST path omits `compute`; the GraphQL path always populates it
 * (null when the peer has no compute signal yet).
 *
 * After the HubComputeAggregateView landing (92cad4734), `compute` is part
 * of the canonical `MyClusterView['devices'][number]` shape with
 * number-typed bytes. The intersection used to widen the canonical type
 * with the gql string-typed variant; that's now redundant. We keep the
 * alias for downstream call-site readability.
 */
export type MyClusterViewWithCompute = MyClusterView;

// ---------------------------------------------------------------------------
// Public adapters
// ---------------------------------------------------------------------------

/**
 * Adapt the `data` payload of a `VIEWER_HUB_QUERY` response into the
 * REST-equivalent `MyClusterView` (extended with per-device compute triptych).
 *
 * The caller is responsible for unwrapping the GraphQL envelope
 * (`{ data, errors }`); this function receives the typed `data` value.
 */
export function adaptHubResponse(gql: ViewerHubResponse): MyClusterViewWithCompute {
  const hub = gql.viewer.hub;
  return {
    agentCid: hub.agentCid,
    devices: hub.devices.map(adaptDevice),
    totals: {
      storageUsedBytes: Number(hub.totals.storageUsedBytes),
      storageTotalBytes: Number(hub.totals.storageTotalBytes),
      externalCommittedBytes: Number(hub.totals.externalCommittedBytes),
      reciprocityNetBytes: Number(hub.totals.reciprocityNetBytes),
    },
    freshness: adaptFreshnessTop(hub.freshness),
  };
}

/**
 * Adapt the `data` payload of a `VIEWER_PEERS_QUERY` response into the
 * REST-equivalent `PeerTopologyView`.
 *
 * The caller is responsible for unwrapping the GraphQL envelope.
 */
export function adaptPeersResponse(gql: ViewerPeersResponse): PeerTopologyView {
  const peers = gql.viewer.peers;
  return {
    agentCid: peers.agentCid,
    edges: peers.edges.map(adaptEdge),
    reciprocationCount: peers.reciprocationCount,
    resilienceCliffs: peers.resilienceCliffs.map(c => ({
      householdId: c.householdId,
      soleReplicaCidCount: c.soleReplicaCidCount,
    })),
    freshness: adaptFreshness(peers.freshness),
  };
}

/**
 * Map a single GraphQL `ReciprocityRowGql` to the REST-equivalent
 * `ReciprocityRow`. String byte counters → number (precision trade-off
 * accepted for the visual-parity sprint; same caveat as adaptHubResponse).
 */
function adaptReciprocityRow(gql: ReciprocityRowGql): ReciprocityRow {
  const row: ReciprocityRow = {
    counterpartyHouseholdId: gql.counterpartyHouseholdId,
    committedBytes: Number(gql.committedBytes),
    deliveredBytes: Number(gql.deliveredBytes),
    honoredPercent: gql.honoredPercent,
  };
  // `displayName?: string` — JSON-schema-to-typescript renders Option<String>
  // as an optional, so we omit the key when null rather than setting it to null.
  if (gql.displayName !== null) {
    row.displayName = gql.displayName;
  }
  if (gql.online !== null) {
    row.online = gql.online;
  }
  return row;
}

/**
 * Adapt the `data` payload of a `VIEWER_RECIPROCITY_QUERY` response into the
 * REST-equivalent `ReciprocityView`.
 *
 * The caller is responsible for unwrapping the GraphQL envelope.
 */
export function adaptReciprocityResponse(gql: ViewerReciprocityResponse): ReciprocityView {
  const r = gql.viewer.reciprocity;
  return {
    agentCid: r.agentCid,
    inflow: r.inflow.map(adaptReciprocityRow),
    outflow: r.outflow.map(adaptReciprocityRow),
    netHostedBytes: Number(r.netHostedBytes),
    capacityAvailableBytes: Number(r.capacityAvailableBytes),
  };
}
