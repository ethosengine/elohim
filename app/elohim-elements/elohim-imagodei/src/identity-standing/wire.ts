/**
 * Identity standing wire — what a person's identity rests on, and beginning
 * it, at the node that served this page (`/auth/identity/*`, same origin).
 * Typed request and response, no other logic.
 *
 * Framework-free and Lit-free; it names no kind of host.
 */

import { sameOriginJson, type SameOriginOptions, type SameOriginResult } from '../same-origin.js';

export const IDENTITY_STANDING_PATH = '/auth/identity/standing';
export const IDENTITY_BEGIN_PATH = '/auth/identity/begin';

/** 200 from GET /auth/identity/standing (consent_grant::StandingView). */
export interface IdentityStandingView {
  identityRoot: string;
  authority: string;
  networkDna: string;
  /** Every device that speaks for the person (a controller), as their identity authority names them. */
  controllers: string[];
  controllerCount: number;
  /** How many of them the person's own policy asks to approve a new device. */
  required: number;
  /** Whether the node answering is one of the devices that speak for the person. */
  thisNodeIsController: boolean;
  /** Whether the node answering is the only device that speaks for the person. */
  restsOnThisNodeAlone: boolean;
}

/** Body of POST /auth/identity/begin. */
export interface IdentityBeginRequest {
  /** The name the person wants shown. Required. */
  displayName: string;
  identifier?: string;
}

/** 201 (something was created) or 200 (nothing was) from POST /auth/identity/begin. */
export interface IdentityBeginResponse {
  standing: IdentityStandingView;
  session: { id: string; humanId: string; identifier: string };
  created: { human: boolean; authority: boolean; session: boolean };
}

/** The two calls an identity screen makes. */
export interface IdentityStandingClient {
  standing(): Promise<SameOriginResult<IdentityStandingView>>;
  begin(body: IdentityBeginRequest): Promise<SameOriginResult<IdentityBeginResponse>>;
}

/** Same-origin client for `/auth/identity/*`. */
export function createIdentityStandingClient(
  options: SameOriginOptions = {}
): IdentityStandingClient {
  return {
    standing: async () =>
      sameOriginJson<IdentityStandingView>(options, 'GET', IDENTITY_STANDING_PATH),
    begin: async body =>
      sameOriginJson<IdentityBeginResponse>(options, 'POST', IDENTITY_BEGIN_PATH, body),
  };
}
