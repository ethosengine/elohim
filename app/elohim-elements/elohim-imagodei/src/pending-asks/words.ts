/**
 * What the pending-asks panel says, built from the node's answer. Whose
 * identity an approval is for leads with the sign-in word; the record id is
 * never shown. A node that speaks for nobody, or cannot say yet, gets the
 * node's own one line and no list.
 */

import type { PendingAsksView, SpeaksForPerson } from './wire.js';

/** Keep a person-given name from reordering the sentence around it. */
const isolate = (value: string): string => `⁨${value}⁩`;

/** "matthew (Matthew)", "matthew", "Matthew" — never the record id. */
// eslint-disable-next-line sonarjs/function-return-type -- a name or none
export function whoFor(person: Omit<SpeaksForPerson, 'kind'> | undefined): string | undefined {
  const word = person?.identifier?.trim();
  const name = person?.displayName?.trim();
  if (word && name && name !== word) return isolate(`${word} (${name})`);
  if (word) return isolate(word);
  return name ? isolate(name) : undefined;
}

/** Whose identity an approval here is for. */
export function approvalIsFor(person: Omit<SpeaksForPerson, 'kind'> | undefined): string {
  const who = whoFor(person);
  const fingerprint = person?.identityFingerprint;
  if (who && fingerprint) return `An approval here is for ${who}, identity ${fingerprint}.`;
  if (who) return `An approval here is for ${who}.`;
  return fingerprint
    ? `An approval here is for identity ${fingerprint}.`
    : 'An approval here is for the identity this device speaks for.';
}

/** The one line for a node that lists nothing, in the node's own words. */
// eslint-disable-next-line sonarjs/function-return-type -- a sentence or none
export function listsNothingLine(view: PendingAsksView): string | undefined {
  return view.speaksFor.kind === 'person' ? undefined : view.speaksForWords;
}

/** "About 4 minutes left", "40 seconds left", "No time left". */
export function timeLeft(seconds: number): string {
  if (seconds <= 0) return 'No time left';
  if (seconds < 60) return `${seconds} seconds left`;
  const minutes = Math.round(seconds / 60);
  return minutes === 1 ? 'About 1 minute left' : `About ${minutes} minutes left`;
}
