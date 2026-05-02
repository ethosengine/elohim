/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/peer-household-edge.schema.json -- DO NOT EDIT */

/**
 * Per-household reciprocation edge in a peer topology view. Source of truth: peer_identity_bindings (Category A from imagodei DHT) joined with rea_commitments + economic_events for hosting accounting (Operational, Category C). Reconstructed per request; not persisted.
 */
export interface PeerHouseholdEdge {
  /**
   * Stable identifier for the counterparty household.
   */
  householdId: string;
  /**
   * Human-readable label for the household, when known.
   */
  displayName?: string;
  /**
   * Whether at least one peer in the household is currently online.
   */
  online: boolean;
  /**
   * Seconds since the last successful sync with the counterparty.
   */
  lastSyncSec?: number;
  /**
   * Count of viewer's CIDs currently replicated by the counterparty household.
   */
  myCidsHostedByThem: number;
  /**
   * Count of counterparty CIDs currently replicated by the viewer's household.
   */
  theirCidsHostedByMe: number;
  /**
   * Signed difference (theirCidsHostedByMe - myCidsHostedByThem).
   */
  netDiff?: number;
  /**
   * True when this household holds CIDs for which the viewer would lose sole-replica protection if it goes dark.
   */
  isCriticalForMe?: boolean;
  /**
   * True when the viewer is the only off-household replica of CIDs the counterparty owns.
   */
  iAmCriticalForThem?: boolean;
}
