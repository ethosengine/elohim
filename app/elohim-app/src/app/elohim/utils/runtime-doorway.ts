/**
 * The host app selects its HTTP doorway; a packaged browser app is served by
 * that doorway, and a development server proxies the same HTTP surface.
 * Native shells retain their configured fallback; SSR has no serving browser.
 * This resolves configuration only and never rewrites an external request URL.
 */
export function resolveDoorwayUrl(configured: string | undefined): string {
  // eslint-disable-next-line no-restricted-syntax -- SSR-safe optional location access; no location means the configured fallback is retained.
  const origin = globalThis.location?.origin;
  if (!('__TAURI__' in globalThis) && origin && /^https?:\/\//.test(origin)) {
    return origin;
  }
  return configured ?? 'http://localhost:8888';
}
