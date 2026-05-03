/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/my-cluster-view.schema.json -- DO NOT EDIT */

/**
 * Hardware/deployment archetype of the device presenting an AgentPeerBinding. Source of truth: DHT (Notarized, Category A — imagodei integrity zome). DNA-notarized as part of the AgentPeerBinding entry type. Values: 'node' = dedicated elohim-node server/blade; 'desktop' = Tauri desktop app (workstation or laptop); 'mobile' = mobile client (phone or tablet); 'steward' = steward process managing collective infrastructure.
 */
export type DeviceArchetype = 'node' | 'desktop' | 'mobile' | 'steward';

/**
 * Federated view of all devices belonging to a single agent (the viewer). Source of truth: peer_identity_bindings filtered by agent_cid (Category A from imagodei DHT), federated across household peers via the view-slice aggregator (Operational, Category C). Reconstructed per request; not persisted.
 */
export interface MyClusterView {
  /**
   * Agent CID this cluster view is scoped to.
   */
  agentCid: string;
  /**
   * Per-device summary rows, one per registered AgentPeerBinding for this agent.
   */
  devices: {
    peerId: string;
    archetype: DeviceArchetype;
    displayName?: string;
    online: boolean;
    freshness: Freshness;
    storageUsedBytes?: number;
    storageTotalBytes?: number;
    memoryUsedBytes?: number;
    memoryTotalBytes?: number;
    hostingCount?: number;
    projectingCount?: number;
    beaconAgeMs?: number;
  }[];
  /**
   * Aggregated totals across all devices in the cluster.
   */
  totals: {
    storageUsedBytes: number;
    storageTotalBytes: number;
    /**
     * Bytes committed to peers outside this agent's cluster.
     */
    externalCommittedBytes: number;
    /**
     * Net reciprocation bytes across this agent's devices (signed).
     */
    reciprocityNetBytes: number;
  };
  freshness: Freshness1;
}
/**
 * Liveness/staleness indicator carried on cluster + topology + slice views. Source of truth: composed from libp2p swarm liveness signals + slice timestamps (Operational, Category C). Reconstructed per request; not persisted.
 */
export interface Freshness {
  /**
   * Liveness bucket. live = fresh signal; stale = past freshness window; offline = no signal; cached_offline_until_reconnect = served from cache awaiting reconnect; unverifiable = signal received but signature/freshness cannot be checked; all_offline = entire device set is offline.
   */
  state:
    | 'live'
    | 'stale'
    | 'offline'
    | 'cached_offline_until_reconnect'
    | 'unverifiable'
    | 'all_offline';
  /**
   * Milliseconds since the last fresh signal, when state != live.
   */
  staleSinceMs?: number;
}
/**
 * Overall cluster freshness — worst-of-devices when any device is offline, live when all live.
 */
export interface Freshness1 {
  /**
   * Liveness bucket. live = fresh signal; stale = past freshness window; offline = no signal; cached_offline_until_reconnect = served from cache awaiting reconnect; unverifiable = signal received but signature/freshness cannot be checked; all_offline = entire device set is offline.
   */
  state:
    | 'live'
    | 'stale'
    | 'offline'
    | 'cached_offline_until_reconnect'
    | 'unverifiable'
    | 'all_offline';
  /**
   * Milliseconds since the last fresh signal, when state != live.
   */
  staleSinceMs?: number;
}
