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
}
