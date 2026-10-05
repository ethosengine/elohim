/**
 * Devices asking this node over a private network — reading the list, and
 * a review client per ask so the device approval page's card and controller
 * decide it exactly as they decide a link.
 *
 * Declining an ask is the card's own decline: it makes no call and no
 * signature, and the ask simply runs out on the node.
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

export interface PendingPageState {
  phase: PendingPhase;
  view: PendingAsksView | null;
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
    const result = await this.options.client.list().catch(() => NO_ANSWER);
    if (!result.ok || !result.body || !Array.isArray(result.body.asks)) {
      this.set({ phase: 'unavailable', view: null });
      return;
    }
    const view = result.body;
    if (view.carrier !== 'private-network') this.set({ phase: 'absent', view });
    else if (view.speaksFor?.kind === 'person') this.set({ phase: 'listed', view });
    else this.set({ phase: 'lists-nothing', view });
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
