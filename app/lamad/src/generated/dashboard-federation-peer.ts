/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/dashboard-federation-peer.schema.json -- DO NOT EDIT */

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
