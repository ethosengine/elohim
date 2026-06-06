import type { RouteClaimTemplate } from '@elohim/service';

/**
 * lamad's DECLARED route claims (spec §3.1 — the bundle's request; the
 * steward's project-epr grant activates them doorway-side). Keep in sync with
 * the granted shape in genesis/seeder/src/seed-projections.ts (lamadAt) —
 * drift between declaration and grant is the spec §3.4 claims-stale condition.
 */
export const LAMAD_ROUTE_CLAIMS: readonly RouteClaimTemplate[] = [
  { contentType: 'path', template: 'path/{id}', fragments: { step: 'path/{id}/step/{n}' } },
];
