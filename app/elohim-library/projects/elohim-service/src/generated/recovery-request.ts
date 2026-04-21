/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/recovery-request.schema.json -- DO NOT EDIT */

/**
 * Projection of imagodei RecoveryRequest DHT entry (modernized). Source of truth: DHT.
 */
export interface RecoveryRequestView {
  dhtAnchorHash: string;
  humanAgentPubkey: string;
  newAgentPubkey: string;
  hostingDoorwayPubkey: string;
  proposedAuthorityKind:
    | 'intimateQuorum'
    | 'communityConsensus'
    | 'governanceAct'
    | 'networkWitness'
    | 'cryptographicQuorum';
  proposedAuthorityJson: string;
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
