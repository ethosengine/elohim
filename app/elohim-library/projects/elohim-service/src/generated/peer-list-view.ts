/* Generated from protocol schema: views/peer-list-view.schema.json -- DO NOT EDIT */

/**
 * NAT traversal status as detected by libp2p autonat protocol
 */
export type NatStatus = 'unknown' | 'public' | 'private';

/**
 * Paginated list of connected peers. Source of truth: libp2p Swarm state (Operational, Category C).
 */
export interface PeerListView {
  /**
   * Connected peers with detail
   */
  peers: PeerInfoView[];
  /**
   * Total connected peer count
   */
  total: number;
}
/**
 * Per-peer detail from libp2p Swarm state. Source of truth: libp2p identify protocol + per-peer runtime tracking (Operational, Category C). All ephemeral.
 */
export interface PeerInfoView {
  /**
   * libp2p PeerId (base58 encoded)
   */
  peerId: string;
  /**
   * Known multiaddrs for this peer (from identify)
   */
  multiaddrs: string[];
  /**
   * Protocols supported by this peer (from identify)
   */
  protocols: string[];
  /**
   * Agent version string from identify protocol
   */
  agentVersion: string;
  /**
   * Connection direction — who initiated
   */
  direction: 'inbound' | 'outbound';
  /**
   * Round-trip time in milliseconds. Tier 3 — null until follow-up sprint implements RTT ring buffer
   */
  rttMs?: number | null;
  /**
   * Unix epoch millis of last activity. Tier 3 — null until follow-up sprint implements lastSeen tracking
   */
  lastSeenMs?: number | null;
  /**
   * Remote peer's NAT status. Tier 3 — null until follow-up sprint implements dial-back results
   */
  remoteNatStatus?: NatStatus | null;
  /**
   * Bytes received from this peer since connection. Tier 3 — null until follow-up sprint implements BandwidthSinks
   */
  bandwidthIn?: number | null;
  /**
   * Bytes sent to this peer since connection. Tier 3 — null until follow-up sprint implements BandwidthSinks
   */
  bandwidthOut?: number | null;
}
