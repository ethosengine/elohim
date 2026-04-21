/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/key-rotation.schema.json -- DO NOT EDIT */

/**
 * Projection of imagodei KeyRotation DHT entry with RecoveryAuthority enum. Source of truth: DHT. The authoritative claim that a human's agent key has rotated. Authority evidence carried as variant_kind + JSON-encoded variant fields.
 */
export interface KeyRotationView {
  dhtAnchorHash: string;
  humanAgentPubkey: string;
  newAgentPubkey: string;
  supersededAgentPubkey: string;
  recoveryRequestHash: string;
  authorityKind:
    | 'intimateQuorum'
    | 'communityConsensus'
    | 'governanceAct'
    | 'networkWitness'
    | 'cryptographicQuorum';
  authorityJson: string;
  rotatedAt: string;
}
