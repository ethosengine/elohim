/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: inputs/withdraw-membership-input.schema.json -- DO NOT EDIT */

/**
 * Source of truth: Cat-A Membership DHT entry in imagodei DNA (DNA-notarized update setting withdrawn_at_block_height). Category C — HTTP wire-shape input projecting onto the withdraw_membership_clean coordinator flow. Body for POST /api/v1/collab/{cid}/withdraw.
 */
export interface WithdrawMembershipInput {
  membershipCid: string;
  collabQahalCid: string;
}
