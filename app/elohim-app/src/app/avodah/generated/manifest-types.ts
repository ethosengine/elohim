// AUTO-GENERATED from app manifest: elohim/sdk/domains/avodah/manifest.json
// DO NOT EDIT — regenerate with: pnpm run avodah:codegen

export const AVODAH_CONTENT_TYPES = ['work-story', 'work-project'] as const;
export type AvodahContentType = (typeof AVODAH_CONTENT_TYPES)[number];

export const AVODAH_RELATIONSHIPS = ['CONTAINS', 'BELONGS_TO', 'DEPENDS_ON', 'REQUIRES'] as const;
export type AvodahRelationship = (typeof AVODAH_RELATIONSHIPS)[number];

export const AVODAH_SIGNALS = ['task-completed', 'sprint-completed', 'cadence-reset'] as const;
export type AvodahSignal = (typeof AVODAH_SIGNALS)[number];
