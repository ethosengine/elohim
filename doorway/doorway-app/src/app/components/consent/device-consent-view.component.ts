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

import {
  approvalWords,
  type ApprovalStanding,
  type ConsentViewResponse,
  type DeviceConsentApproval,
  type DeviceConsentPagePhase,
} from 'elohim-imagodei/device-consent';
import type { WitnessStep } from 'elohim-imagodei/witness-step';

/** The card's phases (the element's `phase` property). */
export type DeviceConsentPhase = Exclude<DeviceConsentPagePhase, 'loading'>;

/**
 * How this host names the node that holds a hosted person's key: their
 * doorway, which holds it and signs as them.
 */
const DOORWAY_WORDS = approvalWords({ name: 'Your doorway', inSentence: 'your doorway' });

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
            [strings]="cardStrings"
            (approve)="onApprove($event)"
            (decline)="declined.emit()"
            (expired)="expired.emit()"
          ></elohim-imagodei-device-consent-card>

          @if (standingLine(); as line) {
            <p class="portal-standing" data-testid="device-consent-standing">{{ line }}</p>
          }

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
  /** What the approval rests on, as the node counted it. */
  readonly standing = input<ApprovalStanding | null>(null);
  /** How long the live trail waits before showing (the dev preview pins it to 0). */
  readonly trailRevealAfterMs = input(400);

  readonly approved = output<DeviceConsentApproval>();
  readonly declined = output<void>();
  readonly expired = output<void>();

  readonly cardStrings = DOORWAY_WORDS.card;
  /** The standing line below says what the approval rests on; the trail need not guess. */
  readonly trailStrings = DOORWAY_WORDS.trail;

  readonly cardRequest = computed(() => this.request() ?? EMPTY_REQUEST);

  /** Shown once the approval is done (a code, or handed back), from the node's own count. */
  readonly standingLine = computed(() => {
    const phase = this.phase();
    return phase === 'code' || phase === 'handed-back'
      ? DOORWAY_WORDS.standing(this.standing())
      : undefined;
  });

  onApprove(event: Event): void {
    const detail = (event as CustomEvent<Partial<DeviceConsentApproval>>).detail;
    if (!detail || !Array.isArray(detail.agreedActs)) return;
    this.approved.emit({
      agreedActs: detail.agreedActs,
      declinedActs: Array.isArray(detail.declinedActs) ? detail.declinedActs : [],
    });
  }
}
