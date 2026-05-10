/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: enums/epr-kind.schema.json -- DO NOT EDIT */

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
