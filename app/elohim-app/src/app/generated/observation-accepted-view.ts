/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/observation-accepted-view.schema.json -- DO NOT EDIT */

/**
 * Source of truth: the observer's own append-only observation log on their node (Private, Category B); this ack is its head after the append (Operational, Category C — returned once per write, not persisted apart from the log head). Response to POST /api/v1/observations. It names what the row honestly is: observerCidNamespace 'as-asserted' says the observer is the caller's header identifier verbatim (the browser path sends a session human id, the desktop path an agent key; the two namespaces are named here, not reconciled), and signed 'absent' says the row carries no signature until the signing graduation. Both are single-valued today by design; the graduation widens them.
 */
export interface ObservationAcceptedView {
  /**
   * The observer: the X-Agent-Cid header, verbatim.
   */
  observerCid: string;
  /**
   * How observerCid was established: as the caller asserted it.
   */
  observerCidNamespace: 'as-asserted';
  /**
   * The observer's log root after this append.
   */
  logCid: string;
  /**
   * This observation's position in the observer's log (0-based).
   */
  logOffset: number;
  /**
   * The observer's own sequence number for this observation (1-based).
   */
  seq: number;
  /**
   * Whether the row carries the observer's signature. Absent until the signing graduation.
   */
  signed: 'absent';
}
