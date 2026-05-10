/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: manifests/discovery-resolvers.schema.json -- DO NOT EDIT */

/**
 * A peer's declared list of pkarr discovery resolvers. Trusted resolvers a peer queries (and publishes to) for iroh NodeId → NodeAddr lookups. Per spec 2026-05-08-iroh-libp2p-complementarity.md, Step 3: a hub that distrusts n0 publishes its manifest with self-hosted resolvers only.
 */
export interface DiscoveryResolversManifest {
  /**
   * Ordered list. iroh queries them in parallel via ConcurrentDiscovery; first success wins. Order is operator-meaningful (preference signal) but not protocol-meaningful.
   *
   * @minItems 1
   */
  resolvers: [DiscoveryResolver, ...DiscoveryResolver[]];
}
export interface DiscoveryResolver {
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
}
