/* Generated from protocol schema: views/agent-peer-binding-view.schema.json -- DO NOT EDIT */

/**
 * Source of truth: DHT (Notarized, Category A). AgentPeerBinding from EPR 2B Batch A binds a peer's libp2p PeerId to its agent CID with signed validity window.
 */
export interface AgentPeerBindingView {
  agentCid: string;
  /**
   * libp2p PeerId
   */
  peerId: string;
  validFrom: string;
  validUntil?: string | null;
  /**
   * Ed25519 signature over canonical bytes
   */
  signature: string;
  dhtAnchorHash: string;
}
