// AUTO-GENERATED from app manifest: elohim/sdk/domains/imagodei/manifest.json
// DO NOT EDIT — regenerate with: pnpm run imagodei:codegen

export const IMAGODEI_CONTENT_TYPES = ['human', 'role', 'contributor'] as const;
export type ImagodeiContentType = (typeof IMAGODEI_CONTENT_TYPES)[number];

export const IMAGODEI_RELATIONSHIPS = [
  'IDENTIFIES',
  'IDENTIFIED_BY',
  'ASSIGNED_TO',
  'GRANTS',
  'SCOPED_TO',
  'STEWARDS',
  'ATTESTS',
  'PARTICIPATES_IN',
] as const;
export type ImagodeiRelationship = (typeof IMAGODEI_RELATIONSHIPS)[number];

export const IMAGODEI_SIGNALS = [
  'identity-created',
  'presence-established',
  'attestation-granted',
  'attestation-revoked',
  'agency-progressed',
  'relationship-formed',
] as const;
export type ImagodeiSignal = (typeof IMAGODEI_SIGNALS)[number];
