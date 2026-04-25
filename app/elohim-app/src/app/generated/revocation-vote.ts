/* Generated from protocol schema: views/revocation-vote.schema.json -- DO NOT EDIT */

/**
 * Read-optimized projection of an imagodei RevocationVote DHT entry. Source of truth: DHT. Rebuildable via signal replay on RecoveryV2Signal::RevocationVoteSubmitted.
 */
export interface RevocationVoteView {
  /**
   * Holochain ActionHash of the RevocationVote entry (base64).
   */
  dhtAnchorHash: string;
  /**
   * Coordinator-generated vote ID.
   */
  id: string;
  /**
   * dhtAnchorHash of the parent KeyRevocation entry.
   */
  revocationDhtAnchorHash: string;
  /**
   * Coordinator-generated ID of the parent KeyRevocation.
   */
  revocationId: string;
  /**
   * human_id of the voting steward.
   */
  stewardId: string;
  /**
   * True if the steward approves the revocation.
   */
  approved: boolean;
  /**
   * Non-empty human-readable justification for the vote.
   */
  attestation: string;
  /**
   * ISO-8601 timestamp of vote submission.
   */
  votedAt: string;
}
