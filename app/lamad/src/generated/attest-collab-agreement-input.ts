/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: inputs/attest-collab-agreement-input.schema.json -- DO NOT EDIT */

/**
 * Source of truth: Cat-A2 AgreementOnCollab link in imagodei DNA (DNA-notarized link with attesting collective CID in the tag bytes). Category C — HTTP wire-shape input projecting onto the attest_collab_agreement coordinator flow. Body for POST /api/v1/collab/agreement/{cid}/attest.
 */
export interface AttestCollabAgreementInput {
  agreementCid: string;
  attestingCollectiveCid: string;
}
