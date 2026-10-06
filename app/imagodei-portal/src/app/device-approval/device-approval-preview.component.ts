/**
 * DEVELOPMENT ONLY — the native portal's device approval states with sample
 * data, for looking at them without a node.
 *
 * `<portal base>consent/device/preview?…`
 * - `phase=review|signing|code|handed-back|declined|refused` (default review)
 * - `acts=1|2` — one asked act or both (default 2)
 * - `code=<refusal code>` — the refusal shown (default consent_unavailable)
 * - `witnesses=1` — sample reported witnesses under the code
 * - `required=<n>&signed=<n>` — the node's count (default 1 of 1)
 * - `reveal=now` — pin the live trail's reveal delay to 0 so a render shows it
 * - `begin=1` — this node has no identity yet: the begin step in place
 *   (`beginning=1` for its wait, `refusal=<code>` for a refused begin)
 * - `devices=<n>&required=<n>` (with `phase=review`) — what the identity
 *   rests on, said while the person decides
 * - `code=consent_reauthentication_asked` — a witness asks the person to sign
 *   in again (`reason=<text>` for the node's reason; a sample by default)
 * - `identifier=<word>` — the sign-in word the node gives with the standing
 * - `code=consent_caller_not_local` — the page open on another machine,
 *   with the terminal command for the same link
 *
 * AppComponent loads this only when `ngDevMode` is on, which optimized
 * (production) builds replace with `false`: the chunk is removed at build
 * time and no sample-data page exists on a live node. Nothing here calls
 * anything. The states are built with the shared piece's own helpers, so what
 * is previewed is what the controller would produce.
 */

import { ChangeDetectionStrategy, Component, computed } from '@angular/core';
import {
  keyHolderStep,
  standingFor,
  trailAfterAgreement,
  type ConsentViewResponse,
  type DeviceConsentPagePhase,
  type DeviceConsentPageState,
  type KeyHolderStep,
} from 'elohim-imagodei/device-consent';
import type { IdentityPageState } from 'elohim-imagodei/identity-standing';
import type { WitnessStep } from 'elohim-imagodei/witness-step';

import { DeviceApprovalViewComponent } from './device-approval-view.component.js';

const PHASES: readonly DeviceConsentPagePhase[] = [
  'review',
  'signing',
  'code',
  'handed-back',
  'declined',
  'refused',
];

const ONE_ACT: ConsentViewResponse = {
  clientId: 'epr-cli',
  label: 'workspace',
  deviceFingerprint: 'uhCAk…3FOt·ATGM',
  askedActs: ['device.enroll'],
};

const TWO_ACTS: ConsentViewResponse = {
  ...ONE_ACT,
  deviceRootFingerprint: 'uhCkk…PrcW·HA5_',
  askedActs: ['device.enroll', 'device.bind-root'],
};

const HOLDER: KeyHolderStep = { relation: 'this-device' };

/** A reason of the kind an attending witness gives (plain text, at most 280 characters). */
const SAMPLE_REASON =
  'You signed in on this device more than a day ago, and this approval lets a new device act for you.';

/** Sample `witnesses`: this device signed, and peers recorded it. */
const SAMPLE_WITNESSES: WitnessStep[] = [
  { id: 'this-device-sign', act: 'signed', relation: 'this-device', state: 'done' },
  { id: 'peers', act: 'recorded', relation: 'others', count: 3, state: 'done' },
];

@Component({
  selector: 'imagodei-portal-device-approval-preview',
  standalone: true,
  imports: [DeviceApprovalViewComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <imagodei-portal-device-approval-view
      [state]="state()"
      [identity]="identity"
      [link]="link"
      [trailRevealAfterMs]="revealAfterMs"
    />
  `,
})
export class DeviceApprovalPreviewComponent {
  private readonly params = new URLSearchParams(globalThis.location.search);

  /** Four minutes and a bit from when the preview opened. */
  private readonly expiresAt = Date.now() + 4 * 60_000 + 12_000;

  readonly revealAfterMs = this.params.get('reveal') === 'now' ? 0 : 400;

  /** A link of the shape a terminal prints, for the elsewhere command. */
  readonly link =
    'http://workspace.local:8090/auth/portal/consent/device?request=eyJjbGllbnRJZCI6ImVwci1jbGkifQ';

  /** What the identity controller would hold for this preview. */
  readonly identity: IdentityPageState = this.sampleIdentity();

  private sampleIdentity(): IdentityPageState {
    const p = this.params;
    if (p.get('begin') === '1') {
      return {
        phase: p.get('beginning') === '1' ? 'beginning' : 'begin',
        standing: null,
        displayName: p.get('name') ?? undefined,
        beginRefusal: p.get('refusal') ?? undefined,
      };
    }
    const devices = Math.max(1, Number(p.get('devices') ?? 1));
    return {
      phase: 'standing',
      standing: {
        identityRoot: 'uhCAkJ3u…root',
        authority: 'uhCEkV7q…authority',
        networkDna: 'uhC0kP2m…dna',
        controllers: Array.from({ length: devices }, (_, i) => `uhCAkdevice${i}`),
        controllerCount: devices,
        required: Math.max(1, Number(p.get('required') ?? 1)),
        thisNodeIsController: true,
        restsOnThisNodeAlone: devices === 1,
        ...(p.get('identifier') ? { identifier: p.get('identifier')! } : {}),
      },
    };
  }

  readonly state = computed<DeviceConsentPageState>(() => {
    const asked = this.params.get('phase') as DeviceConsentPagePhase | null;
    const phase = asked && PHASES.includes(asked) ? asked : 'review';
    const view = this.params.get('acts') === '1' ? ONE_ACT : TWO_ACTS;
    const witnesses = this.params.get('witnesses') === '1' ? SAMPLE_WITNESSES : undefined;
    const standing = standingFor({
      required: Number(this.params.get('required') ?? 1),
      signed: Number(this.params.get('signed') ?? 1),
    });
    const base: DeviceConsentPageState = { phase, view, trail: null, standing: null };
    switch (phase) {
      case 'signing':
        return { ...base, trail: [keyHolderStep(HOLDER, 'working')] };
      case 'code':
        return {
          ...base,
          code: 'K7QF-2MXD-9PLA',
          expiresAt: this.expiresAt,
          trail: trailAfterAgreement(HOLDER, witnesses),
          standing,
        };
      case 'handed-back':
        return { ...base, trail: trailAfterAgreement(HOLDER, witnesses), standing };
      case 'refused': {
        const code = this.params.get('code') ?? 'consent_unavailable';
        return {
          ...base,
          refusalCode: code,
          refusalReason:
            code === 'consent_reauthentication_asked'
              ? (this.params.get('reason') ?? SAMPLE_REASON)
              : undefined,
        };
      }
      default:
        return base;
    }
  });
}
