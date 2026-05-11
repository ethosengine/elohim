/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/peer-topology-view.schema.json -- DO NOT EDIT */

/**
 * Per-agent peer topology — household-to-household reciprocation edges + resilience cliffs. Source of truth: peer_identity_bindings (Category A) + libp2p swarm state, federated across household peers via the T26 aggregator (Operational, Category C). Reconstructed per request; not persisted.
 */
export interface PeerTopologyView {
  /**
   * Agent CID this topology view is scoped to.
   */
  agentCid: string;
  /**
   * Per-household reciprocation edges from the viewer's perspective.
   */
  edges: PeerHouseholdEdge[];
  /**
   * Count of households the viewer reciprocates with (top-card metric per the household-is-resilience-unit principle).
   */
  reciprocationCount: number;
  /**
   * Households whose disappearance would create sole-replica risk for the viewer.
   */
  resilienceCliffs: {
    householdId: string;
    /**
     * Count of CIDs that would lose sole-replica protection if this household goes dark.
     */
    soleReplicaCidCount: number;
  }[];
  freshness: Freshness;
}
/**
 * Per-household reciprocation edge in a peer topology view. Source of truth: peer_identity_bindings (Category A from imagodei DHT) joined with rea_commitments + economic_events for hosting accounting (Operational, Category C). Reconstructed per request; not persisted.
 */
export interface PeerHouseholdEdge {
  /**
   * Stable identifier for the counterparty household.
   */
  householdId: string;
  /**
   * Human-readable label for the household, when known.
   */
  displayName?: string;
  /**
   * Whether at least one peer in the household is currently online.
   */
  online: boolean;
  /**
   * Seconds since the last successful sync with the counterparty.
   */
  lastSyncSec?: number;
  /**
   * Count of viewer's CIDs currently replicated by the counterparty household.
   */
  myCidsHostedByThem: number;
  /**
   * Count of counterparty CIDs currently replicated by the viewer's household.
   */
  theirCidsHostedByMe: number;
  /**
   * Signed difference (theirCidsHostedByMe - myCidsHostedByThem).
   */
  netDiff?: number;
  /**
   * True when this household holds CIDs for which the viewer would lose sole-replica protection if it goes dark.
   */
  isCriticalForMe?: boolean;
  /**
   * True when the viewer is the only off-household replica of CIDs the counterparty owns.
   */
  iAmCriticalForThem?: boolean;
}
/**
 * Topology freshness — composed from edge liveness.
 */
export interface Freshness {
  /**
   * Liveness bucket. live = fresh signal; stale = past freshness window; offline = no signal; cached_offline_until_reconnect = served from cache awaiting reconnect; unverifiable = signal received but signature/freshness cannot be checked; all_offline = entire device set is offline.
   */
  state: 'live' | 'stale' | 'offline' | 'cached_offline_until_reconnect' | 'unverifiable' | 'all_offline';
  /**
   * Milliseconds since the last fresh signal, when state != live.
   */
  staleSinceMs?: number;
}
