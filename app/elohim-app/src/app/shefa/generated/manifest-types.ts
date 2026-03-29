// AUTO-GENERATED from app manifest: elohim/sdk/domains/shefa/manifest.json
// DO NOT EDIT — regenerate with: pnpm run shefa:codegen

export const SHEFA_CONTENT_TYPES = ['stewardship-context'] as const;
export type ShefaContentType = (typeof SHEFA_CONTENT_TYPES)[number];

export const SHEFA_RELATIONSHIPS = ['STEWARDS', 'STEWARDED_BY', 'FUNDS', 'OBLIGATES'] as const;
export type ShefaRelationship = (typeof SHEFA_RELATIONSHIPS)[number];

export const SHEFA_SIGNALS = [
  'economic-event-recorded',
  'stewardship-allocated',
  'resource-transferred',
  'obligation-fulfilled',
  'custodian-attestation',
  'insurance-claim',
] as const;
export type ShefaSignal = (typeof SHEFA_SIGNALS)[number];
