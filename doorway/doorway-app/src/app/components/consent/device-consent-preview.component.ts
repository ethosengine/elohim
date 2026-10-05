/**
 * DEVELOPMENT ONLY — the portal's waiting and approval states with sample
 * data, for looking at them before (or without) a backend.
 *
 * `/threshold/consent/device/preview?…`
 * - device approval: `phase=review|signing|code|handed-back|declined|refused`,
 *   `acts=1|2`, `code=<refusal code>`, `witnesses=1` (sample reported
 *   witnesses), `required=<n>&signed=<n>` (the node's count; default 1 of 1)
 * - sign-in / create account in flight: `page=login|register`,
 *   `step=working|failed`
 * - `reveal=now` pins the witness trail's reveal delay to 0 so a render shows
 *   it at once (preview only; the real pages always wait 400ms)
 *
 * The route is registered only in dev mode (see app.routes.ts), so production
 * builds carry neither the route nor this chunk. Nothing here calls the doorway.
 */

import {
  ChangeDetectionStrategy,
  Component,
  afterNextRender,
  computed,
  inject,
  viewChild,
} from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { ActivatedRoute } from '@angular/router';

import {
  keyHolderStep,
  standingFor,
  trailAfterAgreement,
  type ConsentViewResponse,
  type KeyHolderStep,
} from 'elohim-imagodei/device-consent';
import type { WitnessStep } from 'elohim-imagodei/witness-step';

import { ThresholdLoginComponent } from '../login/threshold-login.component';
import { ThresholdRegisterComponent } from '../register/threshold-register.component';

import {
  DeviceConsentViewComponent,
  type DeviceConsentPhase,
} from './device-consent-view.component';

const PHASES: readonly DeviceConsentPhase[] = [
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

const HOST = 'alpha.elohim.host';
const HOLDER: KeyHolderStep = { relation: 'your-doorway', label: HOST };

/** Sample `witnesses`: the parties a node reports as having signed, in order. */
const SAMPLE_WITNESSES: WitnessStep[] = [
  { id: 'doorway-sign', act: 'signed', relation: 'your-doorway', label: HOST, state: 'done' },
  { id: 'peers', act: 'recorded', relation: 'others', count: 3, state: 'done' },
];

@Component({
  selector: 'app-device-consent-preview',
  standalone: true,
  imports: [DeviceConsentViewComponent, ThresholdLoginComponent, ThresholdRegisterComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @switch (page()) {
      @case ('login') {
        <app-threshold-login />
      }
      @case ('register') {
        <app-threshold-register />
      }
      @default {
        <app-device-consent-view
          [phase]="phase()"
          [request]="request()"
          personLabel="matthew"
          hostLabel="alpha.elohim.host"
          [code]="phase() === 'code' ? 'K7QF-2MXD-9PLA' : undefined"
          [expiresAt]="expiresAt"
          [refusalCode]="refusalCode()"
          [trail]="trail()"
          [standing]="standing()"
          [trailRevealAfterMs]="revealAfterMs()"
        />
      }
    }
  `,
})
export class DeviceConsentPreviewComponent {
  private readonly params = toSignal(inject(ActivatedRoute).queryParamMap);
  private readonly login = viewChild(ThresholdLoginComponent);
  private readonly register = viewChild(ThresholdRegisterComponent);

  readonly page = computed(() => this.params()?.get('page') ?? 'consent');

  readonly phase = computed<DeviceConsentPhase>(() => {
    const asked = this.params()?.get('phase') as DeviceConsentPhase | null;
    return asked && PHASES.includes(asked) ? asked : 'review';
  });

  readonly request = computed(() => (this.params()?.get('acts') === '1' ? ONE_ACT : TWO_ACTS));

  readonly refusalCode = computed(() => this.params()?.get('code') ?? 'consent_unavailable');

  readonly revealAfterMs = computed(() => (this.params()?.get('reveal') === 'now' ? 0 : 400));

  /** The trail each approval phase would carry. */
  readonly trail = computed<WitnessStep[] | null>(() => {
    const witnesses = this.params()?.get('witnesses') === '1' ? SAMPLE_WITNESSES : undefined;
    switch (this.phase()) {
      case 'signing':
        return [keyHolderStep(HOLDER, 'working')];
      case 'code':
      case 'handed-back':
        return trailAfterAgreement(HOLDER, witnesses);
      case 'refused':
        return this.params()?.has('code') ? null : [keyHolderStep(HOLDER, 'failed')];
      default:
        return null;
    }
  });

  /** What the node would report about the person's own nodes agreeing. */
  readonly standing = computed(() =>
    standingFor({
      required: Number(this.params()?.get('required') ?? 1),
      signed: Number(this.params()?.get('signed') ?? 1),
    })
  );

  /** Four minutes and a bit from when the preview opened. */
  readonly expiresAt = Date.now() + 4 * 60_000 + 12_000;

  constructor() {
    afterNextRender(() => this.pinInFlight());
  }

  /** Put the sign-in or create-account page into its after-click state. */
  private pinInFlight(): void {
    const failed = this.params()?.get('step') === 'failed';
    const reveal = this.revealAfterMs();
    const login = this.login();
    if (login) {
      login.form.identifier = 'matthew';
      login.form.password = 'a-password';
      login.trailRevealAfterMs.set(reveal);
      login.trailElementReady.set(true);
      login.trailSteps.set([
        {
          id: 'doorway-check',
          act: 'checked',
          relation: 'your-doorway',
          label: HOST,
          state: failed ? 'failed' : 'working',
        },
      ]);
      login.error.set(failed ? 'Invalid credentials' : '');
      login.state.set(failed ? 'form' : 'authenticating');
    }
    const register = this.register();
    if (register) {
      register.form = {
        displayName: 'Ada Lovelace',
        email: 'ada',
        password: 'a-long-enough-secret',
        confirmPassword: 'a-long-enough-secret',
      };
      register.error.set(
        failed ? 'That username is already taken here. Try another, or sign in.' : ''
      );
      register.state.set(failed ? 'form' : 'registering');
    }
  }
}
