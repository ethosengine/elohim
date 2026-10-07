import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement, nothing, type PropertyValues } from 'lit';
import { property, state } from 'lit/decorators.js';

import type { DeviceAct } from './device-consent/wire.js';
import type { TrustMode } from './elohim-imagodei-trust-indicator.js';

/** The two acts a device can ask its person to agree to (its home is the approval wire). */
export type { DeviceAct } from './device-consent/wire.js';

/** What the device asked for (supplied by the host from the doorway's request view). */
export interface DeviceConsentRequest {
  clientId: string;
  /** The person's own name for the machine, e.g. `workspace`. */
  label: string;
  /** Short form of the device key, exactly as the terminal printed it. */
  deviceFingerprint: string;
  /** Short form of the device's root key, present when `device.bind-root` is asked. */
  deviceRootFingerprint?: string;
  askedActs: DeviceAct[];
}

export type DeviceConsentPhase =
  | 'review'
  | 'signing'
  | 'code'
  | 'handed-back'
  | 'declined'
  | 'refused';

/** Who signs when the person agrees: their doorway (hosted) or this node (their own). */
export type DeviceConsentSigner = TrustMode;

export interface DeviceConsentApproveDetail {
  agreedActs: DeviceAct[];
  declinedActs: DeviceAct[];
}

/** Refusal codes with a known human sentence. Any other code falls back to `refusedUnknown`. */
export type KnownDeviceRefusalCode =
  | 'request_acts_incoherent'
  | 'act_unknown'
  | 'redemption_expired'
  | 'consent_unavailable'
  | 'consent_not_signed_in'
  | 'consent_identity_unbootstrapped'
  | 'consent_signing_unavailable'
  | 'consent_caller_not_local'
  | 'consent_reauthentication_asked'
  | 'session_proof_missing'
  | 'session_proof_invalid'
  | 'session_proof_stale'
  | 'session_proof_replayed';

/**
 * Every visible sentence the card speaks. Hosts may replace any subset through
 * the `strings` property (e.g. from their own localisation catalogue); the
 * English defaults below are used for whatever is not supplied.
 */
export interface DeviceConsentStrings {
  reviewHeading: (label: string) => string;
  signedInAs: (person: string) => string;
  deviceKeyLabel: string;
  compareHint: string;
  asksLegend: string;
  enrollName: string;
  enrollWhat: string;
  bindRootName: string;
  bindRootWhat: string;
  rootKeyLabel: string;
  bindRootNeedsEnroll: string;
  scopeNote: string;
  signerHosted: (host?: string) => string;
  signerOwnNode: (host?: string) => string;
  nothingChosen: string;
  approve: string;
  decline: string;
  signingHeading: (label: string) => string;
  signingHosted: (host?: string) => string;
  signingOwnNode: (host?: string) => string;
  codeHeading: string;
  codePasteHint: string;
  codeLabel: string;
  copy: string;
  copied: string;
  selectedToCopy: string;
  commandSelectedToCopy: string;
  countdown: (clock: string) => string;
  announceMinutes: (minutes: number) => string;
  announceSeconds: (seconds: number) => string;
  expiredHeading: string;
  expiredBody: string;
  handedBackHeading: string;
  handedBackBody: string;
  /** The code went to the asking device over the private network, not to a terminal here. */
  handedBackOverNetworkHeading: string;
  handedBackOverNetworkBody: string;
  /** The node holds the code and the asking device collects it itself. */
  heldHeading: string;
  heldBody: (label: string) => string;
  declinedHeading: string;
  declinedBody: (label: string) => string;
  refusedHeading: string;
  /**
   * Headings that replace `refusedHeading` for one code — a wait or a missing
   * set-up is not a refusal and must not be headed as one.
   */
  refusalHeading: Partial<Record<KnownDeviceRefusalCode, string>>;
  refusal: Record<KnownDeviceRefusalCode, string>;
  refusedUnknown: string;
  refusalCodeLabel: string;
  /** Leads the terminal command a refusal may carry (`command`). */
  wayThroughLead: string;
  copyCommand: string;
}

/**
 * What a host may pass as `strings`: any sentence, and any refusal sentence or
 * heading by code — including codes the card has no default for. Refusal maps
 * merge per code with the defaults instead of replacing them.
 */
export type DeviceConsentStringOverrides = Partial<
  Omit<DeviceConsentStrings, 'refusal' | 'refusalHeading'>
> & {
  refusal?: Partial<Record<string, string>>;
  refusalHeading?: Partial<Record<string, string>>;
};

const SIGN_IN_AGAIN = 'Sign in again to go ahead';

/** This browser's sign-in can no longer be confirmed (any session-proof refusal). */
const SESSION_UNCONFIRMED =
  'This browser’s sign-in can no longer be confirmed, so nothing was signed. Sign in again, and you’ll come straight back here.';

const named = (host: string | undefined, fallback: string): string =>
  host ? `${fallback} (${host})` : fallback;

