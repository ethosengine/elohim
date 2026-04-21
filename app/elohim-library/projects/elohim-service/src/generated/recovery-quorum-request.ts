/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/recovery-quorum-request.schema.json -- DO NOT EDIT */

/**
 * Projection of imagodei RecoveryQuorumRequest DHT entry. Source of truth: DHT (Notarized, Category A).
 */
export interface RecoveryQuorumRequestView {
  dhtAnchorHash: string;
  humanAgentPubkey: string;
  seedCommitmentHash: string;
  newAgentPubkey: string;
  hostingDoorwayPubkey: string;
  recoveryMode: 'normal' | 'stewarded';
  stewardedGrantHash?: string | null;
  /**
   * @minItems 16
   * @maxItems 16
   */
  requestNonce: [
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
  createdAt: string;
}
