/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/epr-verify-view.schema.json -- DO NOT EDIT */

/**
 * Source of truth: EPR atom verification result (derived on request). Category C — operational (reconstructed per request, not persisted).
 */
export interface EprVerifyView {
  /**
   * CIDv1 base32 of the EPR being verified
   */
  cid: string;
  /**
   * true iff all run stages passed
   */
  verified: boolean;
  stagesRun: ('canonicalization' | 'signature' | 'coupling' | 'payloadSchema')[];
  /**
   * Stages that were not run (e.g., payloadSchema deferred to Phase 3)
   */
  stagesSkipped?: ('canonicalization' | 'signature' | 'coupling' | 'payloadSchema')[];
  error?: {
    stage: 'canonicalization' | 'signature' | 'coupling' | 'payloadSchema';
    message: string;
  } | null;
}
