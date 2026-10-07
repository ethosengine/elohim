/**
 * Device approval — the portal-side rules, as pure functions.
 *
 * Everything here shapes what the person SEES. Whether a request is fit to be
 * shown, what may be agreed, how many of the person's nodes must sign, who
 * witnessed it, and the code itself are decided by the node behind
 * /auth/consent/*; the portal never judges a request's content.
 */

import { isSessionProofRefusal } from '../session-key/index.js';
import { approveCommand } from '../terminal.js';

import type { WitnessStep, WitnessStepState } from '../witness-step.js';
import type {
  ConsentAgreeResponse,
  ConsentControllers,
  ConsentViewResponse,
  ConsentWireResult,
  GrantRequestJson,
} from './wire.js';

/** Largest request a link may carry, in decoded bytes. */
export const MAX_REQUEST_BYTES = 4096;

/** Portal-side refusal codes (the node's own codes pass through untouched). */
export const REFUSAL = {
  /** The link's `request` is missing, not base64url, not JSON, or too large. */
  requestUnreadable: 'request_unreadable',
  /** This host cannot take device approvals (no endpoint, or unreachable). */
  consentUnavailable: 'consent_unavailable',
  /** The node asked the browser to hand the code somewhere other than this machine's terminal. */
  returnPathRefused: 'return_path_refused',
  /** The person left while an approval was being signed; it is not sent twice. */
  approvalInterrupted: 'approval_interrupted',
} as const;

/** The node's refusal codes the page treats differently from a plain refusal. */
export const NODE_CODE = {
  /** No one is signed in here: sign in, then come straight back. */
  notSignedIn: 'consent_not_signed_in',
  /** This node has not recorded the person's identity yet. Nothing was signed. */
  identityUnbootstrapped: 'consent_identity_unbootstrapped',
  /** The node cannot reach its signer right now — a wait. Nothing was signed. */
  signingUnavailable: 'consent_signing_unavailable',
  /**
   * This page is open on another machine; the node signs only for a caller
   * on its own. Nothing was signed, and approving happens on that machine.
   */
  callerNotLocal: 'consent_caller_not_local',
  /**
   * A witness attending the person asked them to sign in again before the
   * approval goes ahead. Not a fault: nothing was signed, no code issued.
   */
  reauthenticationAsked: 'consent_reauthentication_asked',
} as const;

/**
 * Codes after which nothing was signed, so coming back to the link may ask
 * again: they are not remembered as the answer for this link.
 */
const NOTHING_SIGNED = new Set<string>([
  NODE_CODE.identityUnbootstrapped,
  NODE_CODE.signingUnavailable,
  NODE_CODE.callerNotLocal,
  NODE_CODE.reauthenticationAsked,
]);

/**
 * Codes after which the person may sign in and come straight back: a
 * witness asked them to, this browser's sign-in can no longer be confirmed,
 * or the page is neither signed in nor on the node's own machine.
 */
export function signInMayHelp(code: string | undefined): boolean {
  return (
    code === NODE_CODE.reauthenticationAsked ||
    code === NODE_CODE.callerNotLocal ||
    isSessionProofRefusal(code)
  );
}

/** True when a refusal says the node signed nothing (safe to approve again later). */
export function nothingWasSigned(code: string): boolean {
  return NOTHING_SIGNED.has(code) || isSessionProofRefusal(code);
}

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
 * GrantRequest. Transport decoding only — the object is passed to the node
 * unchanged and never interpreted here.
 */
