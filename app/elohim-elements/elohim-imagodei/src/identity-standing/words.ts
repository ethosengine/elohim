/**
 * What an identity rests on, said plainly, built around the host's own name
 * for the node answering ("this device"). A fact, never a warning: one steward
 * is a whole identity, and only a quorum the person set up asks for more.
 *
 * English only, like the elements' own defaults.
 */

import type { IdentityStandingView } from './wire.js';

/** How the person knows the node answering. */
export interface StandingHostWords {
  /** Inside a sentence: "this device". */
  inSentence: string;
}

/** The one line saying what the identity rests on; none when the view gives no stewards. */
// eslint-disable-next-line sonarjs/function-return-type -- a sentence or none: none when there is nothing to say
export function standingLine(
  view: IdentityStandingView | null | undefined,
  { inSentence }: StandingHostWords
): string | undefined {
  if (!view || view.controllerCount < 1) return undefined;
  const n = view.controllerCount;
  if (n === 1) {
    return view.thisNodeIsController
      ? `Your identity rests on ${inSentence} alone.`
      : `Your identity rests on one of your own devices, not on ${inSentence}.`;
  }
  const among = view.thisNodeIsController ? `, ${inSentence} among them` : '';
  const approve =
    view.required > 1
      ? `${view.required} of them must agree to approve a new device.`
      : 'Any one of them can approve a new device.';
  return `Your identity rests on ${n} of your own devices${among}. ${approve}`;
}
