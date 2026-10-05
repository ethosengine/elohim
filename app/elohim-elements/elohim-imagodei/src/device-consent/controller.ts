/**
 * The device approval page, without a framework — the one piece both sign-in
 * portals mount.
 *
 * A terminal on a device printed a link carrying its request. The page reads
 * the request, asks the node that holds this person's key what the device is
 * asking for, lets the person approve all or some of it (or decline), and
 * then shows the one-time code or hands it to a terminal on this machine.
 *
 * Service gravity: UX-surface. The node behind /auth/consent/* checks the
 * request, signs, counts the person's nodes and issues the code; this
 * controller only sequences what the person sees and keeps one rule of its
 * own — an approval is never sent twice from one link.
 *
 * The host supplies everything host-specific through the options: how to
 * reach the node (the client), what the key holder is called in the trail,
 * how to send the person to sign in and back, and how to leave for a
 * terminal. The host renders `state` with the consent card and the witness
 * trail; `onChange` fires after every change.
 */

import {
  NODE_CODE,
  REFUSAL,
  decodeConsentRequest,
  failureFor,
  isShowableView,
  keyHolderStep,
  nothingWasSigned,
  outcomeForAgreement,
  reasonOf,
  signInMayHelp,
  standingFor,
  trailAfterAgreement,
  type ApprovalStanding,
  type KeyHolderStep,
} from './logic.js';
import { tabConsentMemory, type ConsentMemory, type RememberedOutcome } from './progress.js';

import type { WitnessStep } from '../witness-step.js';
import type {
  ConsentViewResponse,
  DeviceAct,
  DeviceConsentClient,
  GrantRequestJson,
} from './wire.js';

/** The card's phases, and `loading` before the card can show anything. */
export type DeviceConsentPagePhase =
  | 'loading'
  | 'review'
  | 'signing'
  | 'code'
  | 'handed-back'
  | 'declined'
  | 'refused';

export interface DeviceConsentPageState {
  phase: DeviceConsentPagePhase;
  /** What the device asked for, once the node has shown it. */
  view: ConsentViewResponse | null;
  /** One-time code to paste (phase `code`). Gone once it has expired. */
  code?: string;
  /** Code expiry, epoch milliseconds (phase `code`). */
  expiresAt?: number;
  /** Machine code for phase `refused`. */
  refusalCode?: string;
  /** The node's own plain reason, when its refusal gave one (asked to sign in again). */
  refusalReason?: string;
  /** Phase `handed-back`: the node delivered the code over a private network itself. */
  handedBackOverNetwork?: boolean;
  /** Who secured the approval: live while signing, settled once it is done. */
  trail: WitnessStep[] | null;
  /** What the approval rests on, as the node counted it. */
  standing: ApprovalStanding | null;
}

/** `approve` event detail from the card. */
export interface DeviceConsentApproval {
  agreedActs: DeviceAct[];
  declinedActs?: DeviceAct[];
}

export interface DeviceConsentControllerOptions {
  /** The raw `request` parameter of the link — also the key this tab remembers it by. */
  requestParam: string | null | undefined;
  /** The node that holds this person's key, as the trail names it. */
  holder: KeyHolderStep;
  client: DeviceConsentClient;
  /** Send the person to sign in and bring them straight back to this link. */
  signIn: () => void;
  /** Hand the code to the asking terminal's listener on this machine (top-level navigation). */
  handBack: (url: string) => void;
  onChange: (state: DeviceConsentPageState) => void;
  /** Where this tab remembers the answer. Defaults to sessionStorage. */
  memory?: ConsentMemory;
}

/** A client that threw is treated as one that got no answer. */
const NO_ANSWER = { ok: false, status: 0, body: null } as const;

const CODE = 'code';
const HANDED_BACK = 'handed-back';

const INITIAL: DeviceConsentPageState = {
  phase: 'loading',
  view: null,
  trail: null,
  standing: null,
};

export class DeviceConsentController {
  private current: DeviceConsentPageState = INITIAL;
  private request: GrantRequestJson | null = null;
  private readonly requestParam: string;
  private readonly memory: ConsentMemory;

  constructor(private readonly options: DeviceConsentControllerOptions) {
    this.requestParam = options.requestParam ?? '';
    this.memory = options.memory ?? tabConsentMemory();
  }

  get state(): DeviceConsentPageState {
    return this.current;
  }

  /**
   * Read the link and show what this tab already did with it, or ask the node
   * what the device wants. Nothing is sent anywhere for an unreadable link.
   */
  async start(): Promise<void> {
    const decoded = decodeConsentRequest(this.requestParam);
    if (!decoded.ok) {
      this.refuse(decoded.code, false);
      return;
    }
    this.request = decoded.request;

    const remembered = this.memory.recall(this.requestParam);
    if (remembered) {
      this.restore(remembered.view, remembered.outcome, {
        trail: remembered.trail ?? null,
        standing: remembered.standing ?? null,
      });
      return;
    }
    await this.loadView();
  }

