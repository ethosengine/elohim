/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/key-revocation.schema.json -- DO NOT EDIT */

/**
 * Read-optimized projection of an elohim-DNA Content key-revocation entry (content_type 'governance-action:key-revocation'). Source of truth: DHT. Rebuildable via signal replay on the ElohimContentSignal dispatcher.
 */
export interface KeyRevocationView {
  /**
   * Hex-encoded Holochain ActionHash of the source Content entry.
   */
  dhtAnchorHash: string;
  /**
   * Coordinator-generated ID (primary key of the projection).
   */
  id: string;
  /**
   * String id of the human whose key is being revoked.
   */
  subjectHumanId: string;
  /**
   * The AgentPubKey being revoked (stringified).
   */
  revokedKey: string;
  /**
   * One of REVOCATION_REASONS: compromised, stolen, challenge_upheld, voluntary.
   */
  reason: string;
  /**
   * How the revocation was initiated.
   */
  triggerType: 'voluntary' | 'steward_vote' | 'challenge' | 'specialist_attestation';
  /**
   * CID of the initiating agent (was: initiatedBy).
   */
  initiatedByCid: string;
  /**
   * Votes required to reach threshold. 1 for voluntary; >=2 for steward_vote/challenge.
   */
  requiredVotes: number;
  /**
   * Approved votes counted so far.
   */
  currentVotes: number;
  /**
   * True when currentVotes >= requiredVotes and the revocation is effective.
   */
  thresholdReached: boolean;
  /**
   * ISO-8601 timestamp when revocation became effective. Null while pending.
   */
  effectiveAt?: string | null;
  /**
   * EPR W2D: ISO-8601 timestamp when compromise was derived (specialist attestation, challenge upheld, etc). Null if not applicable.
   */
  derivedCompromiseAt?: string | null;
  /**
   * ISO-8601 timestamp of initial commit.
   */
  createdAt: string;
  /**
   * ISO-8601 timestamp of last update (e.g., when threshold was reached).
   */
  updatedAt: string;
}
