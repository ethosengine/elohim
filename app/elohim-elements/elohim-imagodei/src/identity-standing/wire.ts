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
  /** The name the person asked to be shown, when the node has it. */
  displayName?: string;
  /**
   * The word the person signs in with — an identity claim, shown wherever the
   * person is named and never used to decide anything. Absent when the node
   * has none to give; never the identity's record id.
   */
  identifier?: string;
}

/** Body of POST /auth/identity/begin. */
export interface IdentityBeginRequest {
  /** The name the person wants shown. Required. */
  displayName: string;
  identifier?: string;
  /** A sign-in secret to set with the identity, so a browser can sign in later. Optional. */
  secret?: string;
}

/** 201 (something was created) or 200 (nothing was) from POST /auth/identity/begin. */
export interface IdentityBeginResponse {
  standing: IdentityStandingView;
  session: { id: string; humanId: string; identifier: string };
  created: { human: boolean; authority: boolean; session: boolean };
  /** Whether a sign-in secret was set with this begin. */
  signInSecretSet?: boolean;
}

/**
 * The 401 body of a signed-out standing read: whether this node holds an
 * identity, and whether a browser may sign in to it (a secret is set).
 */
export interface SignedOutStanding {
  error: string;
  code: 'consent_not_signed_in';
  hasIdentity?: boolean;
  signInSecretSet?: boolean;
}

/** The two calls an identity screen makes. */
export interface IdentityStandingClient {
  standing(): Promise<SameOriginResult<IdentityStandingView>>;
  begin(body: IdentityBeginRequest): Promise<SameOriginResult<IdentityBeginResponse>>;
}

/**
 * Same-origin client for `/auth/identity/*`. Only `begin` makes the node
 * sign, so only it carries the session proof (`options.prove`).
 */
export function createIdentityStandingClient(
  options: SameOriginOptions = {}
): IdentityStandingClient {
  return {
    standing: async () =>
      sameOriginJson<IdentityStandingView>(
        { ...options, prove: undefined },
        'GET',
        IDENTITY_STANDING_PATH
      ),
    begin: async body =>
      sameOriginJson<IdentityBeginResponse>(options, 'POST', IDENTITY_BEGIN_PATH, body),
  };
}
