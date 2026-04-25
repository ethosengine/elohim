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
  /**
   * Coordinator-populated resolution of humanAgentPubkey to the legacy String human_id used by HumanityWitness/HumanRelationship/IdentityFreeze. Null accepted at request commit; required by the time a KeyRotation is attempted for IntimateQuorum rotations or when a freeze may apply.
   */
  humanId?: string | null;
  /**
   * Coordinator-computed threshold for IntimateQuorum authority. Computed as ceil(emergency_contact_count / 2) + 1 at request time, floored at 2. Validator enforces distinct-witness-author count >= this value at KeyRotation commit.
   */
  requiredWitnessCount: number;
  createdAt: string;
}
