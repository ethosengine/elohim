import { BUNDLE_ROUTE_CLAIMS, RouteClaimTemplate } from '../../../generated/route-claims';

/**
 * Where each claiming bundle is mounted on this doorway. Composition-root
 * config until the doorway's pretty-mount resolver (§12.6 Slice 3) makes the
 * client-side lens unnecessary. Default for an unlisted bundle: `/<bundle>`.
 */
export const BUNDLE_MOUNTS: Readonly<Record<string, string>> = {
  lamad: '/lamad', // route-literal-ok: composition-root config (bundle mount table), not a minted route
};

export interface BundleLens {
  href: string;
  bundleName: string;
}

export function openInBundle(
  contentType: string,
  id: string,
  claims: readonly {
    bundle: string;
    claims: readonly RouteClaimTemplate[];
  }[] = BUNDLE_ROUTE_CLAIMS,
  mounts: Readonly<Record<string, string>> = BUNDLE_MOUNTS
): BundleLens | null {
  for (const { bundle, claims: list } of claims) {
    const claim = list.find(c => c.contentType === contentType);
    if (!claim) continue;
    const mount = mounts[bundle] ?? `/${bundle}`;
    const path = claim.template.replace('{id}', encodeURIComponent(id));
    return {
      href: `${mount}/${path}`,
      bundleName: bundle.charAt(0).toUpperCase() + bundle.slice(1),
    };
  }
  return null;
}
