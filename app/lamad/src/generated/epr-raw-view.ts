/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/epr-raw-view.schema.json -- DO NOT EDIT */

/**
 * Raw inspector projection for GET /api/v1/epr/{cid}/raw — the EPR-content facing's fold over its materialized leg-neighborhood. Source of truth: existing epr_coupling table (read-only projection of notarized EPR atoms). Zero new DHT entry types, zero new tables, zero migrations — Category C per p2p-design-gate.
 */
export interface EprRawView {
  /**
   * CIDv1 base32 of the focal atom
   */
  cid: string;
  /**
   * Forward coupling legs pivoted from the focal atom's epr_coupling rows. The unmodelled process leg is dropped (charter Slice 1 owns it) — /raw does not advertise 4-of-4 coverage.
   */
  coupling: {
    /**
     * CIDv1 base32 of the knowledge-leg target
     */
    knowledge?: string | null;
    /**
     * CIDv1 base32 of the value-leg target
     */
    value?: string | null;
    /**
     * CIDv1 base32 of the governance-leg target
     */
    governance?: string | null;
  };
  /**
   * Count of DISTINCT forward coupling targets across all legs (a multi-leg coupling to one target counts once)
   */
  neighborhoodDegree: number;
  /**
   * DISTINCT source atoms that couple TO this atom (inbound part-of set), sorted for deterministic wire order
   */
  reversePartOf: string[];
}
