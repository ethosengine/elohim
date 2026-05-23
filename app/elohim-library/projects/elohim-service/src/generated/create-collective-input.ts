/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: inputs/create-collective-input.schema.json -- DO NOT EDIT */

/**
 * Source of truth: Cat-A Collective DHT entry in imagodei DNA (DNA-notarized at commit via integrity validator). Category C — HTTP wire-shape input projecting onto the create_collective coordinator flow. Body for POST /api/v1/collective.
 */
export interface CreateCollectiveInput {
  charter: string;
  displayName: string;
  salt: string;
}
