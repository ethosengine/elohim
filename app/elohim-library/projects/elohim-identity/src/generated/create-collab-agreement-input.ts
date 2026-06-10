/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: inputs/create-collab-agreement-input.schema.json -- DO NOT EDIT */

/**
 * Source of truth: Cat-A CollabAgreement DHT entry's share_allocation_json.form field in imagodei DNA (DNA-notarized). Category C — wire-shape vocabulary. Form of share-routing function. Per spec §6.1. M1 only supports declared.
 */
export type ShareAllocationForm = 'declared' | 'affinityDerived';

/**
 * Source of truth: Cat-A CollabAgreement DHT entry in imagodei DNA (DNA-notarized at commit via integrity validator + coordinator JSON structural checks). Category C — HTTP wire-shape input projecting onto the create_collab_agreement coordinator flow. Body for POST /api/v1/collab/agreement.
 */
export interface CreateCollabAgreementInput {
  /**
   * @minItems 2
   */
  participants: [string, string, ...string[]];
  scope: string;
  shareAllocation: ShareAllocation;
  governanceTerms?: {
    exitTerms?: 'clean' | 'repair';
  };
  initialTier: 'T0';
  displayNameForQahal: string;
  salt: string;
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
