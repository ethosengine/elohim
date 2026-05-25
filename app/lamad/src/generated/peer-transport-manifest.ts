/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/peer-transport-manifest.schema.json -- DO NOT EDIT */

/**
 * Source of truth: libp2p Swarm + iroh Endpoint observations (Operational, Category C). Persisted via peer_transport_manifest SQLite table for hybrid-window transport selection. Rebuildable from observed handshake arrivals on either transport. Per spec 2026-05-08-iroh-libp2p-complementarity §peer-transport-manifest.
 */
export interface PeerTransportManifestView {
  /**
   * Canonical DHT-anchored agent identity (imagodei). Stable across transports.
   */
  agentCid: string;
  /**
   * libp2p transport profile if peer speaks libp2p. Null when only iroh-observed.
   */
  libp2p?: Libp2PTransportProfileView | null;
  /**
   * iroh transport profile if peer speaks iroh. Null when only libp2p-observed.
   */
  iroh?: IrohTransportProfileView | null;
  /**
   * Discovery methods that surfaced this peer.
   */
  discovery: ('pkarr' | 'kademlia' | 'mdns' | 'doorway-introduction')[];
  /**
   * Device-archetype capability tier: 0=unknown, 1=wearable, 2=phone, 3=laptop, 4=always-on full node, 5=hub default.
   */
  capabilityLevel: number;
  /**
   * Unix epoch seconds of the last observation.
   */
  lastObserved: number;
}
export interface Libp2PTransportProfileView {
  /**
   * libp2p PeerId (multihash, multibase encoded).
   */
  peerId: string;
  /**
   * Listen multiaddrs.
   */
  addrs: string[];
  /**
   * Planes this peer supports over libp2p.
   */
  supports: (
    | 'blob'
    | 'gossip'
    | 'sync'
    | 'epr'
    | 'epr-atom'
    | 'shard'
    | 'view-fed'
    | 'identity-handshake'
    | 'trust'
  )[];
}
export interface IrohTransportProfileView {
  /**
   * iroh NodeId (32-byte ed25519 pubkey, base32 encoded).
   */
  nodeId: string;
  /**
   * Relay URLs the peer is reachable through.
   */
  relays: string[];
  /**
   * Planes this peer supports over iroh.
   */
  supports: (
    | 'blob'
    | 'gossip'
    | 'sync'
    | 'epr'
    | 'epr-atom'
    | 'shard'
    | 'view-fed'
    | 'identity-handshake'
    | 'trust'
  )[];
}
