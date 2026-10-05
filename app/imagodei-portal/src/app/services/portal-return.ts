/**
 * Where the portal may send a person back to after they sign in: a page of
 * this portal, on this origin. Carried as `?return_to=`; anything that could
 * leave the portal (another origin, `//host`, `/\host`, a path outside the
 * portal's base) is refused, so a crafted sign-in link cannot bounce a
 * person elsewhere.
 */

/** Query parameter carrying the page to come back to. */
export const RETURN_TO_PARAM = 'return_to';

/** The return target if it is a page of this portal, else null. */
export function safePortalReturn(value: unknown, portalBase: string): string | null {
  if (typeof value !== 'string' || !value.startsWith('/')) return null;
  if (value.startsWith('//') || value.startsWith('/\\')) return null;
  return value.startsWith(portalBase) ? value : null;
}

/** The portal's sign-in page, remembering `here` to come back to. */
export function signInUrl(portalBase: string, here: string): string {
  return `${portalBase}?${RETURN_TO_PARAM}=${encodeURIComponent(here)}`;
}