export const DEVICE_CONSENT_STRINGS_EN: DeviceConsentStrings = {
  reviewHeading: label => `A device called “${label}” wants to act for you`,
  signedInAs: person => `Signed in as ${person}`,
  deviceKeyLabel: 'Device key',
  compareHint: 'Check that this matches what your terminal printed. If it doesn’t, decline.',
  asksLegend: 'What this device is asking for',
  enrollName: 'enroll this device',
  enrollWhat: 'The network will recognize this machine as one of yours.',
  bindRootName: 'bind this device’s root key',
  bindRootWhat: 'What this machine produces can be traced back to you.',
  rootKeyLabel: 'Root key',
  bindRootNeedsEnroll:
    'A root key can only be bound on a device that is enrolled, so this is left out too.',
  scopeNote:
    'This only recognizes the device as yours. It does not, by itself, let the device change any of your content.',
  signerHosted: host =>
    `${named(host, 'Your doorway')} keeps your key, and will sign this as you when you approve.`,
  signerOwnNode: host =>
    `${named(host, 'This node')} holds your key, and will sign this as you when you approve.`,
  nothingChosen: 'Choose at least one thing to approve, or decline.',
  approve: 'Approve',
  decline: 'Decline',
  signingHeading: label => `Approving “${label}”…`,
  signingHosted: host => `${named(host, 'Your doorway')} is signing this as you. One moment.`,
  signingOwnNode: host => `${named(host, 'This node')} is signing this as you. One moment.`,
  codeHeading: 'Here is your code',
  codePasteHint: 'Paste this into the terminal that asked. Only that terminal can use it.',
  codeLabel: 'One-time code',
  copy: 'Copy code',
  copied: 'Copied',
  selectedToCopy: 'The code is selected. Copy it with your keyboard or menu.',
  commandSelectedToCopy: 'The command is selected. Copy it with your keyboard or menu.',
  countdown: clock => `Good for ${clock} more`,
  announceMinutes: minutes =>
    minutes === 1
      ? 'About 1 minute left to use this code.'
      : `About ${minutes} minutes left to use this code.`,
  announceSeconds: seconds => `${seconds} seconds left to use this code.`,
  expiredHeading: 'This code has run out',
  expiredBody:
    'It was only good for a few minutes. Start again from the terminal on your device to get a new one.',
  handedBackHeading: 'Your device has it',
  handedBackBody: 'The terminal on this machine has received the code. You can close this tab.',
  handedBackOverNetworkHeading: 'The device has it',
  handedBackOverNetworkBody:
    'The code reached the device over the private network, and it enrolls itself now. Nothing more is needed here.',
  heldHeading: 'Done',
  heldBody: label => `“${label}” will finish joining on its own; nothing to copy.`,
  declinedHeading: 'Nothing was approved',
  declinedBody: label => `“${label}” is not recognized as your device. You can close this tab.`,
  refusedHeading: 'This request can’t go ahead',
  refusalHeading: {
    consent_unavailable: 'Can’t approve here',
    consent_not_signed_in: 'Sign in first',
    consent_identity_unbootstrapped: 'Not ready to approve yet',
    consent_signing_unavailable: 'Waiting on the signer',
    consent_caller_not_local: 'Sign in, or approve on the machine that holds your key',
    consent_reauthentication_asked: SIGN_IN_AGAIN,
    session_proof_missing: SIGN_IN_AGAIN,
    session_proof_invalid: SIGN_IN_AGAIN,
    session_proof_stale: SIGN_IN_AGAIN,
    session_proof_replayed: SIGN_IN_AGAIN,
  },
  refusal: {
    request_acts_incoherent:
      'The device asked to bind its root key without also being enrolled, and that can’t be done on its own.',
    act_unknown:
      'The device asked for something your doorway doesn’t recognize, so there was nothing to approve.',
    redemption_expired:
      'The code ran out before it was used. Start again from the terminal on your device.',
    consent_unavailable: 'Approvals can’t be taken right now. Please try again in a little while.',
    consent_not_signed_in:
      'You need to be signed in to approve this. Sign in, and you’ll come straight back here.',
    consent_identity_unbootstrapped:
      'The node that holds your key hasn’t recorded who you are yet, so it can’t approve anything for you. Nothing is wrong with this request.',
    consent_signing_unavailable:
      'The node that holds your key can’t reach its signer right now, so nothing was signed. This is a wait, not a refusal: come back to this link in a few minutes and approve again.',
    consent_caller_not_local:
      'Nothing was signed: this page isn’t signed in, and it is open on a different machine from the one that holds your key. Sign in here, or approve on that machine.',
    consent_reauthentication_asked:
      'You’re being asked to sign in again before this approval goes ahead. Nothing was signed, and no code was issued.',
    session_proof_missing: SESSION_UNCONFIRMED,
    session_proof_invalid: SESSION_UNCONFIRMED,
    session_proof_stale: SESSION_UNCONFIRMED,
    session_proof_replayed: SESSION_UNCONFIRMED,
  },
  refusedUnknown: 'Something stopped this request. Start again from the terminal on your device.',
  refusalCodeLabel: 'Reference',
  wayThroughLead: 'Approve on a device that is already yours:',
  copyCommand: 'Copy command',
};

const ENROLL: DeviceAct = 'device.enroll';
const BIND_ROOT: DeviceAct = 'device.bind-root';
const ACT_ORDER: DeviceAct[] = [ENROLL, BIND_ROOT];
const KNOWN_ACTS = new Set<string>(ACT_ORDER);

/**
 * Wrap a person-supplied name (device label, person, host) in Unicode
 * first-strong isolates so it cannot reorder the sentence around it when the
 * name and the sentence run in different directions.
 */
const isolate = (value: string): string => `⁨${value}⁩`;

