import { BUNDLE_ROUTE_CLAIMS, RouteClaimTemplate } from '../../../generated/route-claims';

/**
 * Where each claiming bundle is mounted on this doorway. Composition-root
 * config until the doorway's pretty-mount resolver (§12.6 Slice 3) makes the
 * client-side lens unnecessary. Default for an unlisted bundle: `/<bundle>`.
 */
export const BUNDLE_MOUNTS: Readonly<Record<string, string>> = {
  lamad: '/lamad', // route-literal-ok: composition-root config (bundle mount table), not a minted route
};

/**
 * Which bundle edits an atom, and its editor's route template under that
 * bundle's mount. Composition-root config beside BUNDLE_MOUNTS: the route
 * claims declare where a type is RENDERED, not where it is edited, so the
 * editor surface is named here until a bundle manifest declares it.
 */
export const BUNDLE_EDITORS: Readonly<Record<string, string>> = {
  lamad: 'resource/{id}/edit', // route-literal-ok: composition-root config (bundle editor table), not a minted route
};

/** Generated path-step ids are positions on a path, not atoms anyone edits. */
const GENERATED_STEP_ID = /^path-.*-step-\d+$/;

/** "Edit in <Bundle>" for an atom, or null for a generated step id or no editor. */
export function editInBundle(
  id: string,
  editors: Readonly<Record<string, string>> = BUNDLE_EDITORS,
  mounts: Readonly<Record<string, string>> = BUNDLE_MOUNTS
): BundleLens | null {
  if (!id || GENERATED_STEP_ID.test(id)) return null;
  const entry = Object.entries(editors)[0];
  if (!entry) return null;
  const [bundle, template] = entry;
  const mount = mounts[bundle] ?? `/${bundle}`;
  return {
    href: `${mount}/${template.replace('{id}', encodeURIComponent(id))}`,
    bundleName: bundle.charAt(0).toUpperCase() + bundle.slice(1),
  };
}

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
