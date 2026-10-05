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
    <imagodei-portal-device-approval-view [state]="state()" [trailRevealAfterMs]="revealAfterMs" />
  `,
})
export class DeviceApprovalPreviewComponent {
  private readonly params = new URLSearchParams(globalThis.location.search);

  /** Four minutes and a bit from when the preview opened. */
  private readonly expiresAt = Date.now() + 4 * 60_000 + 12_000;

  readonly revealAfterMs = this.params.get('reveal') === 'now' ? 0 : 400;

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
      case 'refused':
        return { ...base, refusalCode: this.params.get('code') ?? 'consent_unavailable' };
      default:
        return base;
    }
  });
}