  async approve(approval: DeviceConsentApproval): Promise<void> {
    const request = this.request;
    if (this.current.phase !== 'review' || !request || approval.agreedActs.length === 0) return;
    const { holder } = this.options;
    this.set({ phase: 'signing', trail: [keyHolderStep(holder, 'working')] });
    this.remember({ phase: 'signing' });

    const result = await this.options.client
      .agree({ request, agreedActs: approval.agreedActs })
      .catch(() => NO_ANSWER);

    if (!result.ok) {
      const failure = failureFor(result);
      if (failure.kind === 'sign-in') {
        // Nothing was signed; sign in and approve again.
        this.memory.forget(this.requestParam);
        this.sendToSignIn();
        return;
      }
      if (nothingWasSigned(failure.code)) {
        // A wait, a missing set-up, another machine, or a request to sign in
        // again: nothing to witness, and coming back to this link may ask again.
        this.memory.forget(this.requestParam);
        this.set({ trail: null });
        this.refuse(failure.code, false, reasonOf(result.body));
        return;
      }
      this.set({ trail: [keyHolderStep(holder, 'failed')] });
      this.refuse(failure.code);
      return;
    }

    const response = result.body;
    this.set({
      trail: trailAfterAgreement(holder, response?.witnesses),
      standing: standingFor(response?.controllers),
    });
    const outcome = outcomeForAgreement(response ?? null);
    if (outcome.phase === CODE) {
      this.set({ phase: CODE, code: outcome.code, expiresAt: outcome.expiresAt });
      this.remember({ phase: CODE, code: outcome.code, expiresAt: outcome.expiresAt });
    } else if (outcome.phase === HANDED_BACK) {
      this.set({ phase: HANDED_BACK, handedBackOverNetwork: !outcome.url });
      this.remember({ phase: HANDED_BACK });
      if (outcome.url) this.options.handBack(outcome.url);
    } else {
      this.refuse(outcome.code);
    }
  }

  /**
   * Come back to the approval after a refusal that signed nothing — the
   * person has since begun their identity, or the signer is back. The page
   * returns to review; nothing is sent until the person approves again, so
   * nothing is ever sent twice.
   */
  async resume(): Promise<void> {
    const { phase, refusalCode, view } = this.current;
    if (phase !== 'refused' || !refusalCode || !nothingWasSigned(refusalCode)) return;
    this.memory.forget(this.requestParam);
    if (view) {
      this.set({
        phase: 'review',
        refusalCode: undefined,
        refusalReason: undefined,
        trail: null,
        standing: null,
      });
      return;
    }
    this.set({ phase: 'loading', refusalCode: undefined, trail: null });
    await this.loadView();
  }

  /**
   * The person chose to sign in (again) — a witness asked them to, this
   * browser's sign-in could no longer be confirmed, or the page was not
   * signed in. Sends them to sign in and straight back to this link, where
   * the approval is asked again from review; nothing is resent by itself.
   */
  signInAgain(): void {
    const { phase, refusalCode } = this.current;
    if (phase !== 'refused' || !signInMayHelp(refusalCode)) return;
    this.memory.forget(this.requestParam);
    this.options.signIn();
  }

  /** Declining signs nothing, so nothing is sent. */
  decline(): void {
    if (this.current.phase !== 'review') return;
    this.set({ phase: 'declined' });
    this.remember({ phase: 'declined' });
  }

  /** The code ran out: the card says so; forget the code itself. */
  expired(): void {
    const { phase, expiresAt } = this.current;
    if (phase === CODE && expiresAt !== undefined) {
      this.remember({ phase: CODE, expiresAt });
    }
  }

  private async loadView(): Promise<void> {
    if (!this.request) return;
    const result = await this.options.client.view(this.request).catch(() => NO_ANSWER);
    if (result.ok) {
      if (!isShowableView(result.body)) {
        this.refuse(REFUSAL.consentUnavailable, false);
        return;
      }
      this.set({ view: result.body, phase: 'review' });
      return;
    }
    const failure = failureFor(result);
    if (failure.kind === 'sign-in') {
      this.sendToSignIn();
      return;
    }
    // A refusal before anything was agreed is not remembered: asking again
    // later (a node that gains the endpoint, or its signer) should be able to succeed.
    this.refuse(failure.code, false);
  }

  private restore(
    view: ConsentViewResponse | null,
    outcome: RememberedOutcome,
    shown: { trail: WitnessStep[] | null; standing: ApprovalStanding | null }
  ): void {
    // Shown exactly as recorded; nothing is re-run to rebuild it.
    this.set(outcome.phase === 'signing' ? { view } : { view, ...shown });
    switch (outcome.phase) {
      case CODE:
        this.set({ phase: CODE, code: outcome.code, expiresAt: outcome.expiresAt });
        return;
      case HANDED_BACK:
      case 'declined':
        this.set({ phase: outcome.phase });
        return;
      case 'refused':
        this.refuse(outcome.code, false);
        return;
      default:
        // Left while signing: whether it was signed is unknown here, and an
        // approval is never sent twice from one link.
        this.refuse(REFUSAL.approvalInterrupted, false);
    }
  }

  /** Say plainly that signing in comes first, then go and come straight back. */
  private sendToSignIn(): void {
    this.set({ trail: null });
    this.refuse(NODE_CODE.notSignedIn, false);
    this.options.signIn();
  }

  private refuse(code: string, remember = true, reason?: string): void {
    this.set({ phase: 'refused', refusalCode: code, refusalReason: reason });
    if (remember) this.remember({ phase: 'refused', code });
  }

  private remember(outcome: RememberedOutcome): void {
    if (!this.requestParam) return;
    const { view, trail, standing } = this.current;
    this.memory.remember(this.requestParam, { view, outcome, trail, standing });
  }

  private set(partial: Partial<DeviceConsentPageState>): void {
    this.current = { ...this.current, ...partial };
    this.options.onChange(this.current);
  }
}
