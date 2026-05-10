/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: enums/coupling-leg.schema.json -- DO NOT EDIT */

/**
 * The three legs of the ThreeLegCoupling defined by the graph substrate spec §4.1. Source of truth: elohim-epr protocol constants. Category A — enumeration values are part of the DNA-notarized protocol vocabulary. knowledge = knowledge graph binding (Claim EPR CID); value = economic event binding (EconomicEvent EPR CID); governance = governance attestation binding (Governance EPR CID). Serialized lowercase by serde.
 */
export type CouplingLeg = 'knowledge' | 'value' | 'governance';
