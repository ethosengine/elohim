/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/key-revocation.schema.json -- DO NOT EDIT */

/**
 * Read-optimized projection of an imagodei KeyRevocation DHT entry. Source of truth: DHT. Rebuildable via signal replay on RecoveryV2Signal::KeyRevocationRequested/Effective.
 */
export interface KeyRevocationView {
  /**
   * Holochain ActionHash of the KeyRevocation entry (base64).
   */
  dhtAnchorHash: string;
  /**
   * Coordinator-generated ID.
   */
  id: string;
  /**
   * String id of the human whose key is being revoked.
   */
  humanId: string;
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
  triggerType: 'voluntary' | 'steward_vote' | 'challenge';
  /**
   * human_id of the initiating agent.
   */
  initiatedBy: string;
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
   * ISO-8601 timestamp of initial commit.
   */
  createdAt: string;
  /**
   * ISO-8601 timestamp of last update (e.g., when threshold was reached).
   */
  updatedAt: string;
}
