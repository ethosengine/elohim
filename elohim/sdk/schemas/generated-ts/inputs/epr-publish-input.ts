/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: inputs/epr-publish-input.schema.json -- DO NOT EDIT */

/**
 * The nine EPR atom kinds defined by the graph substrate spec §4.2. Source of truth: elohim-epr protocol constants. Category A — enumeration values are part of the DNA-notarized protocol vocabulary. Each kind declares required coupling legs: Content requires all three (knowledge + value + governance); Agent/Manifest/Attestation/Delegation require governance; Claim/Observation require knowledge; EconomicEvent requires value; Commitment requires value + governance.
 */
export type EprKind =
  | 'Content'
  | 'Agent'
  | 'Manifest'
  | 'Claim'
  | 'Observation'
  | 'EconomicEvent'
  | 'Commitment'
  | 'Attestation'
  | 'Delegation';
/**
 * Content reach/visibility level. Ordered from most restrictive to most open. Source of truth: DNA-notarized CORE_REACH_LEVELS constant in content_store_integrity zome. Category A — enumeration values are part of the protocol vocabulary enforced by gateways without parsing payload.
 */
export type Reach =
  | 'private'
  | 'self'
  | 'intimate'
  | 'trusted'
  | 'familiar'
  | 'community'
  | 'public'
  | 'commons';

/**
 * Input body for POST /api/v1/epr. Accepts a fully-signed EPR from the caller; server validates signature + coupling before storing.
 */
export interface EprPublishInput {
  envelope: EprEnvelopeView;
  /**
   * Hex-encoded payload bytes
   */
  payload: string;
}
/**
 * Source of truth: EPR atom (self-notarized via content-address + Ed25519). Wire-string projection of Envelope for HTTP consumers. CIDs are CIDv1 base32 strings. Category A — notarized via content-derived CID + signer proof.
 */
export interface EprEnvelopeView {
  /**
   * CIDv1 base32
   */
  cid: string;
  kind: EprKind;
  /**
   * CIDv1 base32 of the Manifest EPR
   */
  schemaRef: string;
  /**
   * Content-type key within the referenced manifest
   */
  schemaKey: string;
  reach: Reach;
  coupling: {
    /**
     * CIDv1 base32 of Claim-EPR
     */
    knowledge?: string | null;
    /**
     * CIDv1 base32 of EconomicEvent-EPR
     */
    value?: string | null;
    /**
     * CIDv1 base32 of Governance-EPR
     */
    governance?: string | null;
  };
  claims: string[];
  /**
   * CIDv1 base32
   */
  supersedes?: string | null;
  /**
   * CIDv1 base32 (derived from index, not in canonical bytes)
   */
  supersededBy?: string | null;
  issuedAt: string;
  proof: {
    /**
     * CIDv1 base32 of Agent EPR
     */
    signer: string;
    algorithm: 'ed25519';
    /**
     * Hex-encoded 64 bytes
     */
    signature: string;
  };
}
