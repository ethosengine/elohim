/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/recovery-seed-commitment.schema.json -- DO NOT EDIT */

/**
 * Projection of imagodei RecoverySeedCommitment DHT entry. Source of truth: DHT (Notarized, Category A).
 */
export interface RecoverySeedCommitmentView {
  /**
   * Hex-encoded ActionHash
   */
  dhtAnchorHash: string;
  humanAgentPubkey: string;
  /**
   * Ed25519 public key derived from the recovery seed
   *
   * @minItems 32
   * @maxItems 32
   */
  seedPublicHalf: [
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
  thresholdN: number;
  totalM: number;
  /**
   * @minItems 16
   * @maxItems 16
   */
  commitmentNonce: [
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
  supersededBy?: string | null;
  /**
   * RFC3339 timestamp
   */
  createdAt: string;
}
