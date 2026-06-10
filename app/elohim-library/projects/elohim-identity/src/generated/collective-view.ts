/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/collective-view.schema.json -- DO NOT EDIT */

/**
 * Source of truth: Cat-A CollabAgreement DHT entry's initial_tier field in imagodei DNA, plus the deferred friction-gradient evaluator's derived tier output (DNA-notarized at creation; derived thereafter). Category C — wire-shape vocabulary. Coordination scale tier for a Collab-Qahal. Per spec §3.1. M1 only reaches T0; T1+ requires deferred specs.
 */
export type ElohimTier = 'T0' | 'T1' | 'T2' | 'T3';

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
  elohimTier: ElohimTier;
}
