/**
 * Devices asking this node over a private network — reading the list,
 * declining an ask, and a review client per ask so the device approval
 * page's card and controller approve it exactly as they approve a link.
 *
 * Declining is a real act here, unlike a link's decline: the asking
 * device's terminal is listening, so the node is told. It signs nothing,
 * the ask leaves the list, and the device is told so it stops waiting.
 */

import { base64url } from '../session-key/proof.js';

import type { ConsentAgreeResponse, DeviceConsentClient } from '../device-consent/wire.js';
import type { SameOriginResult } from '../same-origin.js';
import type { PendingAskView, PendingAsksClient, PendingAsksView } from './wire.js';

/**
 * - `listed`: this node speaks for a person; `view.asks` may be empty.
 * - `lists-nothing`: it speaks for nobody, or cannot say yet — one line, no list.
 * - `absent`: no private network here; say nothing.
 * - `unavailable`: no usable answer; say nothing.
 */
export type PendingPhase = 'loading' | 'listed' | 'lists-nothing' | 'absent' | 'unavailable';

/** What the panel says after the person declined an ask. */
export type PendingNotice =
  /** Declined: nothing approved; `told` when the asking device took the answer. */
  | { kind: 'declined'; label: string; told: boolean }
  /** The decline did not go through; `code` is the node's refusal, when it gave one. */
  | { kind: 'decline-failed'; label: string; code?: string };

export interface PendingPageState {
  phase: PendingPhase;
  view: PendingAsksView | null;
  notice?: PendingNotice;
  /** The number of the ask a decline is on its way for. */
  declining?: number;
}

export interface PendingAsksControllerOptions {
  client: PendingAsksClient;
  onChange: (state: PendingPageState) => void;
}

const NO_ANSWER = { ok: false, status: 0, body: null } as const;

export class PendingAsksController {
  private current: PendingPageState = { phase: 'loading', view: null };

  constructor(private readonly options: PendingAsksControllerOptions) {}

  get state(): PendingPageState {
    return this.current;
  }

  async read(): Promise<void> {
    const { notice } = this.current;
    const result = await this.options.client.list().catch(() => NO_ANSWER);
    if (!result.ok || !result.body || !Array.isArray(result.body.asks)) {
      this.set({ phase: 'unavailable', view: null, notice });
      return;
    }
    const view = result.body;
    if (view.carrier !== 'private-network') this.set({ phase: 'absent', view, notice });
    else if (view.speaksFor?.kind === 'person') this.set({ phase: 'listed', view, notice });
    else this.set({ phase: 'lists-nothing', view, notice });
  }

  /**
   * Decline `ask`: the node signs nothing, removes it, and tells the asking
   * device. One decline at a time; resolves true once the node has declined.
   */
  async decline(ask: PendingAskView): Promise<boolean> {
    if (this.current.declining !== undefined) return false;
    this.set({ ...this.current, declining: ask.number, notice: undefined });
    const result = await this.options.client
      .decide({ ask: String(ask.number), answer: { agreedActs: [] } })
      .catch(() => NO_ANSWER);
    const body = result.ok
      ? (result.body as { declined?: boolean; handedBack?: { taken?: boolean } })
      : null;
    if (body?.declined === true) {
      this.set({
        ...this.current,
        declining: undefined,
        notice: { kind: 'declined', label: ask.label, told: body.handedBack?.taken === true },
      });
      await this.read();
      return true;
    }
    const code = result.ok ? undefined : (result.body as { code?: unknown } | null)?.code;
    this.set({
      ...this.current,
      declining: undefined,
      notice: {
        kind: 'decline-failed',
        label: ask.label,
        ...(typeof code === 'string' ? { code } : {}),
      },
    });
    return false;
  }

  /** Clear the notice (the person moved on). */
  dismiss(): void {
    if (this.current.notice) this.set({ ...this.current, notice: undefined });
  }

  private set(state: PendingPageState): void {
    this.current = state;
    this.options.onChange(state);
  }
}

/**
 * What the device approval controller keys an ask by: base64url of a small
 * JSON naming it, so a decided ask is remembered in this tab like a link.
 * The node never sees it.
 */
export function pendingAskRequestParam(ask: PendingAskView): string {
  const json = JSON.stringify({ pendingAsk: ask.number, deviceKey: ask.deviceKey });
  return base64url(new TextEncoder().encode(json));
}

/**
 * A review client for one ask: the card shows what the node already listed
 * (no call), and approving decides the ask with the acts the person agreed
 * to. When the node handed the code back over the private network itself,
 * the answer says it was delivered; otherwise it is shown to type in.
 */
export function pendingAskConsentClient(
  client: PendingAsksClient,
  ask: PendingAskView
): DeviceConsentClient {
  return {
    // eslint-disable-next-line @typescript-eslint/promise-function-async -- answers at once from the listed ask
    view: () =>
      Promise.resolve({
        ok: true,
        body: {
          clientId: '',
          label: ask.label,
          deviceFingerprint: ask.deviceFingerprint,
          ...(ask.deviceRootFingerprint
            ? { deviceRootFingerprint: ask.deviceRootFingerprint }
            : {}),
          askedActs: ask.askedActs,
        },
      } as const),
    agree: async ({ agreedActs }) => {
      const result = await client.decide({ ask: String(ask.number), answer: { agreedActs } });
      if (!result.ok) return result;
      const body = result.body;
      const agreed = (body as { agreed?: ConsentAgreeResponse } | null)?.agreed;
      if (!agreed) {
        return {
          ok: false,
          status: 409,
          body: { code: 'pending_not_agreed' },
        } as SameOriginResult<never>;
      }
      const taken = (body as { handedBack?: { taken?: boolean } }).handedBack?.taken === true;
      return { ok: true, body: { ...agreed, delivered: taken } };
    },
  };
}
