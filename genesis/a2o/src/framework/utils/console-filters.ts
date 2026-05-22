/**
 * Shared console/network noise filters for browser scenarios.
 *
 * Extracted from steps/ui/auth.steps.ts so the After hook in common.steps.ts
 * and individual step definitions share the same filtering logic.
 */

import type { CapturedConsoleLog, CapturedFailedRequest } from '../devices/playwright-device.js';

/**
 * SPA routing causes the browser to log "Failed to load resource: 404" for
 * client-side routes that the server doesn't recognize. These are not real
 * errors — the Angular router handles them before the 404 response matters.
 */
export function isSpaRoutingNoise(log: CapturedConsoleLog): boolean {
  return (
    log.text.includes('Failed to load resource: the server responded with a status of 404') ||
    log.text.includes('Failed to load resource: the server responded with a status of 0') ||
    // Browser logs 403 when admin endpoints deny access — the Angular app
    // handles this gracefully via catchError, but the browser still reports it.
    log.text.includes('Failed to load resource: the server responded with a status of 403') ||
    // doorway-app's DoorwayAdmin service auto-fires /auth/account on every page
    // (including unauthenticated routes like /login). When rendered against
    // doorway-app dev-server directly (E2E_DOORWAY_ALPHA=localhost:8081),
    // there's no doorway-service to handle the proxy and the dev-server
    // returns index.html, which fails JSON parsing. The Angular app handles
    // the result via catchError and the failure is cosmetic.
    log.text.includes('[DoorwayAdmin] getAccount failed')
  );
}

/**
 * Network requests that fail with ERR_ABORTED are typically caused by
 * SPA navigation canceling in-flight fetches, or by external resources
 * (YouTube embeds, CDN badges) that are unavailable in test environments.
 */
export function isExpectedNetworkFailure(req: CapturedFailedRequest): boolean {
  if (req.failure === 'net::ERR_ABORTED') return true;
  const externalHosts = ['youtube.com', 'ytimg.com', 'shields.io', 'googleapis.com'];
  return externalHosts.some(host => req.url.includes(host));
}
