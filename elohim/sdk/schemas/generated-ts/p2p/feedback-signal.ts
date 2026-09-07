/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: p2p/feedback-signal.schema.json -- DO NOT EDIT */

/**
 * Category B2 — agent-scoped with attestation. Wire contract for the FeedbackSignal EPR kind (Phase 3.5). Travels via the existing /elohim/epr-atom/1.0.0 libp2p protocol as a signed EPR atom. Four signal kinds implement a graduated sense-respond nervous system: squelch (steward discretion), correction (epistemic, references evidence), retraction (author-withdrawal), quarantine (governance-collective). Source: genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md §5.1
 */
export type FeedbackSignal = {
  [k: string]: unknown;
} & {
  /**
   * CIDv1 (dag-cbor sha256) of the content EPR being acted on.
   */
  targetCid: string;
  /**
   * Graduated signal kind. squelch: steward discretion — do not propagate further. correction: epistemic — claim was wrong; evidence_cid required. retraction: author-withdrawal — terminates propagation chain. quarantine: governance-collective determination — structural cost imposed; requires mishpat/qahal authorization. vouch: positive-polarity signal — accept-correction (author publicly accepts a correction; standing recovery) or restitution (third party attests reparative work toward a damaged target).
   */
  signalKind: 'squelch' | 'correction' | 'retraction' | 'quarantine' | 'vouch';
  /**
   * Required iff signalKind == 'vouch'. Distinguishes vouch sub-semantics. accept-correction: author publicly accepts a previously-issued correction, partially recovering standing. restitution: third party attests reparative work; provides standing recovery proportional to the original debit.
   */
  vouchKind?: 'accept-correction' | 'restitution';
  /**
   * CIDv1 of a Correction EPR with claims and citations. Required when signalKind is 'correction'; optional (and typically absent) for other kinds.
   */
  evidenceCid?: string;
  /**
   * Graduated standing impact. advisory: informational — no score change, used for squelch and speculative corrections. debit-soft: standing debit recorded; reversal possible on successful appeal. debit-firm: standing debit applied immediately; requires mishpat authorization for reversal.
   */
  standingImpact: 'advisory' | 'debit-soft' | 'debit-firm';
  /**
   * Base64-encoded ed25519 public key of the issuing agent.
   */
  signedBy: string;
  /**
   * Base64-encoded ed25519 signature over canonical_bytes(targetCid || signalKind || evidenceCid? || standingImpact || signedBy).
   */
  signature: string;
  /**
   * OPTIONAL notification reference (accountable-correction contract §4). Carries the SIGNED ACT the semantic payload is about, so a receiver can fetch and verify rather than trust the claim. Identity is (originDnaHash, actionHash) — the signed act, not the entry hash: two authors' identical corrections share an entry hash and are two acts. Additive: absent on every pre-slice-1 message and ignored by a pre-slice-1 receiver, so no protocol-version bump is required and mixed-version peers never drop a correction.
   */
  actRef?: {
    /**
     * The content-cell DNA hash the act lives in. A receiver whose own content cell names a different DNA hash REJECTS the notification; cross-context evidence is not slice 1.
     */
    originDnaHash: string;
    /**
     * Base64 ActionHash of the FeedbackSignal action.
     */
    actionHash: string;
    /**
     * The back-propagation routing key, carried SEPARATELY from the act reference: back_prop's predecessor key is targetCid as a string, and resolving an action to its Content.id does not by itself yield that key.
     */
    routingKey: string;
  };
};