/**
 * Codes that are not a refusal at all — a step the person takes before the
 * approval goes ahead. They carry no reference code, which reads as a fault.
 */
const NOT_A_REFUSAL = new Set<string>([
  'consent_reauthentication_asked',
  'session_proof_missing',
  'session_proof_invalid',
  'session_proof_stale',
  'session_proof_replayed',
]);
const isolateOpt = (value?: string): string | undefined => (value ? isolate(value) : undefined);

/** Seconds-remaining thresholds below one minute at which the live region speaks. */
const ANNOUNCE_SECONDS = [30, 10];

/**
 * <elohim-imagodei-device-consent-card> — a person approves a device that is
 * asking to act for them.
 *
 * The device (a terminal on a machine with its own node) asked the person's
 * doorway to enroll it and, optionally, bind its root key. The card names the
 * device, shows its short key fingerprint for comparison with the terminal,
 * lists each asked act as its own agree/leave-out row, says who will sign, and
 * then walks the phases the host drives: signing → code (paste) or
 * handed-back (same machine, the private network, or held for the device to
 * collect itself) → or declined / refused. A refusal may carry the way
 * through as a terminal command to run on a device that is already the
 * person's (`command`), shown with a copy button.
 *
 * Coherence rule (protocol): a root key can only be bound alongside
 * enrollment. Leaving enrollment out also leaves the root key out and
 * disables its control with an explanation. Agreeing to nothing disables
 * Approve.
 *
 * Thin client: the card holds only the person's in-progress choice and a
 * display clock. The host owns the request, the signing, the code and the
 * phase; the card emits intent events.
 *
 * @element elohim-imagodei-device-consent-card
 *
 * @prop {DeviceConsentRequest} request - What the device asked for (property only)
 * @prop {DeviceConsentPhase} phase - review | signing | code | handed-back | declined | refused (default review)
 * @prop {DeviceConsentSigner} signer - doorway-host | peer-conductor (default doorway-host)
 * @prop {string} personLabel - Who is signed in (optional)
 * @prop {string} hostLabel - The doorway or node name (optional)
 * @prop {string} code - One-time code to paste (phase `code`)
 * @prop {number} expiresAt - Code expiry, epoch ms (phase `code`)
 * @prop {string} refusalCode - Machine code for phase `refused`
 * @prop {string} command - Phase `refused`: the way through, a terminal command to run on a device that is already the person's (optional)
 * @prop {'this-machine'|'network'|'held'} handedBackTo - Where the code went in phase `handed-back`: a terminal here, the device over the private network, or held for the device to collect itself (default this-machine)
 * @prop {DeviceConsentStringOverrides} strings - Replace any visible sentence; refusal sentences and headings merge per code (property only)
 *
 * @fires {CustomEvent<{agreedActs: DeviceAct[], declinedActs: DeviceAct[]}>} approve - Person approved; agreed/declined in askedActs order
 * @fires {CustomEvent<{reason: 'user-rejected'}>} decline - Person declined
 * @fires {CustomEvent<{method: 'clipboard'}>} code-copied - The code was written to the clipboard
 * @fires {CustomEvent<{method: 'clipboard'}>} command-copied - The refusal's command was written to the clipboard
 * @fires {CustomEvent<{expiresAt: number}>} expired - The shown code passed its expiry
 *
 * @cssprop --elohim-device-consent-gap - Grid gap between sections (default: 1rem)
 * @cssprop --elohim-device-consent-radius - Border radius for rows, code and buttons (default: 6px)
 * @cssprop --elohim-device-consent-muted-opacity - Opacity for secondary text (default: 0.8)
 * @cssprop --elohim-device-consent-key-bg - Fingerprint block background (default: 6% currentColor mix)
 * @cssprop --elohim-device-consent-act-row-bg - Act row background (default: transparent)
 * @cssprop --elohim-device-consent-act-row-border - Act row border (FULL shorthand; default: 1px solid 14% currentColor mix)
 * @cssprop --elohim-device-consent-toggle-accent - Act checkbox accent color (default: AccentColor)
 * @cssprop --elohim-device-consent-note-border - Scope note inline-start rule (FULL shorthand; default: 3px solid 30% currentColor mix)
 * @cssprop --elohim-device-consent-approve-bg - Approve button background (default: transparent)
 * @cssprop --elohim-device-consent-approve-fg - Approve button foreground (default: inherit)
 * @cssprop --elohim-device-consent-decline-bg - Decline button background (default: transparent)
 * @cssprop --elohim-device-consent-decline-fg - Decline button foreground (default: inherit)
 * @cssprop --elohim-device-consent-code-bg - One-time code background (default: 6% currentColor mix)
 * @cssprop --elohim-device-consent-code-font - One-time code font family (default: ui-monospace, monospace)
 * @cssprop --elohim-device-consent-code-size - One-time code font size (default: scales with the card's width, 1rem to 1.75rem); the code never wraps
 * @cssprop --elohim-device-consent-focus-ring - Focus outline (FULL shorthand; default: 2px solid currentColor)
 *
 * @csspart card - The outer container (carries data-phase)
 * @csspart heading - The phase heading (receives focus on phase change)
 * @csspart signed-in - The "signed in as" line
 * @csspart device-key - The device fingerprint block
 * @csspart fingerprint - A short key fingerprint value
 * @csspart act-row - One asked act (carries data-act)
 * @csspart scope-note - The "only recognizes the device" note
 * @csspart signer - The "who signs" line (review; replaced by `status` while signing)
 * @csspart actions - The approve/decline row
 * @csspart approve - The Approve button
 * @csspart decline - The Decline button
 * @csspart status - The signing busy status
 * @csspart code - The one-time code value
 * @csspart copy - The copy button
 * @csspart countdown - The visible countdown
 * @csspart message - Body sentence of the handed-back / declined / refused / expired phases
 * @csspart refusal-code - The raw refusal code (small, quotable)
 * @csspart way-through - The line leading a refusal's terminal command
 * @csspart command - A refusal's terminal command (selected whole to copy)
 * @csspart copy-command - The copy button for the command
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual
 * @capabilityRequiredStandings any
 * @capabilityContentCertainty observed
 * @capabilityStates empty:n/a, loading:designed, error:designed, stale:n/a, contested:n/a, offline:n/a
 */
