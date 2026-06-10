/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/projector-identity.schema.json -- DO NOT EDIT */

/**
 * Per-doorway projector row for a CID's distribution-details view. Source of truth: doorway projection registry observed via libp2p (Operational, Category C). Reconstructed per request; not persisted.
 */
export interface ProjectorIdentity {
  /**
   * Public hostname of the projecting doorway (e.g. 'matthew.elohim.host').
   */
  doorwayHostname: string;
  /**
   * Seconds since the projector last acknowledged this CID.
   */
  lastAckSeconds: number;
  /**
   * Coarse region tier of the doorway, when known.
   */
  regionTier?: string;
}
