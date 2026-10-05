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

import { sameOriginJson, type SameOriginOptions, type SameOriginResult } from '../same-origin.js';

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

/** What a call came back with (see {@link SameOriginResult}). */
export type ConsentWireResult<T> = SameOriginResult<T>;

/** The two calls the approval page makes. */
export interface DeviceConsentClient {
  /** What the device is asking for. */
  view(request: GrantRequestJson): Promise<ConsentWireResult<ConsentViewResponse>>;
  /** Sign the agreement as the signed-in person and get the code's destination. */
  agree(body: ConsentAgreeRequest): Promise<ConsentWireResult<ConsentAgreeResponse>>;
}

export type DeviceConsentClientOptions = SameOriginOptions;

/** Same-origin client for `/auth/consent/*`. */
export function createDeviceConsentClient(
  options: DeviceConsentClientOptions = {}
): DeviceConsentClient {
  return {
    view: async request =>
      sameOriginJson<ConsentViewResponse>(options, 'POST', CONSENT_VIEW_PATH, request),
    agree: async body =>
      sameOriginJson<ConsentAgreeResponse>(options, 'POST', CONSENT_AGREE_PATH, body),
  };
}
