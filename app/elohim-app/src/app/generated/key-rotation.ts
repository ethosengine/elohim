/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/key-rotation.schema.json -- DO NOT EDIT */

/**
 * Projection of imagodei KeyRotation DHT entry. Source of truth: DHT (Notarized, Category A). The authoritative claim that a human's agent key has rotated.
 */
export interface KeyRotationView {
  dhtAnchorHash: string;
  humanAgentPubkey: string;
  newAgentPubkey: string;
  supersededAgentPubkey: string;
  seedCommitmentHash: string;
  recoveryRequestHash: string;
  /**
   * Ed25519 signature under seed_public_half over (new_agent_pubkey || recovery_request_hash)
   *
   * @minItems 64
   * @maxItems 64
   */
  quorumSignature: [
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
    number,
  ];
  rotatedAt: string;
}
