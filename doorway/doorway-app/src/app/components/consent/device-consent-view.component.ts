/**
 * The device approval page's presentation: the portal frame around
 * `<elohim-imagodei-device-consent-card>`. Inputs in, intents out — no calls,
 * no state of its own. The route component drives it; the development-only
 * preview drives it with sample data.
 *
 * The element is registered by the route loader (device-consent-elements.ts),
 * not here, so specs never load Lit.
 */

import {
  CUSTOM_ELEMENTS_SCHEMA,
  ChangeDetectionStrategy,
  Component,
  computed,
  input,
  output,
} from '@angular/core';

import type { WitnessStep } from '../../models/witness-step';
import type { ConsentViewResponse, DeviceAct } from '../../services/device-consent.service';

/** The card's phases (the element's `phase` property). */
export type DeviceConsentPhase =
  | 'review'
  | 'signing'
  | 'code'
  | 'handed-back'
  | 'declined'
  | 'refused';

/** Before the card can show anything, the page is checking the request. */
export type DeviceConsentPagePhase = 'loading' | DeviceConsentPhase;

/** `approve` event detail from the element. */
export interface DeviceConsentApproval {
  agreedActs: DeviceAct[];
  declinedActs: DeviceAct[];
}

const EMPTY_REQUEST: ConsentViewResponse = {
  clientId: '',
  label: '',
  deviceFingerprint: '',
  askedActs: [],
};

@Component({
  selector: 'app-device-consent-view',
  standalone: true,
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="portal-page">
      <section class="portal-card portal-card--wide" aria-labelledby="device-consent-title">
        <span class="portal-mark" role="img" aria-label="Elohim"></span>
        <div class="portal-heading">
          <h1 id="device-consent-title">Approve a device</h1>
        </div>

        @if (phase() === 'loading') {
          <div class="loading-state" role="status" data-testid="device-consent-loading">
            <div class="trace" aria-hidden="true">
              <span></span>
              <span></span>
              <span></span>
            </div>
            <p>Checking what this device is asking for…</p>
          </div>
        } @else {
          <elohim-imagodei-device-consent-card
            data-testid="device-consent-card"
            signer="doorway-host"
            [request]="cardRequest()"
            [phase]="phase()"
            [personLabel]="personLabel()"
            [hostLabel]="hostLabel()"
            [code]="code()"
            [expiresAt]="expiresAt()"
            [refusalCode]="refusalCode()"
            (approve)="onApprove($event)"
            (decline)="declined.emit()"
            (expired)="expired.emit()"
          ></elohim-imagodei-device-consent-card>

          @if (trail(); as steps) {
            @if (phase() === 'signing' || phase() === 'refused') {
              <elohim-imagodei-witness-trail
                class="portal-trail"
                data-testid="device-consent-witness-trail"
                layout="list"
                [steps]="steps"
                [strings]="trailStrings"
                [attr.reveal-after-ms]="trailRevealAfterMs()"
              ></elohim-imagodei-witness-trail>
            } @else if (phase() === 'code' || phase() === 'handed-back') {
              <elohim-imagodei-witness-trail
                class="portal-trail"
                data-testid="device-consent-witness-trail"
                mode="settled"
                [steps]="steps"
                [strings]="trailStrings"
              ></elohim-imagodei-witness-trail>
            }
          }
        }
      </section>
    </div>
  `,
  styleUrl: './device-consent-view.component.css',
})
export class DeviceConsentViewComponent {
  readonly phase = input.required<DeviceConsentPagePhase>();
  readonly request = input<ConsentViewResponse | null>(null);
  readonly personLabel = input<string | undefined>(undefined);
  readonly hostLabel = input<string | undefined>(undefined);
  readonly code = input<string | undefined>(undefined);
  readonly expiresAt = input<number | undefined>(undefined);
  readonly refusalCode = input<string | undefined>(undefined);
  /** Who secured the approval; live while signing, settled under a code. */
  readonly trail = input<WitnessStep[] | null>(null);
  /** How long the live trail waits before showing (the dev preview pins it to 0). */
  readonly trailRevealAfterMs = input(400);

  readonly approved = output<DeviceConsentApproval>();
  readonly declined = output<void>();
  readonly expired = output<void>();

  /** The element's default closing line says "sign-in"; this is an approval. */
  readonly trailStrings = { doorwayAlone: 'This approval rests on your doorway alone.' };

  readonly cardRequest = computed(() => this.request() ?? EMPTY_REQUEST);

  onApprove(event: Event): void {
    const detail = (event as CustomEvent<Partial<DeviceConsentApproval>>).detail;
    if (!detail || !Array.isArray(detail.agreedActs)) return;
    this.approved.emit({
      agreedActs: detail.agreedActs,
      declinedActs: Array.isArray(detail.declinedActs) ? detail.declinedActs : [],
    });
  }
}
