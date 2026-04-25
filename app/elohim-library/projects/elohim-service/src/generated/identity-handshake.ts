/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: p2p/identity-handshake.schema.json -- DO NOT EDIT */

/**
 * Category C — operational, session-local. Wire contract for /elohim/identity/handshake/1.0.0 request-response protocol. MessagePack-encoded on the wire (4-byte BE length prefix + body). Wraps a signed AgentPeerBinding whose source of truth is the DHT-notarized entry in imagodei (Category A — Task A.2). The handshake itself is NOT a source of truth; it is a libp2p session-local projection of DHT state, reconstructable on re-handshake. Source of truth for AgentPeerBinding: imagodei DHT (ActionHash is canonical identifier). Transport representation uses camelCase field names matching the objects/agent-peer-binding schema.
 */
export type IdentityHandshakeMessage = HandshakeRequest | HandshakeResponse;

/**
 * Sent by the initiating peer to present its signed AgentPeerBinding.
 */
export interface HandshakeRequest {
  /**
   * Signed AgentPeerBinding. All fields from the DHT entry are included except supersededBy (not part of the canonical signing body). The peerId field MUST match the sender's libp2p PeerId.
   */
  binding: {
    /**
     * libp2p PeerId of the sender (multibase-encoded, base58btc multihash). Must match the sending peer's connection identity.
     */
    peerId: string;
    /**
     * CIDv1 (dag-cbor sha256) of the Agent EPR in imagodei — the content-addressed identifier of the Holochain agent that operates this peer.
     */
    agentCid: string;
    /**
     * UTC timestamp (RFC3339, Z suffix) from which this binding is valid.
     */
    validFrom: string;
    /**
     * UTC timestamp at which this binding expires, or null if no declared expiry.
     */
    validUntil?: string | null;
    /**
     * Hardware/deployment archetype of this peer.
     */
    deviceArchetype: 'node' | 'desktop' | 'mobile' | 'steward';
    /**
     * Base64-encoded Ed25519 signature over the canonical binding body (all fields except signature and supersededBy, deterministically serialized).
     */
    signature: string;
    /**
     * ActionHash (base64url) of the AgentPeerBinding DHT entry, if known to the sender. Optional at handshake time — the receiver uses it as dht_anchor_hash if present, otherwise synthesises a stable placeholder.
     */
    dhtAnchorHash?: string | null;
  };
  /**
   * UTC timestamp of this handshake attempt (RFC3339, Z suffix). Used to bound replay window: receiver rejects handshakes timestamped more than 5 minutes in the past or future.
   */
  timestamp: string;
  /**
   * 16-byte random nonce, base64-encoded. Prevents replay within the replay window. Not verified cryptographically at Stage 1; present for future use.
   */
  nonce: string;
}
/**
 * Sent by the receiver to acknowledge the presented binding.
 */
export interface HandshakeResponse {
  /**
   * Outcome of the handshake verification.
   */
  tag: 'accepted' | 'rejected' | 'error';
  /**
   * Human-readable reason for rejection or error. Present when tag is 'rejected' or 'error'.
   */
  reason?: string | null;
}
