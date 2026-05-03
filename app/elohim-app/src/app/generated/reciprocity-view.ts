/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/reciprocity-view.schema.json -- DO NOT EDIT */

/**
 * Per-agent reciprocity ledger — inflow (others hosting for me) + outflow (me hosting for others) + net + capacity. Source of truth: derived from rea_commitments and economic_events using the T03d action conventions (Operational, Category C). Reconstructed per request; not persisted.
 */
export interface ReciprocityView {
  /**
   * Agent CID this reciprocity view is scoped to.
   */
  agentCid: string;
  /**
   * Reciprocity rows where the counterparty hosts content for the viewer.
   */
  inflow: ReciprocityRow[];
  /**
   * Reciprocity rows where the viewer hosts content for the counterparty.
   */
  outflow: ReciprocityRow1[];
  /**
   * Signed net hosting position. Positive = others hold more for me than I do for them.
   */
  netHostedBytes: number;
  /**
   * Spare capacity the viewer can offer to expand reciprocity.
   */
  capacityAvailableBytes: number;
}
/**
 * Per-counterparty row in a reciprocity view's inflow/outflow ledger. Source of truth: rea_commitments + economic_events (Operational, Category C — derived projection). Reconstructed per request; not persisted.
 */
export interface ReciprocityRow {
  /**
   * Household identifier on the other side of the reciprocity ledger.
   */
  counterpartyHouseholdId: string;
  /**
   * Human-readable label for the counterparty household.
   */
  displayName?: string;
  /**
   * Bytes committed via REA Commitment entries.
   */
  committedBytes: number;
  /**
   * Bytes actually delivered (covered by economic_event observations).
   */
  deliveredBytes: number;
  /**
   * deliveredBytes / committedBytes as a fraction (0.0-1.0+; can exceed 1 when over-delivered).
   */
  honoredPercent: number;
  /**
   * Whether the counterparty has an online peer right now.
   */
  online?: boolean;
}
/**
 * Per-counterparty row in a reciprocity view's inflow/outflow ledger. Source of truth: rea_commitments + economic_events (Operational, Category C — derived projection). Reconstructed per request; not persisted.
 */
export interface ReciprocityRow1 {
  /**
   * Household identifier on the other side of the reciprocity ledger.
   */
  counterpartyHouseholdId: string;
  /**
   * Human-readable label for the counterparty household.
   */
  displayName?: string;
  /**
   * Bytes committed via REA Commitment entries.
   */
  committedBytes: number;
  /**
   * Bytes actually delivered (covered by economic_event observations).
   */
  deliveredBytes: number;
  /**
   * deliveredBytes / committedBytes as a fraction (0.0-1.0+; can exceed 1 when over-delivered).
   */
  honoredPercent: number;
  /**
   * Whether the counterparty has an online peer right now.
   */
  online?: boolean;
}
