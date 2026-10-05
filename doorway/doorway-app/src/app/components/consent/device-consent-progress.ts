/**
 * Where a device approval got to in this tab, so leaving and coming back
 * (Back, reload, a detour) shows the same answer instead of asking again —
 * an approval is never sent twice from one link.
 *
 * sessionStorage: scoped to this tab and this origin, gone when the tab
 * closes. The one-time code is kept only until it expires; it is useless to
 * anyone but the terminal holding the request's verifier.
 */

import type { WitnessStep } from '../../models/witness-step';
import type { ConsentViewResponse } from '../../services/device-consent.service';

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
}

const PREFIX = 'doorway.device-consent:';

function storage(): Storage | null {
  try {
    return globalThis.sessionStorage ?? null;
  } catch {
    return null;
  }
}

/** Read what this tab already did with the request in `requestParam`, if anything. */
export function recallConsent(requestParam: string): RememberedConsent | null {
  try {
    const raw = storage()?.getItem(PREFIX + requestParam);
    if (!raw) return null;
    const value = JSON.parse(raw) as RememberedConsent;
    return value && typeof value === 'object' && value.outcome ? value : null;
  } catch {
    return null;
  }
}

export function rememberConsent(requestParam: string, value: RememberedConsent): void {
  try {
    storage()?.setItem(PREFIX + requestParam, JSON.stringify(value));
  } catch {
    // Storage full or blocked: the page still works, it just cannot recall.
  }
}

export function forgetConsent(requestParam: string): void {
  try {
    storage()?.removeItem(PREFIX + requestParam);
  } catch {
    // Nothing to forget if storage is unavailable.
  }
}
