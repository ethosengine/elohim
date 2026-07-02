// AUTO-GENERATED from app manifest: elohim/sdk/domains/lamad/manifest.json
// DO NOT EDIT — regenerate with: pnpm run route-claims:codegen

/** Serializable claim shape (spec §3.1) — what bundle manifests DECLARE. */
export interface RouteClaimTemplate {
  contentType: string;
  template: string;
  fragments?: Record<string, string>;
}

/** lamad's DECLARED route claims (manifest `routeClaims`). */
export const LAMAD_ROUTE_CLAIMS: readonly RouteClaimTemplate[] = [
  { contentType: 'path', template: 'path/{id}', fragments: { step: 'path/{id}/step/{n}' } },
];

/** Whether this bundle owns the universal /epr route (manifest `ownsUniversalRoute`). */
export const LAMAD_OWNS_UNIVERSAL_ROUTE = false;
