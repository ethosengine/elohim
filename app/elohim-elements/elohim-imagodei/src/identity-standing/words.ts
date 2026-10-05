/**
 * What an identity rests on, said plainly, built around the host's own name
 * for the node answering ("this device"). A fact, never a warning: one device
 * speaking for the person is a whole identity, and only a quorum the person
 * set up asks for more. The person is the steward of their identity; a
 * device only speaks for them.
 *
 * English only, like the elements' own defaults.
 */

import type { IdentityStandingView } from './wire.js';

/** How the person knows the node answering. */
export interface StandingHostWords {
  /** Inside a sentence: "this device". */
  inSentence: string;
}

/** The one line saying what the identity rests on; none when no device speaks for the person. */
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
      : `One of your own devices speaks for you, and it is not ${inSentence}.`;
  }
  const among = view.thisNodeIsController ? `, ${inSentence} among them` : '';
  const approve =
    view.required > 1
      ? `A new device is approved for you once ${view.required} of them agree.`
      : 'Any one of them can approve a new device for you.';
  return `${n} of your own devices speak for you${among}. ${approve}`;
}
