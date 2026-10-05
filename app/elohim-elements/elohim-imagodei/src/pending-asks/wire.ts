/**
 * Devices asking this node over a private network — the wire to
 * `GET /auth/consent/pending` and `POST /auth/consent/pending/decide`, same
 * origin. Deciding makes the node sign, so it carries the session proof.
 *
 * Framework-free and Lit-free.
 */

import { sameOriginJson, type SameOriginOptions, type SameOriginResult } from '../same-origin.js';

import type { ConsentAgreeResponse, DeviceAct } from '../device-consent/wire.js';

export const PENDING_PATH = '/auth/consent/pending';
export const PENDING_DECIDE_PATH = '/auth/consent/pending/decide';

/** The person this node speaks for, as the node names them. */
export interface SpeaksForPerson {
  kind: 'person';
  /** The sign-in word: a claim, shown, never used to decide. */
  identifier?: string;
  /** The Human record's id. Never shown, and nothing here reads it. */
  humanId?: string;
  displayName?: string;
  identityRoot: string;
  identityFingerprint: string;
}

export type SpeaksFor = SpeaksForPerson | { kind: 'nobody' } | { kind: 'unknown' };

/** What the asking node says about itself. */
export interface AskingNodeState {
  kind: 'unassigned' | 'own-identity' | 'joined';
  made?: string;
}

/** One device asking. */
export interface PendingAskView {
  /** A small number that picks the ask. */
  number: number;
  label: string;
  deviceKey: string;
  deviceFingerprint: string;
  deviceRootFingerprint?: string;
  askedActs: DeviceAct[];
  secondsLeft: number;
  state: AskingNodeState;
  /** The node's sentence for `state`. */
  stateWords: string;
  /** Whether the device named this node as its approver. */
  addressedHere: boolean;
  /** Whose identity an approval here would be for. */
  forIdentity?: Omit<SpeaksForPerson, 'kind'>;
}

/** 200 from GET /auth/consent/pending. */
export interface PendingAsksView {
  carrier: 'private-network' | 'absent';
  /** This node's own key on the carrier. */
  approver?: string | null;
  speaksFor: SpeaksFor;
  speaksForWords: string;
  asks: PendingAskView[];
}

/** Body of POST /auth/consent/pending/decide. */
export interface PendingDecideRequest {
  ask: string;
  answer?: { agreedActs: DeviceAct[] };
}

/** 200 from POST /auth/consent/pending/decide. */
export type PendingDecideResponse =
  | {
      number: number;
      decidedBy: string;
      agreed: ConsentAgreeResponse;
      handedBack: { taken: boolean; answer?: unknown; error?: string };
    }
  | {
      number: number;
      decidedBy: string;
      declined: true;
      /** Whether the asking device took the answer, so its terminal stops waiting. */
      handedBack?: { taken: boolean };
    };

export interface PendingAsksClient {
  list(): Promise<SameOriginResult<PendingAsksView>>;
  decide(body: PendingDecideRequest): Promise<SameOriginResult<PendingDecideResponse>>;
}

/** Same-origin client; only `decide` carries the session proof. */
export function createPendingAsksClient(options: SameOriginOptions = {}): PendingAsksClient {
  return {
    list: async () =>
      sameOriginJson<PendingAsksView>({ ...options, prove: undefined }, 'GET', PENDING_PATH),
    decide: async body =>
      sameOriginJson<PendingDecideResponse>(options, 'POST', PENDING_DECIDE_PATH, body),
  };
}
