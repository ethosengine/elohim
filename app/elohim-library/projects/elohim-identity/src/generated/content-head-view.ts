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
   * true iff this peer's projection records that the election standing behind the declared HEAD is EARNED (content_store::select_canonical_winner's earned tier — a steward's earned canonical, which beats any staging declaration regardless of recency); false when no earned election is known here (a staging/unmarked election, or none recorded). A pipeline seed update can never become canonical over an earned head, so a seeder reads this and leaves stewarded atoms alone. Additive: absent from older serving nodes; a reader treats absence as not-known-earned.
   */
  earned: boolean;
  /**
   * The DHT anchor for the resolved row when notarized; null when the HEAD answer rests only on a declared head with no anchor yet.
   */
  dhtAnchorHash?: string | null;
  /**
   * REQ-F10 trust legibility label — same vocabulary as ContentView.trust: 'notarized' (green/DHT-notarized) | 'published' (peer-attested) | 'unconfirmed' (amber/CRDT-converged-only or all-null). Never an authority/attribution source.
   */
  trust: string;
  /**
   * Whether the row's DHT anchor is its declared HEAD: true when they are the same action, false when they differ, null when the row lacks either. A false does not by itself mean the served bytes are wrong (an anchor can advance without the pointer moving), but a torn row — declared head A, pointer written from action B — reads false here while trust still says 'notarized'. Additive: absent from older serving nodes.
   */
  anchorMatchesHead?: boolean | null;
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
  /**
   * Where earned came from. Present ONLY on a live-election read (GET /db/content/{id}/head?election=live); absent on a plain read, which keeps that response identical to one from a peer that predates it. live: this peer's own conductor answered its local canonical election and earned was read from it (true when the winner is EARNED and is the declared head; an earned tier already recorded here for the same head is never lowered by a local link view lacking the earned declaration) — a live answer that finds the projection behind also heals the projection's election columns in the same request, never the head. cached: the conductor could not answer inside the read's budget (absent, errored or timed out), so earned is the projection's recorded column.
   */
  earnedSource?: 'live' | 'cached';
  /**
   * The DHT timestamp (microseconds since the Unix epoch — same convention as ProjectionInventoryEntry.declaredHeadAt) of the canonical-head declaration link that select_canonical_winner elected, as this peer's projection recorded it (content.canonical_declared_at). The election clock: with canonicalLinkHash it lets a reader tell a stale election from a different one. null = no election recorded on this row (single provenance: the row answered). Additive: absent from older serving nodes.
   */
  canonicalDeclaredAt?: number | null;
  /**
   * The ELECTOR — the agent (Holochain AgentPubKey, u-prefixed base64) that signed the winning canonical-head declaration link — as this peer's own conductor answered it on a live-election read (?election=live) whose winner IS the declared head. Never projected or cached: absent on a plain read, absent when the conductor did not answer in budget, absent when the live winner is a different head or the coordinator predates the field. electorSource says which.
   */
  elector?: string;
  /**
   * Present ONLY on a live-election read. live: the conductor answered and named the elector. cached: no live elector was answered, and because the projection never stores an elector, elector is absent.
   */
  electorSource?: 'live' | 'cached';
  /**
   * The election's tiebreak (content.canonical_link_hash, Holochain u-prefixed base64): the winning declaration link, or the root-accepted head action for a delegated declaration. null = unknown (no election recorded, or one recorded before the tiebreak travelled). Additive: absent from older serving nodes.
   */
  canonicalLinkHash?: string | null;
}
