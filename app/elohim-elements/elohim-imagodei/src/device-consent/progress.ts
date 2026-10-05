/**
 * Where a device approval got to in this tab, so leaving and coming back
 * (Back, reload, a detour to sign in) shows the same answer instead of asking
 * again — an approval is never sent twice from one link.
 *
 * sessionStorage: scoped to this tab and this origin, gone when the tab
 * closes. The one-time code is kept only until it expires; it is useless to
 * anyone but the terminal holding the request's verifier.
 */

import type { WitnessStep } from '../witness-step.js';
import type { ApprovalStanding } from './logic.js';
import type { ConsentViewResponse } from './wire.js';

export type RememberedOutcome =
  | { phase: 'signing' }
  | { phase: 'code'; code?: string; expiresAt: number }
  | { phase: 'handed-back' }
  | { phase: 'declined' }
  | { phase: 'refused'; code: string };

export interface RememberedConsent {
  view: ConsentViewResponse | null;
  outcome: RememberedOutcome;
  /** Who secured the approval, as last shown. Restored as is, never re-asked. */
  trail?: WitnessStep[] | null;
  /** What the approval rests on, as the node last said. */
  standing?: ApprovalStanding | null;
}

/** Where the page keeps it; a host or a spec may pass its own. */
export interface ConsentMemory {
  recall(requestParam: string): RememberedConsent | null;
  remember(requestParam: string, value: RememberedConsent): void;
  forget(requestParam: string): void;
}

const PREFIX = 'elohim.device-consent:';

/** This tab's sessionStorage, or a memory that keeps nothing when storage is blocked. */
export function tabConsentMemory(
  storage: () => Storage | null | undefined = () => globalThis.sessionStorage
): ConsentMemory {
  // eslint-disable-next-line sonarjs/function-return-type -- a storage or none: none means this tab cannot remember
  const store = (): Storage | null => {
    try {
      return storage() ?? null;
    } catch {
      return null;
    }
  };
  return {
    // eslint-disable-next-line sonarjs/function-return-type -- an answer or none: none means ask the node
    recall(requestParam) {
      try {
        const raw = store()?.getItem(PREFIX + requestParam);
        if (!raw) return null;
        const value = JSON.parse(raw) as RememberedConsent;
        return value && typeof value === 'object' && value.outcome ? value : null;
      } catch {
        return null;
      }
    },
    remember(requestParam, value) {
      try {
        store()?.setItem(PREFIX + requestParam, JSON.stringify(value));
      } catch {
        // Storage full or blocked: the page still works, it just cannot recall.
      }
    },
    forget(requestParam) {
      try {
        store()?.removeItem(PREFIX + requestParam);
      } catch {
        // Nothing to forget if storage is unavailable.
      }
    },
  };
}
