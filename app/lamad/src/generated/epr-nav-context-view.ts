/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/epr-nav-context-view.schema.json -- DO NOT EDIT */

/**
 * Nav-context projection for GET /api/v1/epr/{cid}/nav-context. Source of truth: existing epr_atoms, epr_coupling, epr_supersedence, and epr_claims tables (read-only projection). Zero new DHT entry types, zero new tables, zero migrations — Category C per p2p-design-gate.
 */
export interface EprNavContextView {
  /**
   * CIDv1 base32 of the focal atom
   */
  cid: string;
  /**
   * Predecessor: atom this one supersedes (the one it replaced)
   */
  prev?: EprNavRef | null;
  /**
   * Successor: atom that superseded this one (what replaced it)
   */
  next?: EprNavRef | null;
  /**
   * Atoms that include this atom as a coupling target (reverse coupling)
   */
  partOf: EprNavRef[];
  /**
   * Outbound coupling targets plus claim CIDs from this atom
   */
  related: EprNavRef[];
  /**
   * Relationship kinds that contributed entries to this view (e.g. supersedence, coupling, claims)
   */
  derivedFrom: string[];
}
/**
 * A lightweight reference to a related EPR atom. Label and resilience tier are best-effort — populated when the referenced atom is locally held; absent otherwise.
 */
export interface EprNavRef {
  /**
   * CIDv1 base32 of the referenced atom
   */
  cid: string;
  /**
   * Short human-readable label, populated when the atom is locally held
   */
  label?: string;
  /**
   * Resilience tier derived from the atom's reach field
   */
  resilienceTier?: 'high' | 'medium' | 'low' | 'unknown';
}