export function decodeConsentRequest(param: string | null | undefined): DecodedRequest {
  const unreadable = { ok: false, code: REFUSAL.requestUnreadable } as const;
  if (!param || param.length > MAX_ENCODED_LENGTH + 2 || !BASE64URL.test(param)) {
    return unreadable;
  }
  let unpadded = param;
  while (unpadded.endsWith('=')) unpadded = unpadded.slice(0, -1);
  if (unpadded.length % 4 === 1) return unreadable;

  let bytes: Uint8Array;
  try {
    const binary = atob(unpadded.replace(/-/g, '+').replace(/_/g, '/'));
    bytes = Uint8Array.from(binary, ch => ch.codePointAt(0) ?? 0);
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
// 3. What the node's answers mean for the page
// ---------------------------------------------------------------------------

/** What to do after a failed call to /auth/consent/*. */
export type ConsentFailure = { kind: 'refused'; code: string } | { kind: 'sign-in' };

// eslint-disable-next-line sonarjs/function-return-type -- a code or none: none when the body carried no code
const codeOf = (body: unknown): string | undefined => {
  const code: unknown = (body as { code?: unknown } | null)?.code;
  return typeof code === 'string' && code.length > 0 ? code : undefined;
};

/**
 * 401, or `consent_not_signed_in` → sign in and come back. No answer, 404/501
 * or a server error → this host cannot approve devices (yet), unless the
 * server error says the node is waiting on its signer. Any other 4xx with a
 * `{ code }` body → the node's own refusal, shown as given.
 */

export function failureFor(
  result: Extract<ConsentWireResult<unknown>, { ok: false }>
): ConsentFailure {
  const unavailable = { kind: 'refused', code: REFUSAL.consentUnavailable } as const;
  const { status } = result;
  const code = codeOf(result.body);
  // Asked to sign in again is its own step, said and offered; never an automatic redirect.
  if (code === NODE_CODE.reauthenticationAsked) return { kind: 'refused', code };
  // So is a proof this browser's sign-in can no longer back (stale is
  // retried once, fresh, by the POST helper before it reaches here).
  if (isSessionProofRefusal(code)) return { kind: 'refused', code: code! };
  if (status === 401 || code === NODE_CODE.notSignedIn) return { kind: 'sign-in' };
  if (status >= 500 && code === NODE_CODE.signingUnavailable) return { kind: 'refused', code };
  if (status === 0 || status === 404 || status === 501 || status >= 500) return unavailable;
  if (status >= 400) return { kind: 'refused', code: code ?? REFUSAL.requestUnreadable };
  return unavailable;
}

/**
 * A page nobody is signed in to asks the host first whether it takes device
 * approvals at all, so a person is never sent through sign-in to a page that
 * then cannot approve. The answer is read as {@link failureFor} reads it:
 * whatever it calls unavailable (no answer, 404/501, a server error) is said
 * now, before sign-in. Any other answer — 401, a 200, the node's own refusal —
 * means this host takes approvals: sign in first, then review here as usual.
 */
export function beforeSignIn(result: ConsentWireResult<unknown>): ConsentFailure {
  if (result.ok) return { kind: 'sign-in' };
  const failure = failureFor(result);
  return failure.kind === 'refused' && failure.code === REFUSAL.consentUnavailable
    ? failure
    : { kind: 'sign-in' };
}

/**
 * The way through a refusal this host cannot get past: the same approval,
 * run in the terminal of a device that is already the person's. Given only
 * for `consent_unavailable` (this host takes no approvals), and only when the
 * host gave the page's own link; none otherwise.
 */
// eslint-disable-next-line sonarjs/function-return-type -- a command or none: none when there is no way through to give
export function wayThroughFor(
  code: string | undefined,
  link: string | undefined
): string | undefined {
  return code === REFUSAL.consentUnavailable && link ? approveCommand(link) : undefined;
}

/** Longest reason the node gives for asking the person to sign in again. */
export const MAX_REASON_LENGTH = 280;

/**
 * The node's own plain reason a refusal carries (`reason`), shown as given;
 * none when it carried no text.
 */
// eslint-disable-next-line sonarjs/function-return-type -- a reason or none
export function reasonOf(body: unknown): string | undefined {
  const reason: unknown = (body as { reason?: unknown } | null)?.reason;
  if (typeof reason !== 'string') return undefined;
  const text = reason.trim();
  return text ? text.slice(0, MAX_REASON_LENGTH) : undefined;
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

/**
 * The node that holds this person's key, as the host names it in the trail:
 * `your-doorway` when a doorway hosts the person, `this-device` when it is
 * the person's own node on this machine.
 */
export interface KeyHolderStep {
  relation: 'your-doorway' | 'this-device';
  /** The host's name for it (e.g. a doorway's domain), when it has one. */
  label?: string;
}

/** The step id the page uses for the key holder's own signature. */
export const KEY_HOLDER_SIGN_STEP = 'key-holder-sign';

/**
 * The key holder signing, in `state`. Shown while the person waits on it,
 * and as the whole trail only when the node reported no witnesses.
 */
export function keyHolderStep(holder: KeyHolderStep, state: WitnessStepState): WitnessStep {
  return {
    id: KEY_HOLDER_SIGN_STEP,
    act: 'signed',
    relation: holder.relation,
    ...(holder.label ? { label: holder.label } : {}),
    state,
  };
}

const isStep = (value: unknown): value is WitnessStep => {
  const step = value as Partial<WitnessStep> | null;
  return (
    step !== null &&
    typeof step === 'object' &&
    typeof step.id === 'string' &&
    typeof step.act === 'string' &&
    typeof step.relation === 'string' &&
    typeof step.state === 'string'
  );
};

/**
 * Who signed, after the node agreed: its `witnesses`, unchanged and in its
 * order. A 200 with no witnesses still means the key holder signed, so the
 * trail falls back to that one step — never to a party nobody reported.
 */
export function trailAfterAgreement(holder: KeyHolderStep, witnesses: unknown): WitnessStep[] {
  const reported = Array.isArray(witnesses) ? witnesses.filter(isStep) : [];
  return reported.length > 0 ? reported : [keyHolderStep(holder, 'done')];
}

// ---------------------------------------------------------------------------
// 5. What the approval rests on
// ---------------------------------------------------------------------------

/**
 * What the node said about the person's own nodes agreeing. One node agreeing
 * is a whole approval by default; the person's other nodes affirm it later
 * and nothing waits on them. Only a person who set up a quorum needs more.
 * - `single`: one was needed and it signed — a complete approval.
 * - `short`: the person's own quorum asks for more than have signed; `more`
 *   must still agree.
 * - `enough`: the person's quorum asked for several and all of them signed.
 */
export type ApprovalStanding =
  | { kind: 'single' }
  | { kind: 'short'; more: number; required: number; signed: number }
  | { kind: 'enough'; required: number; signed: number };

const count = (value: unknown): value is number =>
  typeof value === 'number' && Number.isInteger(value) && value >= 0;

// eslint-disable-next-line sonarjs/function-return-type -- a standing or none: none when the node gave no count
export function standingFor(controllers: unknown): ApprovalStanding | null {
  const c = controllers as Partial<ConsentControllers> | null | undefined;
  if (!c || !count(c.required) || !count(c.signed) || c.required < 1) return null;
  const { required, signed } = c;
  if (signed < required) return { kind: 'short', more: required - signed, required, signed };
  if (required === 1) return { kind: 'single' };
  return { kind: 'enough', required, signed };
}

// ---------------------------------------------------------------------------
// 6. Where an agreement leaves the page
// ---------------------------------------------------------------------------

export type AgreementOutcome =
  | { phase: 'code'; code: string; expiresAt: number }
  /** Handed to a terminal on this machine at `url`, or delivered by the node itself (no url). */
  | { phase: 'handed-back'; url?: string }
  /** The node holds the code until the asking device collects it: nothing to copy here. */
  | { phase: 'held' }
  | { phase: 'refused'; code: string };

export function outcomeForAgreement(response: ConsentAgreeResponse | null): AgreementOutcome {
  if (response?.delivered === true) return { phase: 'handed-back' };
  const target = response?.returnTarget;
  if (target?.kind === 'held') return { phase: 'held' };
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
