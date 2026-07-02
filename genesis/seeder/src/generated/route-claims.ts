// AUTO-GENERATED from app manifests: elohim/sdk/domains/*/manifest.json
// DO NOT EDIT — regenerate with: pnpm run route-claims:codegen

/** Serializable claim shape (spec §3.1) — what bundle manifests DECLARE. */
export interface RouteClaimTemplate {
  contentType: string;
  template: string;
  fragments?: Record<string, string>;
}

/** lamad's DECLARED route claims (manifest `routeClaims`). */
export const LAMAD_ROUTE_CLAIMS: RouteClaimTemplate[] = [
  { contentType: 'path', template: 'path/{id}', fragments: { step: 'path/{id}/step/{n}' } },
];
