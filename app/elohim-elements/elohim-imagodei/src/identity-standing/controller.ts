/**
 * What a person's identity rests on, and beginning it — the framework-free
 * piece a portal mounts beside the device approval page.
 *
 * Service gravity: UX-surface. The node behind /auth/identity/* decides
 * whether an identity exists, makes and keeps its key, counts the devices that speak for the person
 * and refuses a caller on another machine; this controller only sequences
 * what the person sees, and keeps one rule of its own: a begin already on
 * its way is never sent again.
 *
 * The host renders `state` and supplies the client; `onChange` fires after
 * every change.
 */

import { IDENTITY_CODE, identityFailureFor, isStandingView } from './logic.js';

import type {
  SignedOutStanding,
  IdentityBeginResponse,
  IdentityStandingClient,
  IdentityStandingView,
} from './wire.js';

/**
 * - `standing`: the identity exists; `standing` says what it rests on.
 * - `begin`: no identity on this node yet; the person may begin one here.
 * - `beginning`: a begin is on its way (a wait on this node making the key).
 * - `not-signed-in`: the node holds an identity but no one is signed in.
 * - `unavailable`: the host does not answer these routes; say nothing.
 * - `refused`: the node answered the read with one of its own refusals.
 */
export type IdentityPhase =
  | 'loading'
  | 'standing'
  | 'begin'
  | 'beginning'
  | 'not-signed-in'
  | 'unavailable'
  | 'refused';

export interface IdentityPageState {
  phase: IdentityPhase;
  standing: IdentityStandingView | null;
  /** The node's code when the read itself was refused. */
  refusalCode?: string;
  /** The node's code when a begin was refused; the form stays, with this said. */
  beginRefusal?: string;
  /** The name last asked for, kept so a refused begin can be tried again or run elsewhere. */
  displayName?: string;
  /** What the last begin made (identity, authority, session). */
  created?: IdentityBeginResponse['created'];
  /**
   * When no one is signed in: whether this node holds an identity, and
   * whether a browser may sign in to it (a sign-in secret is set). Absent
   * when the node did not say.
   */
  signedOut?: { hasIdentity?: boolean; signInSecretSet?: boolean };
}

export interface IdentityStandingControllerOptions {
  client: IdentityStandingClient;
  onChange: (state: IdentityPageState) => void;
}

/** Code for a begin that got no usable answer. */
export const BEGIN_UNAVAILABLE = 'consent_unavailable';

const NO_ANSWER = { ok: false, status: 0, body: null } as const;

export class IdentityStandingController {
  private current: IdentityPageState = { phase: 'loading', standing: null };

  constructor(private readonly options: IdentityStandingControllerOptions) {}

  get state(): IdentityPageState {
    return this.current;
  }

  /** Ask the node what the signed-in person's identity rests on. */
  async read(): Promise<void> {
    const result = await this.options.client.standing().catch(() => NO_ANSWER);
    if (result.ok) {
      this.set(
        isStandingView(result.body)
          ? { phase: 'standing', standing: result.body }
          : { phase: 'unavailable' }
      );
      return;
    }
    const failure = identityFailureFor(result);
    switch (failure.kind) {
      case 'unbootstrapped':
        this.set({ phase: 'begin' });
        return;
      case 'not-signed-in': {
        const body = (result.body ?? {}) as Partial<SignedOutStanding>;
        const signedOut = {
          ...(typeof body.hasIdentity === 'boolean' ? { hasIdentity: body.hasIdentity } : {}),
          ...(typeof body.signInSecretSet === 'boolean'
            ? { signInSecretSet: body.signInSecretSet }
            : {}),
        };
        // No identity here at all: nothing to sign in to, so the person may begin one.
        this.set(
          signedOut.hasIdentity === false
            ? { phase: 'begin', signedOut }
            : { phase: 'not-signed-in', signedOut }
        );
        return;
      }
      case 'unavailable':
        this.set({ phase: failure.kind });
        return;
      default:
        this.set({ phase: 'refused', refusalCode: failure.code });
    }
  }

  /** The node said, elsewhere, that it has no identity yet: offer to begin here. */
  offerBegin(): void {
    if (this.current.phase === 'standing' || this.current.phase === 'beginning') return;
    this.set({ phase: 'begin', beginRefusal: undefined });
  }

  /**
   * Begin the person's identity on this node with the name they want shown.
   * Resolves true once the node has answered with what the identity rests
   * on. A second call while one is on its way sends nothing.
   */
  async begin(displayName: string, secret?: string): Promise<boolean> {
    if (this.current.phase === 'beginning' || this.current.phase === 'standing') return false;
    const name = displayName.trim();
    if (!name) {
      // Nothing to send: say so on the form, as the node would.
      this.set({ phase: 'begin', beginRefusal: IDENTITY_CODE.nameMalformed, displayName });
      return false;
    }
    this.set({ phase: 'beginning', beginRefusal: undefined, displayName: name });

    const body = secret ? { displayName: name, secret } : { displayName: name };
    const result = await this.options.client.begin(body).catch(() => NO_ANSWER);
    if (result.ok && isStandingView(result.body?.standing)) {
      this.set({
        phase: 'standing',
        standing: result.body.standing,
        created: result.body.created,
      });
      return true;
    }
    const failure = result.ok ? ({ kind: 'unavailable' } as const) : identityFailureFor(result);
    this.set({
      phase: 'begin',
      beginRefusal: failure.kind === 'refused' ? failure.code : BEGIN_UNAVAILABLE,
    });
    return false;
  }

  private set(partial: Partial<IdentityPageState>): void {
    this.current = { ...this.current, ...partial };
    this.options.onChange(this.current);
  }
}
