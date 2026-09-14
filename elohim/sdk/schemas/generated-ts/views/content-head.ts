/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/content-head.schema.json -- DO NOT EDIT */

/**
 * The notary-declared HEAD of a content id's version DAG (HEAD-election, Plan C3 / notary-authority Leg 3). Returned by GET/POST /db/content/{id}/head. Source of truth: DHT (Notarized HEAD election, Category A). Projected from the content row's notary markers; the surface exists only for a row carrying a notary answer (a declared head OR a DHT anchor).
 */
export interface ContentHeadView {
  /**
   * The content id whose HEAD this is.
   */
  contentId: string;
  /**
   * The action hash the notary holds as this id's current HEAD. Prefers the explicitly-declared HEAD; falls back to the DHT anchor when no explicit declaration is stamped.
   */
  headActionHash: string;
  /**
   * true iff an explicit declared HEAD was set (an author moved the HEAD via the declare authority); false when the answer is the DHT-anchor fallback (the single-author implicit head).
   */
  declared: boolean;
  /**
   * The DHT anchor for the resolved row when notarized; null when the HEAD answer rests only on a declared head with no anchor yet.
   */
  dhtAnchorHash?: string | null;
  /**
   * REQ-F10 trust legibility label — same vocabulary as ContentView.trust: 'notarized' (green/DHT-notarized) | 'published' (peer-attested) | 'unconfirmed' (amber/CRDT-converged-only or all-null). Never an authority/attribution source.
   */
  trust: string;
  /**
   * The serving blob hash of the resolved row (browser bundle), if any.
   */
  blobHash?: string | null;
  /**
   * When the resolved row was last written (mirrors ContentView.updatedAt).
   */
  updatedAt?: string | null;
  /**
   * The STAGING canonical-head declaration standing beneath the earned winner — the next version awaiting promotion, addressed by the ActionHash of its DECLARATION (not by a CID). Derived by content_store::select_staging_candidate as a pure function of the same link set every peer holds. Present only when the winner is EARNED and a staging declaration postdates it; stagingCandidateState distinguishes authoritative absence from an unavailable ask.
   */
  stagingCandidate?: string | null;
  /**
   * The blob content address named by stagingCandidate when this peer can resolve that exact declaration through its content projection and holds the bytes locally. Absent/null never authorizes fallback to the converged blobHash.
   */
  stagingCandidateBlobHash?: string | null;
  /**
   * Epistemic status of the staging-candidate read. staged names an authoritative declaration, none is an authoritative withdrawal/absence, and unavailable means the conductor ask could not be put. Missing is an older-peer response and is not authoritative absence.
   */
  stagingCandidateState?: 'staged' | 'none' | 'unavailable' | null;
}
