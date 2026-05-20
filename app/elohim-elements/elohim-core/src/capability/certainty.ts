/**
 * Content Certainty — the content-side observable, parallel to CapabilityProfile.
 *
 * Describes the *content being rendered*, not the viewer. In a P2P / EPR / CID world,
 * "data is loaded" does not mean "data is true." Elements observe this to render with
 * epistemic honesty.
 *
 * See: genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md §4
 */

export type CertaintyState =
  | 'canonical'   // fresh, fully reconciled, multiple confirming attestations
  | 'partial'     // syncing in progress; some peers haven't responded yet
  | 'stale'       // last reconciled long ago; may be out of date
  | 'contested'   // multiple attestations disagree; no canonical view exists
  | 'unreachable' // no peer currently serving; local cache only
  | 'unknown';    // freshly opened, not yet probed

export interface ContentCertainty {
  state: CertaintyState;
  /** ms since last reconciliation against any peer */
  freshness?: number;
  /** how many witnesses have signed this content */
  attestationCount?: number;
  /** hops from viewer to nearest steward of this content */
  reachDistance?: number;
  /** attestation IDs disagreeing with the canonical view */
  contestedBy?: string[];
  /** which peers have served this content */
  sourcePeers?: string[];
}

/** The safe default when no certainty has been provided. Elements MUST NOT render
 *  canonical visuals when their certainty is unknown. */
export const UNKNOWN_CERTAINTY: ContentCertainty = Object.freeze({
  state: 'unknown',
});
