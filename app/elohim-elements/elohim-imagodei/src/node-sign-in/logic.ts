/**
 * Signing in — the portal-side reading of the node's answers. Whether the
 * word and secret match, whether a witness pauses, and how long a slowed
 * source waits are the node's decisions; this only names them.
 */

import { reasonOf } from '../device-consent/logic.js';

import type { SameOriginResult } from '../same-origin.js';

export const SIGN_IN_CODE = {
  invalid: 'INVALID_CREDENTIALS',
  secretUnset: 'signin_secret_unset',
  slowed: 'signin_slowed',
  paused: 'signin_paused',
  needsSessionKey: 'signin_needs_session_key',
  needsSecureChannel: 'signin_needs_secure_channel',
} as const;

/** What a refused sign-in means for the page. */
export type SignInFailure =
  /** The word and secret do not match (the node does not say which). */
  | { kind: 'invalid' }
  /** No sign-in secret is set on this node yet. */
  | { kind: 'secret-unset' }
  /** Too many tries: a wait of `retryAfter` seconds. */
  | { kind: 'slowed'; retryAfter?: number }
  /** A witness attending the person paused the sign-in. */
  | { kind: 'paused'; reason?: string }
  /** The node wants a session key this browser could not send. */
  | { kind: 'needs-session-key' }
  /** The node wants this sign-in over a secure channel. */
  | { kind: 'needs-secure-channel' }
  /** A word or a secret was left out. */
  | { kind: 'missing' }
  /** No usable answer. */
  | { kind: 'unavailable' };

const field = (body: unknown, name: string): unknown =>
  (body as Record<string, unknown> | null)?.[name];

export function signInFailureFor(
  result: Extract<SameOriginResult<unknown>, { ok: false }>
): SignInFailure {
  const code = field(result.body, 'code');
  switch (code) {
    case SIGN_IN_CODE.invalid:
      return { kind: 'invalid' };
    case SIGN_IN_CODE.secretUnset:
      return { kind: 'secret-unset' };
    case SIGN_IN_CODE.slowed: {
      const after = Number(field(result.body, 'retryAfter'));
      return Number.isFinite(after) && after > 0
        ? { kind: 'slowed', retryAfter: Math.ceil(after) }
        : { kind: 'slowed' };
    }
    case SIGN_IN_CODE.paused: {
      const reason = reasonOf(result.body);
      return reason ? { kind: 'paused', reason } : { kind: 'paused' };
    }
    case SIGN_IN_CODE.needsSessionKey:
      return { kind: 'needs-session-key' };
    case SIGN_IN_CODE.needsSecureChannel:
      return { kind: 'needs-secure-channel' };
    default:
      return result.status === 400 ? { kind: 'missing' } : { kind: 'unavailable' };
  }
}
