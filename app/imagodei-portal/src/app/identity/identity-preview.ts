/**
 * DEVELOPMENT ONLY — sample identity states for the native portal's root,
 * for looking at them without a node.
 *
 * `<portal base>identity/preview?…`
 * - `phase=begin|beginning|standing|not-signed-in` (default begin); the last
 *   is the ordinary sign-in, under the header a node that answered gets
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
