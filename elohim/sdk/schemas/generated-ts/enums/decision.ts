/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: enums/decision.schema.json -- DO NOT EDIT */

/**
 * The three-valued verdict discriminator of the protocol's verdict spine. `permit` and `refuse` are the two mechanical outcomes a deterministic check can reach. `refer` is the ceiling marker: a first-class 'this is not decidable by rule; it routes to the named layer with a stated reason'. WHO resolves there (elohim counsel, human, quorum) is governed routing — deployment policy, never vocabulary — bounded by constitutional floors that guarantee value conflicts and irreversible acts reach humans — never a fallthrough, a timeout, or an error. Every gradient axis that classifies a claim (reach today, epistemic standing this slice, compute/trust variance and retention downstream) narrows onto this same spine. Source of truth: elohim/epr/src/verdict.rs (`Decision` enum — Rust is the wire authority, this schema is its projection). Spec: reach-ontology-vocabulary-split-spec §2a.
 */
export type Decision = 'permit' | 'refuse' | 'refer';
