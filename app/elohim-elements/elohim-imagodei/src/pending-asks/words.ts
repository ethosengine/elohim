/**
 * What the pending-asks panel says, built from the node's answer. Whose
 * identity an approval is for leads with the sign-in word. A node that
 * speaks for nobody, or cannot say yet, gets the node's own one line (which
 * the node keeps person-safe) and no list.
 */

import { isSessionProofRefusal } from '../session-key/index.js';

import type { PendingNotice } from './controller.js';
import type { PendingAsksView, SpeaksForPerson } from './wire.js';

/** Keep a person-given name from reordering the sentence around it. */
const isolate = (value: string): string => `⁨${value}⁩`;

/** "matthew (Matthew)", "matthew", "Matthew"; none when the node named no one. */
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

/** After a decline: nothing was approved, and whether the asking device was told. */
export function noticeLine(notice: PendingNotice): string {
  const who = isolate(notice.label);
  if (notice.kind === 'declined') {
    return notice.told
      ? `Nothing was approved. “${who}” was told, so its terminal stops waiting.`
      : `Nothing was approved. “${who}” could not be told, so its terminal waits until its request runs out.`;
  }
  if (notice.code === 'consent_reauthentication_asked' || isSessionProofRefusal(notice.code)) {
    return `Nothing was declined: this browser’s sign-in can no longer be confirmed. Sign in again, then decline “${who}” again.`;
  }
  return `Nothing was declined: this device did not take the answer for “${who}”. Try again in a moment.`;
}

/** Whether a failed decline is one signing in again can help. */
export function noticeAsksSignIn(notice: PendingNotice | undefined): boolean {
  return (
    notice?.kind === 'decline-failed' &&
    (notice.code === 'consent_reauthentication_asked' || isSessionProofRefusal(notice.code))
  );
}

/**
 * What the asking node says of itself, as a sentence: the node gives a
 * predicate ("has no identity of its own"), which reads as "The asking
 * device has no identity of its own." — as the terminal says it.
 */
export function askingDeviceLine(stateWords: string): string {
  let words = stateWords.trim();
  while (words.endsWith('.')) words = words.slice(0, -1);
  return words ? `The asking device ${words}.` : '';
}

/** "About 4 minutes left", "40 seconds left", "No time left". */
export function timeLeft(seconds: number): string {
  if (seconds <= 0) return 'No time left';
  if (seconds < 60) return `${seconds} seconds left`;
  const minutes = Math.round(seconds / 60);
  return minutes === 1 ? 'About 1 minute left' : `About ${minutes} minutes left`;
}
