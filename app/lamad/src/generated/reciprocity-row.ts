/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/reciprocity-row.schema.json -- DO NOT EDIT */

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
