/**
 * Signing in to this node from a browser, and signing out — the
 * framework-free piece a portal mounts. It sends one sign-in at a time and
 * never retries one by itself; the node decides everything else.
 */

import { signInFailureFor, type SignInFailure } from './logic.js';

import type { NodeSignInClient, SignInRequest, SignInResponse } from './wire.js';

export type SignInPhase = 'form' | 'signing-in' | 'signed-in' | 'signing-out' | 'signed-out';

export interface SignInPageState {
  phase: SignInPhase;
  /** Why the last sign-in did not go ahead. */
  refusal?: SignInFailure;
  /** The word last typed, kept so the form need not ask again. */
  identifier?: string;
  /** Who signed in, as the node answered. */
  signedIn?: SignInResponse;
}

export interface SignInControllerOptions {
  client: NodeSignInClient;
  onChange: (state: SignInPageState) => void;
}

const NO_ANSWER = { ok: false, status: 0, body: null } as const;

export class SignInController {
  private current: SignInPageState = { phase: 'form' };

  constructor(private readonly options: SignInControllerOptions) {}

  get state(): SignInPageState {
    return this.current;
  }

  /** Sign in with the word and secret. Resolves true once the node has signed the person in. */
  async signIn(request: SignInRequest): Promise<boolean> {
    if (this.current.phase === 'signing-in') return false;
    const identifier = request.identifier.trim();
    if (!identifier || !request.password) {
      this.set({ phase: 'form', refusal: { kind: 'missing' }, identifier });
      return false;
    }
    this.set({ phase: 'signing-in', refusal: undefined, identifier });
    const result = await this.options.client
      .signIn({ ...request, identifier })
      .catch(() => NO_ANSWER);
    if (result.ok) {
      this.set({ phase: 'signed-in', signedIn: result.body });
      return true;
    }
    this.set({ phase: 'form', refusal: signInFailureFor(result) });
    return false;
  }

  /** Sign out of this node, and forget this browser's key. */
  async signOut(): Promise<void> {
    if (this.current.phase === 'signing-out') return;
    this.set({ phase: 'signing-out' });
    await this.options.client.signOut().catch(() => NO_ANSWER);
    this.set({ phase: 'signed-out', signedIn: undefined });
  }

  private set(partial: Partial<SignInPageState>): void {
    this.current = { ...this.current, ...partial };
    this.options.onChange(this.current);
  }
}
