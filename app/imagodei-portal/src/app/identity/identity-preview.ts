/**
 * DEVELOPMENT ONLY — sample identity states for the native portal's root,
 * for looking at them without a node.
 *
 * `<portal base>identity/preview?…`
 * - `phase=begin|beginning|standing|sign-in|signing-in|secret-unset|not-signed-in`
 *   (default begin); `not-signed-in` is a node that gives no more than that
 * - sign-in: `refusal=invalid|slowed|paused|needs-session-key|needs-secure-channel|missing`
 *   (`retry=<seconds>` for slowed, `reason=<text>` for paused), `word=<text>`
 * - signed in (`phase=standing`): `asks=listed|empty|nobody|unknown|absent`
 *   (default listed), `delivered=0` to show a code instead of a hand-back
 * - `sign_in=1` — sent back to sign in again (the form, though a session is open)
 * - `refusal=<code>` — a refused begin (e.g. identity_name_malformed,
 *   consent_caller_not_local, consent_signing_unavailable)
 * - `name=<text>` — the name already typed
 * - `devices=<n>&required=<n>&here=0|1` — what the identity rests on (default
 *   1 of 1, this device one of them); `created=1` says a session was opened;
 *   `identifier=<word>` is the sign-in word the node gives, `name=<text>` the
 *   name the person asked to be shown
 *
 * AppComponent imports this only when `ngDevMode` is on, which optimized
 * (production) builds replace with `false`. Nothing here calls anything.
 */

import type { IdentityPageState, IdentityStandingView } from 'elohim-imagodei/identity-standing';
import type { SignInFailure, SignInPageState } from 'elohim-imagodei/node-sign-in';
import type { PendingAsksClient, PendingAsksView } from 'elohim-imagodei/pending-asks';

export function previewIdentityState(search: string): IdentityPageState {
  const params = new URLSearchParams(search);
  const phase = params.get('phase') ?? 'begin';
  const devices = Math.max(1, Number(params.get('devices') ?? 1));
  const here = params.get('here') !== '0';
  const standing: IdentityStandingView = {
    identityRoot: 'uhCAkJ3u…root',
    authority: 'uhCEkV7q…authority',
    networkDna: 'uhC0kP2m…dna',
    controllers: Array.from({ length: devices }, (_, i) => `uhCAkdevice${i}`),
    controllerCount: devices,
    required: Math.max(1, Number(params.get('required') ?? 1)),
    thisNodeIsController: here,
    restsOnThisNodeAlone: here && devices === 1,
    ...(params.get('identifier') ? { identifier: params.get('identifier')! } : {}),
  };
  if (phase === 'not-signed-in') return { phase: 'not-signed-in', standing: null };
  if (phase === 'sign-in' || phase === 'signing-in') {
    return {
      phase: 'not-signed-in',
      standing: null,
      signedOut: { hasIdentity: true, signInSecretSet: true },
    };
  }
  if (phase === 'secret-unset') {
    return {
      phase: 'not-signed-in',
      standing: null,
      signedOut: { hasIdentity: true, signInSecretSet: false },
    };
  }
  if (phase === 'standing') {
    return {
      phase: 'standing',
      standing,
      created:
        params.get('created') === '1' ? { human: true, authority: true, session: true } : undefined,
      displayName: params.get('name') ?? undefined,
    };
  }
  return {
    phase: phase === 'beginning' ? 'beginning' : 'begin',
    standing: null,
    displayName: params.get('name') ?? undefined,
    beginRefusal: params.get('refusal') ?? undefined,
  };
}

/** Whether the preview stands for a person sent back to sign in again. */
export function previewRootFlags(search: string): { signInAsked: boolean; keyLost: boolean } {
  return { signInAsked: new URLSearchParams(search).has('sign_in'), keyLost: false };
}

/** The sign-in form's sample state. */
export function previewSignInState(search: string): SignInPageState {
  const params = new URLSearchParams(search);
  const word = params.get('word') ?? undefined;
  if (params.get('phase') === 'signing-in') return { phase: 'signing-in', identifier: word };
  const kind = params.get('refusal');
  const refusal = ((): SignInFailure | undefined => {
    switch (kind) {
      case null:
        return undefined;
      case 'slowed':
        return { kind: 'slowed', retryAfter: Number(params.get('retry') ?? 42) };
      case 'paused':
        return {
          kind: 'paused',
          reason:
            params.get('reason') ??
            'This sign-in comes from a machine this device has not seen before, at an unusual hour.',
        };
      default:
        return { kind } as SignInFailure;
    }
  })();
  return { phase: 'form', identifier: word, ...(refusal ? { refusal } : {}) };
}

const PERSON = {
  identifier: 'matthew',
  displayName: 'Matthew',
  identityRoot: 'uhCkkJ3u…root',
  identityFingerprint: 'uhCkk…J3uQ',
};

/** A pending-asks client answering with samples; deciding answers at once. */
export function previewPendingClient(search: string): PendingAsksClient {
  const params = new URLSearchParams(search);
  const which = params.get('asks') ?? 'listed';
  const view: PendingAsksView = {
    carrier: which === 'absent' ? 'absent' : 'private-network',
    approver: 'uhCAkme',
    speaksFor:
      which === 'nobody' || which === 'unknown' ? { kind: which } : { kind: 'person', ...PERSON },
    speaksForWords:
      which === 'nobody'
        ? 'This node speaks for nobody, so it lists nothing: it is not one of the nodes that may approve a device for anyone.'
        : which === 'unknown'
          ? 'This node cannot say yet whom it speaks for, so it lists nothing for now.'
          : 'An approval here is for matthew (Matthew), identity uhCkk…J3uQ.',
    asks:
      which === 'listed'
        ? [
            {
              number: 1,
              label: 'laptop',
              deviceKey: 'uhCAklaptop',
              deviceFingerprint: 'uhCAk…7Lq2·Vr0d',
              deviceRootFingerprint: 'uhCkk…Ww3n·Q8tB',
              askedActs: ['device.enroll', 'device.bind-root'],
              secondsLeft: 252,
              state: { kind: 'unassigned' },
              stateWords: 'That device has no identity of its own yet.',
              addressedHere: true,
              forIdentity: PERSON,
            },
            {
              number: 2,
              label: 'garden-pi',
              deviceKey: 'uhCAkgarden',
              deviceFingerprint: 'uhCAk…c9Ty·M2aP',
              askedActs: ['device.enroll'],
              secondsLeft: 38,
              state: { kind: 'own-identity', made: '2026-09-30' },
              stateWords: 'That device already began an identity of its own.',
              addressedHere: false,
              forIdentity: PERSON,
            },
          ]
        : [],
  };
  return {
    list: async () => ({ ok: true, body: view }),
    decide: async body => ({
      ok: true,
      body: {
        number: Number(body.ask),
        decidedBy: 'answer',
        agreed: {
          returnTarget: { kind: 'display', value: 'K7QF-2MXD-9PLA' },
          expiresAt: Date.now() + 4 * 60_000,
          consentCid: 'bafyreiconsentpreview',
          controllers: { required: 1, signed: 1 },
          witnesses: [{ id: 'uhCAkme', act: 'signed', relation: 'this-device', state: 'done' }],
        },
        handedBack: { taken: params.get('delivered') !== '0' },
      },
    }),
  };
}
