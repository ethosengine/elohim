// AUTO-GENERATED from app manifest: elohim/sdk/domains/elohim/manifest.json
// DO NOT EDIT — regenerate with: pnpm run route-claims:codegen

/** Serializable claim shape (spec §3.1) — what bundle manifests DECLARE. */
export interface RouteClaimTemplate {
  contentType: string;
  template: string;
  fragments?: Record<string, string>;
}

/** elohim's DECLARED route claims (manifest `routeClaims`). */
export const ELOHIM_ROUTE_CLAIMS: readonly RouteClaimTemplate[] = [];

/** Whether this bundle owns the universal /epr route (manifest `ownsUniversalRoute`). */
export const ELOHIM_OWNS_UNIVERSAL_ROUTE = true;

/** Every declaring bundle's claims — the universal-route owner mints cross-bundle lenses from these. */
export const BUNDLE_ROUTE_CLAIMS: readonly {
  bundle: string;
  claims: readonly RouteClaimTemplate[];
}[] = [
  {
    bundle: 'lamad',
    claims: [
      { contentType: 'path', template: 'path/{id}', fragments: { step: 'path/{id}/step/{n}' } },
    ],
  },
];
