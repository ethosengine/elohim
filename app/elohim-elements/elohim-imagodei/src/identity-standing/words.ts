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

/** Keep a person-given name from reordering the sentence around it. */
const isolate = (value: string): string => `\u2068${value}\u2069`;

/**
 * How the person is named: their sign-in word, with the name they asked to
 * be shown beside it when that differs ("matthew (Matthew)"). None without a
 * sign-in word — the record id is never shown to a person.
 */
// eslint-disable-next-line sonarjs/function-return-type -- a name or none
export function personName(
  view: IdentityStandingView | null | undefined,
  displayName?: string
): string | undefined {
  const word = view?.identifier?.trim();
  if (!word) return undefined;
  const shown = displayName?.trim();
  return isolate(shown && shown !== word ? `${word} (${shown})` : word);
}

/**
 * The one line saying what the identity rests on, leading with the person's
 * sign-in word when the node gives one; none when no device speaks for them.
 */
// eslint-disable-next-line sonarjs/function-return-type -- a sentence or none: none when there is nothing to say
export function standingLine(
  view: IdentityStandingView | null | undefined,
  { inSentence }: StandingHostWords,
  displayName?: string
): string | undefined {
  if (!view || view.controllerCount < 1) return undefined;
  const name = personName(view, displayName);
  const as = name ? ` as ${name}` : '';
  const n = view.controllerCount;
  if (n === 1) {
    if (view.thisNodeIsController) {
      return name
        ? `Your identity, ${name}, rests on ${inSentence} alone.`
        : `Your identity rests on ${inSentence} alone.`;
    }
    return `One of your own devices speaks for you${as}, and it is not ${inSentence}.`;
  }
  const among = view.thisNodeIsController ? `, ${inSentence} among them` : '';
  const approve =
    view.required > 1
      ? `A new device is approved for you once ${view.required} of them agree.`
      : 'Any one of them can approve a new device for you.';
  return `${n} of your own devices speak for you${as}${among}. ${approve}`;
}
