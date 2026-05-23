/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/collab-qahal-view.schema.json -- DO NOT EDIT */

/**
 * Source of truth: Cat-A CollabAgreement DHT entry's initial_tier field in imagodei DNA, plus the deferred friction-gradient evaluator's derived tier output (DNA-notarized at creation; derived thereafter). Category C — wire-shape vocabulary. Coordination scale tier for a Collab-Qahal. Per spec §3.1. M1 only reaches T0; T1+ requires deferred specs.
 */
export type ElohimTier = 'T0' | 'T1' | 'T2' | 'T3';
/**
 * Source of truth: Cat-A CollabAgreement DHT entry's initial_tier field in imagodei DNA, plus the deferred friction-gradient evaluator's derived tier output (DNA-notarized at creation; derived thereafter). Category C — wire-shape vocabulary. Coordination scale tier for a Collab-Qahal. Per spec §3.1. M1 only reaches T0; T1+ requires deferred specs.
 */
export type ElohimTier1 = 'T0' | 'T1' | 'T2' | 'T3';

/**
 * Source of truth: Cat-A Collective DHT entry in imagodei DNA (the Collab-Qahal is itself a recursive Collective; anchor_agreement_cid distinguishes it) plus traversal over Cat-A Membership entries via Cat-A2 HasMembership links (all DNA-notarized). Category C — HTTP wire-shape projection; reconstructible at any time. Wire shape returned by GET /api/v1/collab/:cid.
 */
export interface CollabQahalView {
  cid: string;
  anchorAgreementCid: string;
  displayName: string;
  createdAtBlockHeight: number;
  elohimTier: ElohimTier;
  memberCollectives: CollectiveView[];
  memberPersons: string[];
  commonsPoolBalance?: number;
}
/**
 * Source of truth: Cat-A Collective DHT entry in imagodei DNA (DNA-notarized). Category C — HTTP wire-shape projection; reconstructible at any time from DHT state. Wire shape for GET /api/v1/collective/:cid.
 */
export interface CollectiveView {
  cid: string;
  founderAgentCid: string;
  charter: string;
  displayName: string;
  createdAtBlockHeight: number;
  anchorAgreementCid?: string | null;
  elohimTier: ElohimTier1;
}
