/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/epr-providers-view.schema.json -- DO NOT EDIT */

/**
 * Source of truth: peer providers advertising that they hold the atom at the given cid. Phase 2a returns ['local'] when held locally, [] otherwise. Phase 2c extends with Kad DHT provider records. Category C — operational (DHT query, reconstructed per request).
 */
export interface EprProvidersView {
  /**
   * CIDv1 base32 of the queried atom
   */
  cid: string;
  providers: string[];
}
