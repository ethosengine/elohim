/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/doorway-dashboard-view.schema.json -- DO NOT EDIT */

/**
 * Hardware/deployment archetype of the device presenting an AgentPeerBinding. Source of truth: DHT (Notarized, Category A — imagodei integrity zome). DNA-notarized as part of the AgentPeerBinding entry type. Values: 'node' = dedicated elohim-node server/blade; 'desktop' = Tauri desktop app (workstation or laptop); 'mobile' = mobile client (phone or tablet); 'steward' = steward process managing collective infrastructure.
 */
export type DeviceArchetype = 'node' | 'desktop' | 'mobile' | 'steward';

/**
 * Doorway operator dashboard payload — storage stewards, federation peers, projection coverage, and public-surface health. Source of truth: doorway-side aggregation of cluster + reciprocity views over the libp2p view-federation protocol (Operational, Category C). Reconstructed per request; not persisted as a new entity.
 */
export interface DoorwayDashboardView {
  /**
   * Public hostname of this doorway.
   */
  doorwayHostname: string;
  /**
   * Per-storage-steward rows for peers registered with this doorway.
   */
  storageStewards: DashboardSteward[];
  /**
   * Per-doorway federation rows.
   */
  federationPeers: DashboardFederationPeer[];
  projectionCoverage: ProjectionCoverage;
  publicSurface: PublicSurfaceState;
}
/**
 * Per-storage-steward row in a doorway dashboard. Source of truth: peer_identity_bindings (Category A from imagodei DHT) joined with hosting counts derived from rea_commitments (Operational, Category C). Reconstructed per request; not persisted.
 */
export interface DashboardSteward {
  peerId: string;
  archetype: DeviceArchetype;
  displayName?: string;
  online: boolean;
  /**
   * Count of CIDs this steward currently hosts.
   */
  hostingCount: number;
  /**
   * Approximate libp2p Kad hop distance from the doorway.
   */
  hopHint?: number;
}
/**
 * Per-doorway federation row in a doorway dashboard. Source of truth: doorway-to-doorway federation registry observed via libp2p (Operational, Category C). Reconstructed per request; not persisted.
 */
export interface DashboardFederationPeer {
  doorwayHostname: string;
  online: boolean;
  /**
   * Which way bytes flow across this federation edge.
   */
  direction: 'bidirectional' | 'outbound_only' | 'inbound_only';
  /**
   * Count of CIDs both doorways project.
   */
  sharedCidCount: number;
  /**
   * Resolvers this peer publishes as trusted. Empty/omitted means 'inherits federation defaults'. A peer that explicitly publishes [{kind: 'operator-self-hosted', url: 'https://<their-doorway>/pkarr'}] (and no n0-default entry) is opting out of n0 — gate #10 + Step 3 of the n0-mitigation spec.
   */
  discovery_resolvers?: {
    /**
     * Base HTTPS URL of the pkarr relay endpoint. The pkarr wire protocol appends /<z32-public-key> to this base. Example: https://doorway.elohim.host/pkarr
     */
    url: string;
    /**
     * Provenance of this resolver. Audit + UI hint; not consulted by the wire protocol.
     */
    kind: 'n0-default' | 'operator-self-hosted' | 'federated-peer' | 'third-party';
    /**
     * If kind is 'operator-self-hosted' or 'federated-peer', the doorway_id that runs this resolver. Cross-referenced against federation.doorways.
     */
    operator_doorway_id?: string;
    /**
     * Human-readable label for operator dashboards.
     */
    label?: string;
  }[];
}
/**
 * Projection cache and lag aggregate for this doorway.
 */
export interface ProjectionCoverage {
  /**
   * Number of CIDs the doorway is currently projecting.
   */
  projectedCidCount: number;
  /**
   * Total CIDs the doorway is aware of (projected + known but unprojected).
   */
  knownCidCount: number;
  /**
   * Trailing 24h cache hit ratio (0.0-1.0).
   */
  cacheHitRate24h: number;
  /**
   * Average milliseconds between source observation and projection ack over last 24h.
   */
  projectionLagMsAvg: number;
}
/**
 * DNS/TLS/reachability surface for the doorway hostname.
 */
export interface PublicSurfaceState {
  dnsResolves: boolean;
  /**
   * Resolved A/AAAA target, when DNS resolves.
   */
  dnsTarget?: string;
  tlsValid: boolean;
  /**
   * Days until TLS cert expires (negative if already expired).
   */
  tlsExpiresInDays?: number;
  /**
   * True when an external probe can reach the doorway.
   */
  publicReachable: boolean;
}
