/**
 * Identity standing — the portal-side rules, as pure functions. What an
 * identity rests on, and whether one may begin, are the node's answers; this
 * file only reads them.
 */

import type { SameOriginResult } from '../same-origin.js';
import type { IdentityStandingView } from './wire.js';

/** The node's refusal codes an identity screen tells apart. */
export const IDENTITY_CODE = {
  /** No one is signed in here. */
  notSignedIn: 'consent_not_signed_in',
  /** This node has not recorded the person's identity: one may begin here. */
  unbootstrapped: 'consent_identity_unbootstrapped',
  /** The node cannot reach its signer right now — a wait. */
  signingUnavailable: 'consent_signing_unavailable',
  /** The call did not come from this node's own portal. */
  originRefused: 'consent_origin_refused',
  /** This page is open on another machine; the node signs only for its own. */
  callerNotLocal: 'consent_caller_not_local',
  /** The name is not plain text of a fitting length. */
  nameMalformed: 'identity_name_malformed',
} as const;

/** What a failed identity call means for the screen. */
export type IdentityFailure =
  | { kind: 'not-signed-in' }
  | { kind: 'unbootstrapped' }
  /** The host does not answer these routes (a doorway, an older node, no answer). */
  | { kind: 'unavailable' }
  | { kind: 'refused'; code: string };

// eslint-disable-next-line sonarjs/function-return-type -- a code or none: none when the body carried no code
const codeOf = (body: unknown): string | undefined => {
  const code: unknown = (body as { code?: unknown } | null)?.code;
  return typeof code === 'string' && code.length > 0 ? code : undefined;
};

/**
 * A refusal only counts when it carries a code in the node's own words; a
 * host that does not serve these routes answers some other way, and the
 * screen then says nothing rather than guess.
 */

export function identityFailureFor(
  result: Extract<SameOriginResult<unknown>, { ok: false }>
): IdentityFailure {
  const code = codeOf(result.body);
  if (code === IDENTITY_CODE.notSignedIn) return { kind: 'not-signed-in' };
  if (code === IDENTITY_CODE.unbootstrapped) return { kind: 'unbootstrapped' };
  if (code && Object.values(IDENTITY_CODE).includes(code as never))
    return { kind: 'refused', code };
  return { kind: 'unavailable' };
}

const count = (value: unknown): value is number =>
  typeof value === 'number' && Number.isInteger(value) && value >= 0;

/** A standing view with the counts a line can be built from. */
export function isStandingView(view: unknown): view is IdentityStandingView {
  const v = view as Partial<IdentityStandingView> | null;
  return (
    v !== null &&
    typeof v === 'object' &&
    Array.isArray(v.controllers) &&
    count(v.controllerCount) &&
    count(v.required) &&
    typeof v.thisNodeIsController === 'boolean' &&
    typeof v.restsOnThisNodeAlone === 'boolean'
  );
}
