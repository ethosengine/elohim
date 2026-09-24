/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: inputs/observation-intent.schema.json -- DO NOT EDIT */

/**
 * Source of truth: the observer's own append-only observation log on their node (Private, Category B — agent-scoped, never gossiped for an agent-private kind). HTTP wire-shape input: body for POST /api/v1/observations. The observer is the explicit X-Agent-Cid header, verbatim; observerCid is accepted only to be checked — equal to the header it is redundant, any other value is refused with 403. observationKind must be declared by a pillar manifest (400 otherwise). payloadJson is the pre-stringified payload, validated against the kind's manifest-declared field map (400 with the reason). observedAt is unix epoch seconds on the observer's clock; the server stamps it when absent.
 */
export interface ObservationIntent {
  /**
   * Optional; must equal the X-Agent-Cid header.
   */
  observerCid?: string;
  /**
   * A manifest-declared observation kind, e.g. lamad:content-viewed.
   */
  observationKind: string;
  /**
   * Content-addressed identifier of what was observed. Omitted when the observation has no subject.
   */
  subjectCid?: string;
  /**
   * What kind of thing the subject is (e.g. content). Omitted when there is no subject.
   */
  subjectKind?: string;
  /**
   * Unix epoch seconds on the observer's clock. Omitted → stamped by the server at receipt.
   */
  observedAt?: number;
  /**
   * The payload as a JSON string, shaped by the kind's declared field map.
   */
  payloadJson: string;
}
