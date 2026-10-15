import { eprToUniversalHref } from '@elohim/service';

import { legFromFragment } from './epr-home.model';

import type { Params } from '@angular/router';

/** What the router hands a redirect function (a subset of PartialMatchRouteSnapshot). */
export interface LegacyRedirectInput {
  params: Params;
  queryParams: Params;
  fragment: string | null;
}

function queryString(queryParams: Params): string {
  const q = new URLSearchParams();
  for (const [key, value] of Object.entries(queryParams)) {
    const values: unknown[] = Array.isArray(value) ? value : [value];
    for (const v of values) {
      if (v !== undefined && v !== null) q.append(key, String(v));
    }
  }
  const s = q.toString();
  return s ? `?${s}` : '';
}

/**
 * The legacy `/resource/{id}` URL as the atom's universal address (spec
 * 2026-09-02 §11.1): minted through `eprToUniversalHref`, query params kept,
 * and the retired viewer's tab fragment mapped onto the leg that now carries
 * it (`network→holds`, `governance→governed`, `trust→from`, `content→` none;
 * leg anchors pass through; anything else is dropped).
 */
export function legacyResourceRedirect(input: LegacyRedirectInput): string {
  const id = String(input.params['resourceId'] ?? '');
  const leg = legFromFragment(input.fragment);
  return `${eprToUniversalHref({ id, tier: 'head' })}${queryString(input.queryParams)}${
    leg ? `#${leg}` : ''
  }`;
}
