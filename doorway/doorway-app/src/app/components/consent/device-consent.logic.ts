/**
 * Device approval — the portal-side rules, as pure functions.
 *
 * Everything here shapes what the person SEES. Whether a request is fit to be
 * shown, what may be agreed, and the code itself are decided by the runtime
 * behind /auth/consent/* (consent_grant in the peer runtime); the portal never
 * judges a request's content.
 */

import { HttpErrorResponse } from '@angular/common/http';

import type { WitnessStep } from '../../models/witness-step';
import type {
  ConsentAgreeResponse,
  ConsentViewResponse,
  GrantRequestJson,
} from '../../services/device-consent.service';

/** In-app path of the device approval page (the route is `consent/device`). */
export const DEVICE_CONSENT_PATH = '/consent/device';

/** Largest request a link may carry, in decoded bytes. */
export const MAX_REQUEST_BYTES = 4096;

/** Portal-side refusal codes (the runtime's own codes pass through untouched). */
export const REFUSAL = {
  /** The link's `request` is missing, not base64url, not JSON, or too large. */
  requestUnreadable: 'request_unreadable',
  /** This doorway cannot take device approvals (no endpoint, or unreachable). */
  consentUnavailable: 'consent_unavailable',
  /** The doorway asked the browser to hand the code somewhere other than this machine's terminal. */
  returnPathRefused: 'return_path_refused',
  /** The person left while an approval was being signed; it is not sent twice. */
  approvalInterrupted: 'approval_interrupted',
} as const;

// ---------------------------------------------------------------------------
// 1. Reading the link
// ---------------------------------------------------------------------------

export type DecodedRequest =
  | { ok: true; request: GrantRequestJson }
  | { ok: false; code: typeof REFUSAL.requestUnreadable };

const BASE64URL = /^[A-Za-z0-9_-]+={0,2}$/;
// A base64url string longer than this cannot decode to MAX_REQUEST_BYTES or fewer.
const MAX_ENCODED_LENGTH = Math.ceil(MAX_REQUEST_BYTES / 3) * 4;

/**
 * Decode the `?request=` parameter a terminal printed: base64url of the JSON
 * GrantRequest. Transport decoding only — the object is passed to the doorway
 * unchanged and never interpreted here.
 */
export function decodeConsentRequest(param: string | null | undefined): DecodedRequest {
  const unreadable = { ok: false, code: REFUSAL.requestUnreadable } as const;
  if (!param || param.length > MAX_ENCODED_LENGTH + 2 || !BASE64URL.test(param)) {
    return unreadable;
  }
  const unpadded = param.replace(/=+$/, '');
  if (unpadded.length % 4 === 1) return unreadable;

  let bytes: Uint8Array;
  try {
    const binary = atob(unpadded.replace(/-/g, '+').replace(/_/g, '/'));
    bytes = Uint8Array.from(binary, ch => ch.charCodeAt(0));
  } catch {
    return unreadable;
  }
  if (bytes.length === 0 || bytes.length > MAX_REQUEST_BYTES) return unreadable;

  let parsed: unknown;
  try {
    parsed = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
  } catch {
    return unreadable;
  }
  if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
    return unreadable;
  }
  return { ok: true, request: parsed as GrantRequestJson };
}

// ---------------------------------------------------------------------------
// 2. Handing the code to a terminal on this machine
// ---------------------------------------------------------------------------

const TERMINAL_RETURN = /^http:\/\/(?:127\.0\.0\.1|localhost):(\d{1,5})\/[^\s\\]*$/;

/**
 * The browser follows a redirect only to the asking terminal's own listener
 * on this machine: exactly `http://127.0.0.1:<port>/…` or
 * `http://localhost:<port>/…`. No other host, scheme, spelling of loopback,
 * credentials, or missing port.
 */
export function isTerminalReturnUrl(url: unknown): url is string {
  if (typeof url !== 'string') return false;
  const match = TERMINAL_RETURN.exec(url);
  if (!match) return false;
  const port = Number(match[1]);
  return port >= 1 && port <= 65535;
}

// ---------------------------------------------------------------------------
// 3. What the doorway's answers mean for the page
// ---------------------------------------------------------------------------

/** What to do after a failed call to /auth/consent/*. */
export type ConsentFailure = { kind: 'refused'; code: string } | { kind: 'sign-in' };

/**
 * 404/501, a network failure, or a server error → this doorway cannot approve
 * devices (yet). 401 → the session ran out; sign in again. Any other 4xx with a
 * `{ code }` body → the runtime's own refusal, shown as given.
 */
export function failureFor(error: unknown): ConsentFailure {
  const unavailable = { kind: 'refused', code: REFUSAL.consentUnavailable } as const;
  if (!(error instanceof HttpErrorResponse)) return unavailable;
  const { status } = error;
  if (status === 401) return { kind: 'sign-in' };
  if (status === 0 || status === 404 || status === 501 || status >= 500) return unavailable;
  if (status >= 400) {
    const code: unknown = (error.error as { code?: unknown } | null)?.code;
    return typeof code === 'string' && code.length > 0
      ? { kind: 'refused', code }
      : { kind: 'refused', code: REFUSAL.requestUnreadable };
  }
  return unavailable;
}

/** The review phase needs a view with something to ask. */
export function isShowableView(view: unknown): view is ConsentViewResponse {
  if (view === null || typeof view !== 'object') return false;
  const v = view as Partial<ConsentViewResponse>;
  return (
    typeof v.label === 'string' &&
    typeof v.deviceFingerprint === 'string' &&
    Array.isArray(v.askedActs) &&
    v.askedActs.length > 0
  );
}

// ---------------------------------------------------------------------------
// 4. Who secured the approval (the witness trail)
// ---------------------------------------------------------------------------

/** The step the portal itself observes: the person's doorway signing as them. */
export const DOORWAY_SIGN_STEP = 'doorway-sign';

/**
 * The trail for an approval: the doorway's signature in `state`, followed by
 * whatever witnesses the runtime reported, passed through unchanged.
 */
export function signingTrail(
  host: string,
  state: WitnessStep['state'],
  witnesses?: unknown
): WitnessStep[] {
  const doorway: WitnessStep = {
    id: DOORWAY_SIGN_STEP,
    act: 'signed',
    relation: 'your-doorway',
    label: host,
    state,
  };
  return Array.isArray(witnesses) ? [doorway, ...(witnesses as WitnessStep[])] : [doorway];
}

/** Where an agreement leaves the page. */
export type AgreementOutcome =
  | { phase: 'code'; code: string; expiresAt: number }
  | { phase: 'handed-back'; url: string }
  | { phase: 'refused'; code: string };

export function outcomeForAgreement(response: ConsentAgreeResponse | null): AgreementOutcome {
  const target = response?.returnTarget;
  if (target?.kind === 'display') {
    return typeof target.value === 'string' &&
      target.value.length > 0 &&
      typeof response?.expiresAt === 'number'
      ? { phase: 'code', code: target.value, expiresAt: response.expiresAt }
      : { phase: 'refused', code: REFUSAL.consentUnavailable };
  }
  if (target?.kind === 'redirect') {
    return isTerminalReturnUrl(target.url)
      ? { phase: 'handed-back', url: target.url }
      : { phase: 'refused', code: REFUSAL.returnPathRefused };
  }
  return { phase: 'refused', code: REFUSAL.consentUnavailable };
}
