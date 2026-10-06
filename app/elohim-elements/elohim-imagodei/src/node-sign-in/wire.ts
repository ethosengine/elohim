/**
 * Signing in to the node that served this page, as its own person — the
 * wire to `/auth/login` and `/auth/logout`, same origin. Where the page can
 * keep a key, a sign-in binds the session to this browser's session key:
 * the public half goes in the body, and the request carries a proof made
 * with it. Where it cannot, the sign-in goes without one; nothing is said
 * to the person either way.
 *
 * Framework-free and Lit-free; it names no kind of host.
 */

import { sameOriginJson, type SameOriginResult } from '../same-origin.js';
import { sessionKeys, type SessionKeys } from '../session-key/session.js';

export const LOGIN_PATH = '/auth/login';
export const LOGOUT_PATH = '/auth/logout';

/** What the person types, and where to come back to. */
export interface SignInRequest {
  /** The sign-in word. */
  identifier: string;
  /** The sign-in secret. */
  password: string;
  remember?: boolean;
  returnTo?: string;
}

/** 200 from POST /auth/login. `expiresAt` is unix seconds. */
export interface SignInResponse {
  humanId: string;
  agentPubKey: string;
  identifier: string;
  displayName?: string;
  expiresAt: number;
  isSteward: true;
  redirect: string;
  /** Whether the node bound this session to the key this browser sent. */
  sessionKeyBound: boolean;
}

export interface NodeSignInClient {
  signIn(request: SignInRequest): Promise<SameOriginResult<SignInResponse>>;
  signOut(): Promise<SameOriginResult<unknown>>;
}

export interface NodeSignInOptions {
  fetch?: typeof fetch;
  /** This browser's session keys; defaults to the page's own. */
  keys?: SessionKeys;
}

export function createNodeSignInClient(options: NodeSignInOptions = {}): NodeSignInClient {
  const keys = options.keys ?? sessionKeys();
  return {
    async signIn(request) {
      const signer = await keys.forSignIn().catch(() => null);
      const body = signer
        ? { ...request, sessionKey: { alg: signer.alg, jwk: await signer.publicJwk() } }
        : request;
      const result = await sameOriginJson<SignInResponse>(
        // eslint-disable-next-line @typescript-eslint/promise-function-async -- the key made for this sign-in
        { fetch: options.fetch, prove: () => Promise.resolve(signer) },
        'POST',
        LOGIN_PATH,
        body
      );
      if (result.ok) await keys.signedIn(signer, result.body?.sessionKeyBound === true);
      return result;
    },
    async signOut() {
      const result = await sameOriginJson<unknown>(
        { fetch: options.fetch, prove: async () => keys.current() },
        'POST',
        LOGOUT_PATH,
        {}
      );
      await keys.forget();
      return result;
    },
  };
}
