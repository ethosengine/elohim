// AUTO-GENERATED from app manifest: elohim/sdk/domains/qahal/manifest.json
// DO NOT EDIT — regenerate with: pnpm run qahal:codegen

export const QAHAL_CONTENT_TYPES = [
  'collective',
  'proposal',
  'challenge',
  'appeal',
  'statement',
  'post',
  'event',
  'group',
  'message',
  'thread',
] as const;
export type QahalContentType = (typeof QAHAL_CONTENT_TYPES)[number];

export const QAHAL_RELATIONSHIPS = [
  'CONTAINS',
  'BELONGS_TO',
  'CHALLENGES',
  'APPEALS',
  'RELATES_TO',
] as const;
export type QahalRelationship = (typeof QAHAL_RELATIONSHIPS)[number];

export const QAHAL_SIGNALS = [
  'governance-decision',
  'community-report',
  'challenge-filed',
  'appeal-filed',
  'consensus-reached',
  'social-engagement',
  'relationship-formed',
] as const;
export type QahalSignal = (typeof QAHAL_SIGNALS)[number];
