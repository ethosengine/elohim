/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/observation-stream-view.schema.json -- DO NOT EDIT */

/**
 * Source of truth: the requester's own agent-private observations on their node (the observations projection of their append-only log, Private, Category B); this view is rendered per request from those rows and never persisted (Operational, Category C). Response to GET /api/v1/observations/stream?asOf&window&lens&kind — the person's own lifestream, arranged by the observation-lifestream recipe (audience self: only rows whose observer is the requester). recipe names the recipe and its content address so the page prints its provenance. omissions are named lines for what the view could not show or vouch for: the absent signature, rows outside the window, payloads that did not parse.
 */
export interface ObservationStreamView {
  /**
   * Unix epoch seconds the window ends at.
   */
  asOf: number;
  /**
   * How far back from asOf the view reaches: N days (Nd) or N hours (Nh).
   */
  window: string;
  recipe: RecipeRefView;
  /**
   * The recipe lens applied — a key of the recipe's lens table (all, content, long-dwell).
   */
  lens: string;
  /**
   * The requester's own observations inside the window, ranked by the recipe: newest first, then longest dwell.
   */
  entries: ObservationStreamEntryView[];
  /**
   * One named line per omission, e.g. 'signature: absent — …'. Present even when empty.
   */
  omissions: string[];
  /**
   * How many of the requester's observations the lens selected before the window; entries never exceed it.
   */
  totalCount: number;
}
/**
 * The recipe the view was rendered through.
 */
export interface RecipeRefView {
  /**
   * The recipe id, e.g. observation-lifestream.
   */
  name: string;
  /**
   * BLAKE3 over the recipe's governed bytes.
   */
  cid: string;
}
/**
 * One observation in the stream.
 */
export interface ObservationStreamEntryView {
  /**
   * Unix epoch seconds on the observer's clock.
   */
  observedAt: number;
  /**
   * The manifest-declared observation kind.
   */
  kind: string;
  /**
   * What was observed. Null only when the observation named no subject (a single provenance).
   */
  subjectCid: string | null;
  /**
   * The subject's title when known; omitted otherwise, never null.
   */
  title?: string;
  /**
   * Time the person spent with the subject, in milliseconds (0 when the payload did not parse, named in omissions).
   */
  dwellMs: number;
  /**
   * The deepest point reached, as a percentage of the page.
   */
  scrollDepthPct: number;
}
