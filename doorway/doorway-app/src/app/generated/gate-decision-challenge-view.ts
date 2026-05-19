/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/gate-decision-challenge-view.schema.json -- DO NOT EDIT */

/**
 * Source of truth: DHT (mishpat DNA, GateDecisionChallenge entry, Notarized, Category A). A formal challenge filed against a GateDecisionAttestation. This record is a read-optimised projection populated by the MishpatSignal::GateDecisionChallengeCreated post-commit signal. If this projection and the DHT disagree, the DHT wins.
 */
export interface GateDecisionChallengeView {
  /**
   * CID of this challenge (self-addressing, globally unique)
   */
  challengeId: string;
  /**
   * CID of the GateDecisionAttestation being challenged
   */
  challengedDecisionCid: string;
  /**
   * AgentPubKey (base64) of the challenger
   */
  challengerId: string;
  /**
   * Grounds on which the challenge is filed
   */
  grounds: 'factual-error' | 'safety' | 'policy' | 'constitutional' | 'indemnification-request';
  /**
   * Challenger's articulation of the grievance
   */
  summary: string;
  /**
   * Content-addressed evidence refs (comma-separated CIDs; empty string if none)
   */
  evidenceRefs: string;
  /**
   * ISO 8601 timestamp of when the challenge was filed
   */
  filedAt: string;
  /**
   * Reach level of the challenged decision's impact
   */
  reach: 'self' | 'intimate' | 'community' | 'commons';
  /**
   * ActionHash (base64) of the upstream DHT entry — client can use this to verify provenance
   */
  dhtAnchorHash: string;
  /**
   * ISO 8601 timestamp of when this projection row was inserted
   */
  createdAt: string;
}
