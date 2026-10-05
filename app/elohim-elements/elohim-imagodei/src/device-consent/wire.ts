/**
 * Device approval wire — the one place a portal talks to `/auth/consent/*`.
 *
 * The calls go to the same origin the portal was served from: the node that
 * holds this person's key. Which kind of host that is (a doorway hosting the
 * person, or the person's own node) is the host's business, never this
 * file's. Typed request and response, no other logic: when the backend
 * changes, this file is where it changes.
 *
 * Framework-free and Lit-free; both sign-in portals import it.
 */

import type { WitnessStep } from '../witness-step.js';

export const CONSENT_VIEW_PATH = '/auth/consent/view';
export const CONSENT_AGREE_PATH = '/auth/consent/agree';

/** The two acts a device can ask its person to agree to. */
export type DeviceAct = 'device.enroll' | 'device.bind-root';

/**
 * The terminal's GrantRequest, exactly as its link carried it. The portal
 * passes it back unchanged and never reads its fields.
 */
export type GrantRequestJson = Readonly<Record<string, unknown>>;

/** 200 from POST /auth/consent/view. */
export interface ConsentViewResponse {
  clientId: string;
  label: string;
  deviceFingerprint: string;
  deviceRootFingerprint?: string;
  askedActs: DeviceAct[];
}

/** Body of POST /auth/consent/agree. */
export interface ConsentAgreeRequest {
  request: GrantRequestJson;
  agreedActs: DeviceAct[];
}

/** Where the one-time code goes: shown to paste, or handed to a terminal on this machine. */
export type ConsentReturnTarget =
  | { kind: 'display'; value: string }
  | { kind: 'redirect'; url: string };

/** How many of the person's own nodes this approval needs, and how many have signed. */
export interface ConsentControllers {
  required: number;
  signed: number;
}

/** 200 from POST /auth/consent/agree. `expiresAt` is epoch milliseconds. */
export interface ConsentAgreeResponse {
  returnTarget: ConsentReturnTarget;
  expiresAt: number;
  consentCid: string;
  controllers: ConsentControllers;
  /** Only the parties that actually signed, in the order they signed. */
  witnesses: WitnessStep[];
}

/** 4xx body from either call. */
export interface ConsentRefusalBody {
  error: string;
  code: string;
}

/**
 * What a call came back with. `status` 0 means no answer at all (offline,
 * blocked, the node unreachable); `body` is whatever JSON the refusal
 * carried, or null.
 */
export type ConsentWireResult<T> =
  | { ok: true; body: T }
  | { ok: false; status: number; body: unknown };

/** The two calls the approval page makes. */
export interface DeviceConsentClient {
  /** What the device is asking for. */
  view(request: GrantRequestJson): Promise<ConsentWireResult<ConsentViewResponse>>;
  /** Sign the agreement as the signed-in person and get the code's destination. */
  agree(body: ConsentAgreeRequest): Promise<ConsentWireResult<ConsentAgreeResponse>>;
}

export interface DeviceConsentClientOptions {
  /** Defaults to the global `fetch`. */
  fetch?: typeof fetch;
  /**
   * Extra headers per call — how a host that keeps its session outside a
   * cookie (a bearer token) proves it. Cookies on this origin are always sent.
   */
  headers?: () => Record<string, string>;
}

/** Same-origin client for `/auth/consent/*`. */
export function createDeviceConsentClient(
  options: DeviceConsentClientOptions = {}
): DeviceConsentClient {
  const post = async <T>(path: string, body: unknown): Promise<ConsentWireResult<T>> => {
    const doFetch = options.fetch ?? globalThis.fetch.bind(globalThis);
    let response: Response;
    try {
      response = await doFetch(path, {
        method: 'POST',
        credentials: 'same-origin',
        headers: { 'Content-Type': 'application/json', ...options.headers?.() },
        body: JSON.stringify(body),
      });
    } catch {
      return { ok: false, status: 0, body: null };
    }
    const payload: unknown = await response.json().catch(() => null);
    return response.ok
      ? { ok: true, body: payload as T }
      : { ok: false, status: response.status, body: payload };
  };

  return {
    view: async request => post<ConsentViewResponse>(CONSENT_VIEW_PATH, request),
    agree: async body => post<ConsentAgreeResponse>(CONSENT_AGREE_PATH, body),
  };
}
