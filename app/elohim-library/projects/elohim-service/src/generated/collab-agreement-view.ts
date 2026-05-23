/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/collab-agreement-view.schema.json -- DO NOT EDIT */

/**
 * Source of truth: Cat-A CollabAgreement DHT entry's share_allocation_json.form field in imagodei DNA (DNA-notarized). Category C — wire-shape vocabulary. Form of share-routing function. Per spec §6.1. M1 only supports Declared.
 */
export type ShareAllocationForm = 'Declared' | 'AffinityDerived';
/**
 * Source of truth: Cat-A CollabAgreement DHT entry's initial_tier field in imagodei DNA, plus the deferred friction-gradient evaluator's derived tier output (DNA-notarized at creation; derived thereafter). Category C — wire-shape vocabulary. Coordination scale tier for a Collab-Qahal. Per spec §3.1. M1 only reaches T0; T1+ requires deferred specs.
 */
export type ElohimTier = 'T0' | 'T1' | 'T2' | 'T3';

/**
 * Source of truth: Cat-A CollabAgreement DHT entry in imagodei DNA (DNA-notarized) plus derived counter-attestation status computed from Cat-A2 AgreementOnCollab links (DNA-notarized link metadata). Category C — HTTP wire-shape projection; reconstructible at any time from DHT state. Includes counter-attestation status + the Collab-Qahal CID once instantiated.
 */
export interface CollabAgreementView {
  cid: string;
  authoredByAgentCid: string;
  participants: string[];
  scope: string;
  shareAllocation: ShareAllocation;
  commonsPoolTribute: number;
  governanceTerms?: {
    exitTerms?: 'clean' | 'repair';
  };
  initialTier: ElohimTier;
  createdAtBlockHeight: number;
  status: 'PendingAttestations' | 'Instantiated';
  attestedBy?: string[];
  collabQahalCid?: string | null;
}
/**
 * Source of truth: Cat-A CollabAgreement DHT entry's share_allocation_json field in imagodei DNA (DNA-notarized as canonical CBOR within the entry). Category C — wire-shape projection. Share-routing function declared on a CollabAgreement. Form A = Declared shares; Form B = AffinityDerived (M2).
 */
export interface ShareAllocation {
  form: ShareAllocationForm;
  /**
   * Required when form == Declared. Each share is fractional (0,1); sum + commonsPoolTribute must equal 1.0.
   */
  shares?: {
    collectiveCid: string;
    share: number;
  }[];
  /**
   * Required when form == AffinityDerived (M2).
   */
  affinityWindowBlocks?: number;
  /**
   * Required when form == AffinityDerived (M2).
   */
  rebalanceCadenceBlocks?: number;
  /**
   * Substrate-validated > 0. Zero tribute is refused per spec §6.3.
   */
  commonsPoolTribute: number;
}
