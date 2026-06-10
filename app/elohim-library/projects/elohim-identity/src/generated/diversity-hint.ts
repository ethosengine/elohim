/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/diversity-hint.schema.json -- DO NOT EDIT */

/**
 * Tagged variant carrying a peer-diversity hint for a CID's replica set. Source of truth: composed from peer_identity_bindings + libp2p swarm metadata (Operational, Category C). Value shape depends on `kind`: region_metro/household_archetypes carry an array of strings, collective_member_count carries an integer, none carries null.
 */
export interface DiversityHint {
  /**
   * Which diversity dimension the hint addresses.
   */
  kind: 'region_metro' | 'household_archetypes' | 'collective_member_count' | 'none';
  /**
   * Hint payload. Array of region/archetype strings, an integer count, or null/absent when kind=none. Serde tag/content adjacent tagging omits this key for unit variants.
   */
  value?: string[] | number | null;
}
