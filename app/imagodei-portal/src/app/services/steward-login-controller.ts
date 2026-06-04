/**
 * steward-login-controller — orchestrates the doorway→steward portal handoff
 * on the consuming (portal-host) side.
 *
 * When a graduated steward logs in at a doorway, the doorway (acting as OAuth
 * relying party, NOT identity authority) redirects the browser to this portal
 * host carrying an opaque single-use `session_token` and the issuer origin
 * `doorway_url`. This controller redeems that token at the steward's own
 * storage backend, then re-runs the normal `/auth/me` discovery so the portal
 * shell proceeds exactly as a locally-authenticated peer-native session.
 *
 * Service gravity: UX-surface. The doorway owns token issuance; the steward's
 * storage owns redemption + trust adjudication (it decides whether the issuer
 * is trusted — a 403). This controller carries values across the wire and maps
 * outcomes to display states. It computes no auth/trust truth of its own, and
 * it NEVER configures `trustMode` — that is discovered downstream from
 * `/auth/me` via `StandaloneResolver.fetchAuthority`.
 *
 * Kept as a pure function over an effects seam so it is unit-testable without a
 * full Angular TestBed and without touching the DOM directly; the component
 * supplies the effects (URL rewrite, authority refresh, consent re-check).
 */

import type { StandaloneResolver } from './standalone-resolver.js';

/** The two query params doorway hands across; both required for a handoff. */
export interface StewardHandoff {
  sessionToken: string;
  doorwayUrl: string;
}

export type StewardLoginStatus = 'authenticated' | 'failed' | 'unreachable';

export interface StewardLoginOutcome {
  status: StewardLoginStatus;
  /** Non-scary message to display on failure/unreachable. Empty on success. */
  message: string;
  /** Where the "return to your doorway" action should send the person. */
  returnToDoorwayUrl?: string;
}

/**
 * Effects the controller invokes; the component wires these to real DOM /
 * service calls. Kept narrow so the controller stays pure and testable.
 */
export interface StewardLoginEffects {
  /**
   * Remove `session_token` + `doorway_url` from the address bar (via
   * history.replaceState) so the opaque code never lingers in the URL.
   * Receives the current `location.search`.
   */
  stripHandoffParams(search: string): void;
  /** Re-run `GET /auth/me` discovery and bind the resulting authority. */
  refreshAuthority(): Promise<void>;
  /**
   * Re-evaluate whether the (now session-bearing) URL carries an OAuth consent
   * request and, if so, enter the consent step. Receives the current search.
   */
  enterConsentIfRequested(search: string): Promise<void>;
}

/**
 * Inspect a query string for the doorway→steward handoff. Returns the handoff
 * only when BOTH params are present; otherwise null (the normal login/consent
 * flow is untouched).
 */
export function readStewardHandoff(search: string): StewardHandoff | null {
  const params = new URLSearchParams(search);
  const sessionToken = params.get('session_token');
  const doorwayUrl = params.get('doorway_url');
  if (!sessionToken || !doorwayUrl) return null;
  return { sessionToken, doorwayUrl };
}

const UNREACHABLE_MESSAGE =
  "Your conductor isn't reachable right now. Your account is still safe — " +
  'this just means the device that holds your identity is offline. ' +
  'You can try again in a moment.';

const FAILED_MESSAGE =
  "We couldn't complete the hand-off from your doorway. Nothing about your " +
  'account has changed and it is still safe. You can head back to your ' +
  'doorway and sign in again.';

/**
 * Redeem the handoff token and, on success, hand control back to the normal
 * discovery path. Maps the redemption outcome to a display state:
 *
 *  - 201            → re-run /auth/me discovery, strip the code from the URL,
 *                     preserve any OAuth params for the consent flow.
 *  - 401 / 403      → 'failed' with a non-scary message + return-to-doorway.
 *  - network error  → 'unreachable' (conductor offline) + return-to-doorway.
 */
export async function runStewardLogin(
  resolver: StandaloneResolver,
  handoff: StewardHandoff,
  search: string,
  effects: StewardLoginEffects
): Promise<StewardLoginOutcome> {
  const result = await resolver.loginWithSessionToken(handoff.sessionToken, handoff.doorwayUrl);

  if (result.ok) {
    // Strip the opaque code from the address bar BEFORE anything else navigates,
    // so it never lingers in history. OAuth params are preserved by the strip.
    effects.stripHandoffParams(search);
    // Re-run the normal discovery path — trustMode is read from /auth/me here,
    // never configured. The shell then proceeds as a local session.
    await effects.refreshAuthority();
    // If the doorway carried an OAuth consent request through the hop, pick it
    // up now that the session has landed so the code dance can complete.
    await effects.enterConsentIfRequested(stripHandoffFromSearch(search));
    return { status: 'authenticated', message: '' };
  }

  // No status ⇒ the storage backend was unreachable (conductor offline).
  if (result.status === undefined) {
    return {
      status: 'unreachable',
      message: UNREACHABLE_MESSAGE,
      returnToDoorwayUrl: handoff.doorwayUrl,
    };
  }

  // 401 (redeem failed / expired / replayed) or 403 (issuer not trusted).
  return {
    status: 'failed',
    message: FAILED_MESSAGE,
    returnToDoorwayUrl: handoff.doorwayUrl,
  };
}

/**
 * Produce a query string with the two handoff params removed but every other
 * param (notably the OAuth client_id/redirect_uri/response_type/state/scope)
 * preserved — the shape the consent flow expects after the session lands.
 */
export function stripHandoffFromSearch(search: string): string {
  const params = new URLSearchParams(search);
  params.delete('session_token');
  params.delete('doorway_url');
  const rest = params.toString();
  return rest ? `?${rest}` : '';
}
