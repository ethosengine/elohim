/**
 * The approval page's host-specific sentences, built around the name the
 * host gives the node that holds the person's key ("Your doorway", "This
 * device"). This file never names a kind of host itself: every sentence that
 * would differ between a doorway and a person's own node takes the host's
 * words.
 *
 * English only, like the elements' own defaults; a host with a catalogue
 * passes its own strings instead.
 */

import { REFUSAL, type ApprovalStanding } from './logic.js';

import type { DeviceConsentStringOverrides } from '../elohim-imagodei-device-consent-card.js';

/** How the person knows the node that holds their key. */
export interface KeyHolderWords {
  /** At the start of a sentence: "Your doorway", "This device". */
  name: string;
  /** Inside a sentence: "your doorway", "this device". */
  inSentence: string;
  /**
   * What to say when the key holder will sign only for a page on its own
   * machine and this page is elsewhere. Defaults to a sentence about "the
   * machine that holds your key", which names no host.
   */
  callerNotLocal?: string;
  /** Heading for the same refusal; defaults to "Approve on the machine that holds your key". */
  callerNotLocalHeading?: string;
}

/** The trail's own "rests on … alone" lines, which the standing line replaces on this page. */
export interface ApprovalTrailStrings {
  doorwayAlone: string;
  thisDeviceAlone: string;
}

export interface ApprovalWords {
  /** For `<elohim-imagodei-device-consent-card>`'s `strings`. */
  card: DeviceConsentStringOverrides;
  /** For `<elohim-imagodei-witness-trail>`'s `strings`. */
  trail: ApprovalTrailStrings;
  /** What the approval rests on, from the node's `controllers`; none when it said nothing. */
  standing: (standing: ApprovalStanding | null) => string | undefined;
}

/** Not a retry: the approval belongs on the machine that holds the key. */
const CALLER_NOT_LOCAL =
  'This page is open on a different machine from the one that holds your key, so nothing was signed. Approving happens on that machine.';
const CALLER_NOT_LOCAL_HEADING = 'Approve on the machine that holds your key';

/** "Your doorway (alpha.elohim.host)" when the card has a host name, else "Your doorway". */
const named = (name: string, host?: string): string => (host ? `${name} (${host})` : name);

export function approvalWords(holder: KeyHolderWords): ApprovalWords {
  const { name, inSentence } = holder;
  const signer = (host?: string) =>
    `${named(name, host)} holds your key, and will sign this as you when you approve.`;
  const signing = (host?: string) => `${named(name, host)} is signing this as you. One moment.`;

  return {
    card: {
      signerHosted: signer,
      signerOwnNode: signer,
      signingHosted: signing,
      signingOwnNode: signing,
      refusal: {
        act_unknown: `The device asked for something ${inSentence} doesn’t recognize, so there was nothing to approve.`,
        consent_identity_unbootstrapped: `${name} hasn’t recorded who you are yet, so it can’t approve anything for you. Nothing is wrong with this request.`,
        consent_signing_unavailable: `${name} can’t reach its signer right now, so nothing was signed. This is a wait, not a refusal: come back to this link in a few minutes and approve again.`,
        consent_caller_not_local: holder.callerNotLocal ?? CALLER_NOT_LOCAL,
        [REFUSAL.requestUnreadable]:
          'This link doesn’t carry a request that can be read. Start again from the terminal on your device.',
        [REFUSAL.returnPathRefused]: `${name} answered with somewhere other than this machine’s terminal to send the code, so it was not sent anywhere.`,
        [REFUSAL.approvalInterrupted]:
          'You left this page while it was being approved, so it was not sent a second time. If your device didn’t get a code, start again from its terminal.',
      },
      refusalHeading: {
        [REFUSAL.approvalInterrupted]: 'Not sent a second time',
        consent_caller_not_local: holder.callerNotLocalHeading ?? CALLER_NOT_LOCAL_HEADING,
      },
    },
    // The standing line says what the approval rests on, from the node's own
    // count; the trail's guess from its steps would only repeat or contradict it.
    trail: { doorwayAlone: '', thisDeviceAlone: '' },
    // eslint-disable-next-line sonarjs/function-return-type -- a sentence or none: none when the node gave no count
    standing: standing => {
      switch (standing?.kind) {
        case 'single':
          // A complete approval, said as a fact: nothing waits on other devices.
          return `${name} signed as you, and that is enough: this approval is complete. If you have other devices, they can affirm it later.`;
        case 'short':
          // Only a quorum the person set up asks for more than one.
          return `You asked that ${standing.required} of the devices that speak for you agree to approvals like this. ${standing.more} more of them must agree before it counts.`;
        case 'enough':
          return `This approval rests on ${standing.signed} of the devices that speak for you, as many as you asked for.`;
        default:
          return undefined;
      }
    },
  };
}