export class ElohimImagodeiDeviceConsentCard extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
      font: inherit;
      min-inline-size: 0;
    }

    [part='card'] {
      display: grid;
      gap: var(--elohim-device-consent-gap, 1rem);
      min-inline-size: 0;
      /* The code's type scales with the card's own width (cqi below). */
      container-type: inline-size;
    }

    [part='heading'] {
      margin: 0;
      font-size: 1.25rem;
      line-height: 1.3;
      overflow-wrap: anywhere;
    }

    [part='heading']:focus {
      outline: none;
    }

    [part='heading']:focus-visible {
      outline: var(--elohim-device-consent-focus-ring, 2px solid currentColor);
      outline-offset: 4px;
    }

    p {
      margin: 0;
    }

    .muted,
    [part='signed-in'],
    [part='refusal-code'] {
      opacity: var(--elohim-device-consent-muted-opacity, 0.8);
      font-size: 0.875rem;
    }

    [part='device-key'] {
      display: grid;
      gap: 0.25rem;
      padding-block: 0.75rem;
      padding-inline: 0.75rem;
      background: var(
        --elohim-device-consent-key-bg,
        color-mix(in srgb, currentColor 6%, transparent)
      );
      border-radius: var(--elohim-device-consent-radius, 6px);
    }

    .key-line {
      display: flex;
      flex-wrap: wrap;
      gap: 0.25rem 0.5rem;
      align-items: baseline;
    }

    .key-label {
      font-weight: bold;
    }

    [part='fingerprint'] {
      font-family: var(--elohim-device-consent-code-font, ui-monospace, monospace);
      overflow-wrap: anywhere;
      unicode-bidi: isolate;
      direction: ltr;
    }

    fieldset {
      display: grid;
      gap: 0.5rem;
      margin: 0;
      padding: 0;
      border: 0;
      min-inline-size: 0;
    }

    legend {
      padding: 0;
      margin-block-end: 0.5rem;
      font-weight: bold;
    }

    [part='act-row'] {
      display: grid;
      grid-template-columns: auto 1fr;
      gap: 0.75rem;
      align-items: start;
      padding-block: 0.75rem;
      padding-inline: 0.75rem;
      background: var(--elohim-device-consent-act-row-bg, transparent);
      border: var(
        --elohim-device-consent-act-row-border,
        1px solid color-mix(in srgb, currentColor 14%, transparent)
      );
      border-radius: var(--elohim-device-consent-radius, 6px);
      cursor: pointer;
      min-inline-size: 0;
    }

    [part='act-row'][data-disabled] {
      cursor: default;
    }

    [part='act-row'] input[type='checkbox'] {
      margin: 0;
      margin-block-start: 0.2rem;
      inline-size: 1.125rem;
      block-size: 1.125rem;
      cursor: inherit;
      accent-color: var(--elohim-device-consent-toggle-accent, AccentColor);
    }

    [part='act-row'] input[type='checkbox']:focus-visible {
      outline: var(--elohim-device-consent-focus-ring, 2px solid currentColor);
      outline-offset: 2px;
    }

    .act-details {
      display: grid;
      gap: 0.25rem;
      min-inline-size: 0;
    }

    .act-name {
      font-weight: bold;
    }

    [part='scope-note'] {
      padding-inline-start: 0.75rem;
      border-inline-start: var(
        --elohim-device-consent-note-border,
        3px solid color-mix(in srgb, currentColor 30%, transparent)
      );
    }

    [part='actions'] {
      display: flex;
      flex-wrap: wrap;
      gap: 0.5rem;
      justify-content: end;
    }

    button {
      min-block-size: 2.5rem;
      padding-block: 0.5rem;
      padding-inline: 1rem;
      border: 1px solid currentcolor;
      border-radius: var(--elohim-device-consent-radius, 6px);
      font: inherit;
      cursor: pointer;
    }

    button:disabled {
      cursor: not-allowed;
      opacity: 0.6;
    }

    button:focus-visible {
      outline: var(--elohim-device-consent-focus-ring, 2px solid currentColor);
      outline-offset: 2px;
    }

    [part='approve'] {
      background: var(--elohim-device-consent-approve-bg, transparent);
      color: var(--elohim-device-consent-approve-fg, inherit);
      font-weight: bold;
    }

    [part='decline'] {
      background: var(--elohim-device-consent-decline-bg, transparent);
      color: var(--elohim-device-consent-decline-fg, inherit);
    }

    [part='copy'],
    [part='copy-command'] {
      background: transparent;
      color: inherit;
      justify-self: start;
    }

    /* A command is selected whole to copy; unlike the code it may wrap to
       fit, since it is pasted as one line whatever the screen showed. */
    [part='command'] {
      margin: 0;
      padding-block: 0.75rem;
      padding-inline: 1rem;
      background: var(
        --elohim-device-consent-code-bg,
        color-mix(in srgb, currentColor 6%, transparent)
      );
      border-radius: var(--elohim-device-consent-radius, 6px);
      font-family: var(--elohim-device-consent-code-font, ui-monospace, monospace);
      line-height: 1.5;
      overflow-wrap: anywhere;
      user-select: all;
      unicode-bidi: isolate;
      direction: ltr;
      text-align: start;
    }

    [part='code'] {
      margin: 0;
      padding-block: 0.75rem;
      padding-inline: 1rem;
      background: var(
        --elohim-device-consent-code-bg,
        color-mix(in srgb, currentColor 6%, transparent)
      );
      border-radius: var(--elohim-device-consent-radius, 6px);
      font-family: var(--elohim-device-consent-code-font, ui-monospace, monospace);
      /* The code is copied and compared by eye, so it stays on one line:
         its type shrinks to the card's width, and a block too narrow even
         for that scrolls instead of breaking it at a hyphen. */
      font-size: var(--elohim-device-consent-code-size, clamp(1rem, 8cqi, 1.75rem));
      white-space: nowrap;
      overflow-x: auto;
      letter-spacing: 0.08em;
      line-height: 1.3;
      overflow-wrap: anywhere;
      user-select: all;
      unicode-bidi: isolate;
      direction: ltr;
      text-align: start;
    }

    .code-tools {
      display: flex;
      flex-wrap: wrap;
      gap: 0.5rem 1rem;
      align-items: center;
    }

    [part='countdown'] {
      font-variant-numeric: tabular-nums;
    }

    .visually-hidden {
      position: absolute;
      inline-size: 1px;
      block-size: 1px;
      overflow: hidden;
      clip-path: inset(50%);
      white-space: nowrap;
    }

    @media (pointer: coarse) {
      button {
        min-block-size: 2.75rem;
      }

      [part='act-row'] {
        min-block-size: 2.75rem;
      }
    }

    @media (forced-colors: active) {
      [part='device-key'],
      [part='code'],
      [part='command'] {
        background: Canvas;
        color: CanvasText;
        border: 1px solid CanvasText;
      }

      [part='act-row'] {
        background: Canvas;
        color: CanvasText;
        border-color: CanvasText;
      }

      [part='scope-note'] {
        border-inline-start-color: CanvasText;
      }

      [part='approve'],
      [part='copy'],
      [part='copy-command'] {
        border-color: ButtonText;
        background: ButtonFace;
        color: ButtonText;
      }

      [part='decline'] {
        border-color: CanvasText;
        background: Canvas;
        color: CanvasText;
      }

      button:disabled {
        border-color: GrayText;
        color: GrayText;
        opacity: 1;
      }

      button:focus-visible,
      [part='heading']:focus-visible,
      [part='act-row'] input[type='checkbox']:focus-visible {
        outline: 2px solid Highlight;
      }
    }
  `;

  /** What the device asked for. */
  @property({ attribute: false }) request: DeviceConsentRequest = {
    clientId: '',
    label: '',
    deviceFingerprint: '',
    askedActs: [],
  };

  /** Which step the host has reached. */
  @property({ reflect: true }) phase: DeviceConsentPhase = 'review';

  /** Who signs when the person agrees. */
  @property() signer: DeviceConsentSigner = 'doorway-host';

  /** Who is signed in (optional). */
  @property({ attribute: 'person-label' }) personLabel?: string;

  /** The doorway or node name (optional). */
  @property({ attribute: 'host-label' }) hostLabel?: string;

  /** One-time code to paste (phase `code`). */
  @property() code?: string;

  /** Code expiry, epoch milliseconds (phase `code`). */
  @property({ attribute: 'expires-at', type: Number }) expiresAt?: number;

  /** Machine code for phase `refused`. */
  @property({ attribute: 'refusal-code' }) refusalCode?: string;

  /**
   * Phase `refused`: the way through, as a terminal command to run on a
   * device that is already the person's. Shown only when given.
   */
  @property() command?: string;

  /**
   * Where the code went in phase `handed-back`: a terminal on this machine
   * (`this-machine`, the default), the asking device over a private network
   * (`network`), or held by the node until the asking device collects it
   * itself (`held` — it finishes joining on its own, nothing to copy).
   */
  @property({ attribute: 'handed-back-to' }) handedBackTo: 'this-machine' | 'network' | 'held' =
    'this-machine';

  /** Replace any visible sentence; unspecified keys use the English defaults. */
  @property({ attribute: false }) strings: DeviceConsentStringOverrides = {};

  /** The person's choices, keyed by act. Absent = agreed (all asked acts start agreed). */
  @state() private _choice: Partial<Record<DeviceAct, boolean>> = {};
  @state() private _now = Date.now();
  @state() private _announcement = '';
  @state() private _copyNote = '';

  private _tick?: ReturnType<typeof setInterval>;
  private _expiryTimer?: ReturnType<typeof setTimeout>;
  private _expiredFor?: number;
  private _lastAnnounced?: string;

  private get _s(): DeviceConsentStrings {
    return { ...DEVICE_CONSENT_STRINGS_EN, ...this.strings } as DeviceConsentStrings;
  }

  /** Refusal sentences and headings by code: the defaults, then the host's, per code. */
  private get _refusals(): {
    sentence: Partial<Record<string, string>>;
    heading: Partial<Record<string, string>>;
  } {
    return {
      sentence: { ...DEVICE_CONSENT_STRINGS_EN.refusal, ...this.strings.refusal },
      heading: { ...DEVICE_CONSENT_STRINGS_EN.refusalHeading, ...this.strings.refusalHeading },
    };
  }

  /** Asked acts the card understands, in protocol order, de-duplicated. */
  private get _asked(): DeviceAct[] {
    const asked = new Set(this.request.askedActs.filter(a => KNOWN_ACTS.has(a)));
    return ACT_ORDER.filter(a => asked.has(a));
  }

  /** Effective agreement for one act after the coherence rule is applied. */
  private _isAgreed(act: DeviceAct): boolean {
    const asked = this._asked;
    if (!asked.includes(act)) return false;
    const chosen = this._choice[act] !== false;
    if (act === BIND_ROOT) {
      return chosen && this._isAgreed(ENROLL);
    }
    return chosen;
  }

  private get _agreedActs(): DeviceAct[] {
    return this._asked.filter(a => this._isAgreed(a));
  }

  private get _expired(): boolean {
    return (
      this.phase === 'code' && typeof this.expiresAt === 'number' && this._now >= this.expiresAt
    );
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this._stopClock();
  }

  override connectedCallback(): void {
    super.connectedCallback();
    // Re-attaching restarts the clock if a code is showing.
    if (this.hasUpdated) this._syncClock();
  }

  protected override willUpdate(changed: PropertyValues<this>): void {
    if (changed.has('request')) {
      const prev = changed.get('request');
      if (prev?.clientId !== this.request.clientId) {
        // A new request starts fully agreed.
        this._choice = {};
      }
    }
    if ((changed.has('phase') && this.phase !== 'code') || changed.has('command')) {
      this._copyNote = '';
    }
    if (changed.has('phase') || changed.has('expiresAt')) {
      this._now = Date.now();
      this._lastAnnounced = undefined;
      this._announcement = '';
      if (this.phase === 'code' && typeof this.expiresAt === 'number') {
        const remaining = this.expiresAt - this._now;
        if (remaining > 0) this._announce(remaining);
        else this._announcement = this._s.expiredHeading;
      }
    }
  }

  protected override updated(changed: PropertyValues<this>): void {
    if (changed.has('phase') || changed.has('expiresAt')) {
      this._syncClock();
    }
    if (changed.has('phase') && changed.get('phase') !== undefined) {
      this.shadowRoot?.querySelector<HTMLElement>('[part="heading"]')?.focus();
    }
  }

  // ── clock ────────────────────────────────────────────────────────────────

  private _syncClock(): void {
    this._stopClock();
    if (this.phase !== 'code' || typeof this.expiresAt !== 'number') return;
    const remaining = this.expiresAt - Date.now();
    if (remaining <= 0) {
      this._fireExpired();
      return;
    }
    this._tick = setInterval(() => this._onTick(), 1000);
    this._expiryTimer = setTimeout(() => this._onTick(), remaining);
  }

  private _stopClock(): void {
    if (this._tick !== undefined) clearInterval(this._tick);
    if (this._expiryTimer !== undefined) clearTimeout(this._expiryTimer);
    this._tick = undefined;
    this._expiryTimer = undefined;
  }

  private _onTick(): void {
    this._now = Date.now();
    if (typeof this.expiresAt !== 'number') return;
    const remaining = this.expiresAt - this._now;
    if (remaining <= 0) {
      this._stopClock();
      this._announcement = this._s.expiredHeading;
      this._fireExpired();
      return;
    }
    this._announce(remaining);
  }

  /** Fire `expired` once per expiry value. */
  private _fireExpired(): void {
    if (typeof this.expiresAt !== 'number' || this._expiredFor === this.expiresAt) return;
    this._expiredFor = this.expiresAt;
    this.dispatchEvent(
      new CustomEvent('expired', {
        detail: { expiresAt: this.expiresAt },
        bubbles: true,
        composed: true,
      })
    );
  }

  /**
   * Low-frequency live announcement: once per whole minute, then at 30s and
   * 10s. Never every second.
   */
  private _announce(remainingMs: number): void {
    const seconds = Math.ceil(remainingMs / 1000);
    let key: string | undefined;
    let text: string | undefined;
    if (seconds > 60) {
      const minutes = Math.ceil(seconds / 60);
      key = `m${minutes}`;
      text = this._s.announceMinutes(minutes);
    } else {
      const mark = ANNOUNCE_SECONDS.find(t => seconds <= t && seconds > t - 5);
      if (mark !== undefined) {
        key = `s${mark}`;
        text = this._s.announceSeconds(mark);
      } else if (this._lastAnnounced === undefined) {
        key = 'm1';
        text = this._s.announceMinutes(1);
      }
    }
    if (key && text && key !== this._lastAnnounced) {
      this._lastAnnounced = key;
      this._announcement = text;
    }
  }

  private _clock(): string {
    const remaining = Math.max(0, (this.expiresAt ?? 0) - this._now);
    const total = Math.ceil(remaining / 1000);
    const minutes = Math.floor(total / 60);
    const seconds = total % 60;
    const fmt = new Intl.NumberFormat(undefined, { minimumIntegerDigits: 2, useGrouping: false });
    return `${minutes}:${fmt.format(seconds)}`;
  }

  // ── render ───────────────────────────────────────────────────────────────

  override render() {
    const busy = this.phase === 'signing';
    return html`
      <section
        part="card"
        data-phase=${this.phase}
        aria-labelledby="heading"
        aria-busy=${busy ? 'true' : 'false'}
      >
        ${this._renderPhase()}
        <div class="visually-hidden" aria-live="polite" aria-atomic="true">
          ${this._announcement}
        </div>
      </section>
    `;
  }

  private _renderPhase() {
    switch (this.phase) {
      case 'signing':
        return this._renderReview(true);
      case 'code':
        return this._renderCode();
      case 'handed-back':
        return this._renderHandedBack();
      case 'declined':
        return this._renderMessage(
          this._s.declinedHeading,
          this._s.declinedBody(isolate(this.request.label))
        );
      case 'refused':
        return this._renderRefused();
      default:
        return this._renderReview(false);
    }
  }

  private _heading(text: string) {
    return html`
      <h2 id="heading" part="heading" tabindex="-1">${text}</h2>
    `;
  }

  private _renderReview(busy: boolean) {
    const s = this._s;
    const { label, deviceFingerprint } = this.request;
    const noneAgreed = this._agreedActs.length === 0;
    const hosted = this.signer !== 'peer-conductor';
    const host = isolateOpt(this.hostLabel);
    const signingLine = hosted ? s.signingHosted(host) : s.signingOwnNode(host);
    const signerLine = hosted ? s.signerHosted(host) : s.signerOwnNode(host);
    const named = isolate(label);
    return html`
      ${this._heading(busy ? s.signingHeading(named) : s.reviewHeading(named))}
      ${this.personLabel
        ? html`
            <p part="signed-in">${s.signedInAs(isolate(this.personLabel))}</p>
          `
        : nothing}
      <div part="device-key">
        <p class="key-line">
          <span class="key-label">${s.deviceKeyLabel}</span>
          <span part="fingerprint" data-key="device">${deviceFingerprint}</span>
        </p>
        <p class="muted">${s.compareHint}</p>
      </div>
      <fieldset ?disabled=${busy}>
        <legend>${s.asksLegend}</legend>
        ${this._asked.map(act => this._renderActRow(act, busy))}
      </fieldset>
      <p part="scope-note">${s.scopeNote}</p>
      ${busy
        ? html`
            <p part="status" role="status" data-signer=${this.signer}>${signingLine}</p>
          `
        : html`
            <p part="signer" data-signer=${this.signer}>${signerLine}</p>
          `}
      ${noneAgreed && !busy
        ? html`
            <p id="nothing-chosen" class="muted">${s.nothingChosen}</p>
          `
        : nothing}
      <div part="actions">
        <button type="button" part="decline" ?disabled=${busy} @click=${this._decline}>
          ${s.decline}
        </button>
        <button
          type="button"
          part="approve"
          ?disabled=${busy || noneAgreed}
          aria-describedby=${noneAgreed && !busy ? 'nothing-chosen' : nothing}
          @click=${this._approve}
        >
          ${s.approve}
        </button>
      </div>
    `;
  }

  private _renderActRow(act: DeviceAct, busy: boolean) {
    const s = this._s;
    const isRoot = act === BIND_ROOT;
    const blocked = isRoot && !this._isAgreed(ENROLL);
    const disabled = busy || blocked;
    const id = `act-${act.replace('.', '-')}`;
    const whatId = `${id}-what`;
    const blockedId = `${id}-blocked`;
    return html`
      <label part="act-row" data-act=${act} ?data-disabled=${disabled}>
        <input
          id=${id}
          type="checkbox"
          name="act"
          value=${act}
          .checked=${this._isAgreed(act)}
          ?disabled=${disabled}
          aria-describedby=${blocked ? `${whatId} ${blockedId}` : whatId}
          @change=${(e: Event) => this._toggle(act, (e.target as HTMLInputElement).checked)}
        />
        <span class="act-details">
          <span class="act-name">${isRoot ? s.bindRootName : s.enrollName}</span>
          <span id=${whatId} class="muted">${isRoot ? s.bindRootWhat : s.enrollWhat}</span>
          ${isRoot && this.request.deviceRootFingerprint
            ? html`
                <span class="key-line">
                  <span class="key-label">${s.rootKeyLabel}</span>
                  <span part="fingerprint" data-key="root">
                    ${this.request.deviceRootFingerprint}
                  </span>
                </span>
              `
            : nothing}
          ${blocked
            ? html`
                <span id=${blockedId} class="muted" data-blocked>${s.bindRootNeedsEnroll}</span>
              `
            : nothing}
        </span>
      </label>
    `;
  }

  private _renderCode() {
    const s = this._s;
    if (this._expired) {
      return html`
        ${this._heading(s.expiredHeading)}
        <p part="message">${s.expiredBody}</p>
      `;
    }
    return html`
      ${this._heading(s.codeHeading)}
      <p>${s.codePasteHint}</p>
      <p part="code" id="code" aria-label=${s.codeLabel}>${this.code ?? ''}</p>
      <div class="code-tools">
        <button type="button" part="copy" @click=${this._copy}>${s.copy}</button>
        ${typeof this.expiresAt === 'number'
          ? html`
              <span part="countdown" role="timer">${s.countdown(this._clock())}</span>
            `
          : nothing}
      </div>
      ${this._copyNote
        ? html`
            <p class="muted" role="status">${this._copyNote}</p>
          `
        : nothing}
    `;
  }

  private _renderHandedBack() {
    const s = this._s;
    switch (this.handedBackTo) {
      case 'network':
        return this._renderMessage(s.handedBackOverNetworkHeading, s.handedBackOverNetworkBody);
      case 'held':
        // Nothing to copy: the device collects the code itself. Its key is
        // shown once more, the one it printed, so the person knows which.
        return html`
          ${this._renderMessage(s.heldHeading, s.heldBody(isolate(this.request.label)))}
          ${this.request.deviceFingerprint
            ? html`
                <div part="device-key">
                  <p class="key-line">
                    <span class="key-label">${s.deviceKeyLabel}</span>
                    <span part="fingerprint" data-key="device">
                      ${this.request.deviceFingerprint}
                    </span>
                  </p>
                </div>
              `
            : nothing}
        `;
      default:
        return this._renderMessage(s.handedBackHeading, s.handedBackBody);
    }
  }

  private _renderMessage(heading: string, body: string) {
    return html`
      ${this._heading(heading)}
      <p part="message">${body}</p>
    `;
  }

  private _renderRefused() {
    const s = this._s;
    const code = this.refusalCode ?? '';
    const { sentence: sentences, heading: headings } = this._refusals;
    const byCode = (map: Partial<Record<string, string>>): string | undefined =>
      Object.hasOwn(map, code) ? map[code] : undefined;
    const sentence = byCode(sentences) ?? s.refusedUnknown;
    const heading = byCode(headings) ?? s.refusedHeading;
    return html`
      ${this._heading(heading)}
      <p part="message">${sentence}</p>
      ${this.command ? this._renderCommand(this.command) : nothing}
      ${code && !NOT_A_REFUSAL.has(code)
        ? html`
            <p part="refusal-code">
              ${s.refusalCodeLabel}:
              <code>${code}</code>
            </p>
          `
        : nothing}
    `;
  }

  /** The way through: a command to run on a device that is already the person's. */
  private _renderCommand(command: string) {
    const s = this._s;
    return html`
      <p part="way-through" id="way-through">${s.wayThroughLead}</p>
      <p part="command" aria-describedby="way-through">${command}</p>
      <div class="code-tools">
        <button type="button" part="copy-command" @click=${this._copyCommand}>
          ${s.copyCommand}
        </button>
      </div>
      ${this._copyNote
        ? html`
            <p class="muted" role="status">${this._copyNote}</p>
          `
        : nothing}
    `;
  }

  // ── intents ──────────────────────────────────────────────────────────────

  private _toggle(act: DeviceAct, on: boolean): void {
    this._choice = { ...this._choice, [act]: on };
  }

  private readonly _approve = (): void => {
    if (this.phase !== 'review') return;
    const agreedActs = this._agreedActs;
    if (agreedActs.length === 0) return;
    const declinedActs = this._asked.filter(a => !agreedActs.includes(a));
    this.dispatchEvent(
      new CustomEvent<DeviceConsentApproveDetail>('approve', {
        detail: { agreedActs, declinedActs },
        bubbles: true,
        composed: true,
      })
    );
  };

  private readonly _decline = (): void => {
    if (this.phase !== 'review') return;
    this.dispatchEvent(
      new CustomEvent('decline', {
        detail: { reason: 'user-rejected' },
        bubbles: true,
        composed: true,
      })
    );
  };

  private readonly _copy = async (): Promise<void> => {
    await this._copyText(this.code ?? '', 'code', 'code-copied');
  };

  private readonly _copyCommand = async (): Promise<void> => {
    await this._copyText(this.command ?? '', 'command', 'command-copied');
  };

  /** Write `text` to the clipboard; where that is blocked, select it for the person to copy. */
  private async _copyText(
    text: string,
    part: 'code' | 'command',
    event: 'code-copied' | 'command-copied'
  ): Promise<void> {
    if (!text) return;
    try {
      if (!navigator.clipboard?.writeText) throw new Error('clipboard unavailable');
      await navigator.clipboard.writeText(text);
      this._copyNote = this._s.copied;
      this.dispatchEvent(
        new CustomEvent(event, {
          detail: { method: 'clipboard' },
          bubbles: true,
          composed: true,
        })
      );
    } catch {
      this._selectPart(part);
      this._copyNote = part === 'code' ? this._s.selectedToCopy : this._s.commandSelectedToCopy;
    }
  }

  private _selectPart(part: 'code' | 'command'): void {
    const target = this.shadowRoot?.querySelector(`[part="${part}"]`);
    const selection = globalThis.getSelection?.();
    if (!target || !selection) return;
    const range = document.createRange();
    range.selectNodeContents(target);
    selection.removeAllRanges();
    selection.addRange(range);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-imagodei-device-consent-card': ElohimImagodeiDeviceConsentCard;
  }
}
