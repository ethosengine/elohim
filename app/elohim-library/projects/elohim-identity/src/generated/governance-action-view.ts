/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/governance-action-view.schema.json -- DO NOT EDIT */

/**
 * Source of truth: Holochain DHT (elohim DNA, Content entry, content_type LIKE 'governance-action:%', Category A per p2p-design-gate). Parent entry; vote attestations reference its CID as parentGovernanceActionCid. If this projection and the DHT disagree, the DHT wins.
 */
export interface GovernanceActionView {
  /**
   * CID of this governance action (content-derived identity)
   */
  id: string;
  /**
   * ActionHash (hex) of the DHT entry — provenance anchor
   */
  dhtAnchorHash: string;
  /**
   * Discriminator matching governance-action:<subtype>
   */
  governanceKind: string;
  /**
   * CID of the entity being acted upon
   */
  subjectCid: string;
  /**
   * CID of the proposing agent
   */
  proposerCid: string;
  /**
   * Serialised threshold JSON (e.g. {"m":3} or {"percentage":0.51})
   */
  thresholdJson: string;
  /**
   * Serialised eligibility predicate JSON, if constrained
   */
  eligibilityPredicateJson?: string | null;
  /**
   * Ballot format: approve-reject | ranked-choice | weighted
   */
  ballotFormat: string;
  /**
   * ISO 8601 close time for the voting window
   */
  closesAt: string;
  /**
   * Serialised additional parameters JSON, if any
   */
  parametersJson?: string | null;
  /**
   * Human-readable title
   */
  title: string;
  /**
   * Optional description
   */
  description?: string | null;
  /**
   * ISO 8601 creation timestamp
   */
  createdAt: string;
}
