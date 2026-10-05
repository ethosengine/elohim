/**
 * The native portal's device approval page, as presentation only: the
 * consent card, what the approval rests on, and who signed. Inputs in,
 * intents out — no calls, no state of its own. The mount drives it with the
 * shared controller; the development-only preview drives it with sample data.
 *
 * Host words: on a person's own node the key holder is "this device". The
 * trail leads with it (the element puts `this-device` first), and the card
 * says plainly that this device holds the key and signs.
 *
 * When this node has no identity for the person yet, the begin step takes
 * the card's place; when the page is open on another machine, the same link
 * is shown as a terminal command to run on the machine that holds the key.
 */

import {
  CUSTOM_ELEMENTS_SCHEMA,
  ChangeDetectionStrategy,
  Component,
  EventEmitter,
  Input,
  Output,
} from '@angular/core';
import {
  NODE_CODE,
  approvalWords,
  approveCommand,
  type DeviceConsentApproval,
  type DeviceConsentPageState,
} from 'elohim-imagodei/device-consent';
import { standingLine, type IdentityPageState } from 'elohim-imagodei/identity-standing';

import { IdentityViewComponent } from '../identity/identity-view.component.js';

const THIS_DEVICE_WORDS = approvalWords({ name: 'This device', inSentence: 'this device' });

const EMPTY_REQUEST = { clientId: '', label: '', deviceFingerprint: '', askedActs: [] };

@Component({
  selector: 'imagodei-portal-device-approval-view',
  standalone: true,
  imports: [IdentityViewComponent],
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <section class="device-approval" aria-labelledby="device-approval-title">
      <h2 id="device-approval-title" class="device-approval__title">Approve a device</h2>

      @if (state.phase === 'loading') {
        <p class="device-approval__loading" role="status" data-testid="device-consent-loading">
          Checking what this device is asking for…
        </p>
      } @else if (beginInPlace && identity) {
        <!-- This node has no identity for the person yet: begin it here, then
             come back to this approval. Nothing was signed before this. -->
        <imagodei-portal-identity-view
          [state]="identity"
          [returnNote]="returnNote"
          (begin)="beginIdentity.emit($event)"
        />
      } @else {
        <elohim-imagodei-device-consent-card
          data-testid="device-consent-card"
          signer="peer-conductor"
          [request]="cardRequest"
          [phase]="state.phase"
          [personLabel]="personLabel"
          [code]="state.code"
          [expiresAt]="state.expiresAt"
          [refusalCode]="state.refusalCode"
          [strings]="cardStrings"
          (approve)="onApprove($event)"
          (decline)="declined.emit()"
          (expired)="expired.emit()"
        ></elohim-imagodei-device-consent-card>

        @if (identityLine; as line) {
          <p class="device-approval__standing" data-testid="identity-standing">{{ line }}</p>
        }

        @if (elsewhere) {
          <div class="device-approval__elsewhere" data-testid="device-consent-elsewhere">
            <p>Open this same link in a browser there, or run this in its terminal:</p>
            <code class="device-approval__command" data-testid="device-consent-command">
              {{ command }}
            </code>
          </div>
        }

        @if (standingLine; as line) {
          <p class="device-approval__standing" data-testid="device-consent-standing">{{ line }}</p>
        }

        @if (state.trail; as steps) {
          @if (state.phase === 'signing' || state.phase === 'refused') {
            <elohim-imagodei-witness-trail
              class="device-approval__trail"
              data-testid="device-consent-witness-trail"
              layout="list"
              [steps]="steps"
              [strings]="trailStrings"
              [attr.reveal-after-ms]="trailRevealAfterMs"
            ></elohim-imagodei-witness-trail>
          } @else if (state.phase === 'code' || state.phase === 'handed-back') {
            <elohim-imagodei-witness-trail
              class="device-approval__trail"
              data-testid="device-consent-witness-trail"
              mode="settled"
              [steps]="steps"
              [strings]="trailStrings"
            ></elohim-imagodei-witness-trail>
          }
        }
      }
    </section>
  `,
  styles: [
    `
      :host {
        display: block;
      }

      .device-approval {
        display: grid;
        gap: 1rem;
        min-inline-size: 0;
      }

      .device-approval__title {
        margin: 0;
        font-size: 1.25rem;
      }

      .device-approval__loading {
        margin: 0;
      }

      .device-approval__elsewhere {
        display: grid;
        gap: 0.5rem;
      }

      .device-approval__elsewhere p {
        margin: 0;
        line-height: 1.5;
      }

      /* A command is selected whole to copy; it may wrap to fit, since its
         words are pasted as one line whatever the screen showed. */
      .device-approval__command {
        display: block;
        padding: 0.75rem 1rem;
        overflow-wrap: anywhere;
        user-select: all;
        font-family: ui-monospace, monospace;
        background: color-mix(in srgb, currentColor 6%, transparent);
        border-radius: 6px;
      }

      .device-approval__standing,
      .device-approval__trail {
        padding-block-start: 0.75rem;
        border-block-start: 1px solid color-mix(in srgb, currentColor 18%, transparent);
      }

      .device-approval__standing {
        margin: 0;
        line-height: 1.5;
      }

      .device-approval__standing + .device-approval__trail {
        padding-block-start: 0;
        border-block-start: 0;
      }
    `,
  ],
})
export class DeviceApprovalViewComponent {
  // Decorator inputs, like the rest of this bundle: its specs compile in JIT,
  // which does not see signal inputs.
  @Input({ required: true }) state!: DeviceConsentPageState;
  @Input() personLabel?: string;
  /** What the person's identity rests on, from the shared identity controller. */
  @Input() identity?: IdentityPageState;
  /** This page's own link, for the terminal command when it is open elsewhere. */
  @Input() link = '';
  /** How long the live trail waits before showing (the dev preview pins it to 0). */
  @Input() trailRevealAfterMs = 400;

  @Output() readonly approved = new EventEmitter<DeviceConsentApproval>();
  @Output() readonly declined = new EventEmitter<void>();
  @Output() readonly expired = new EventEmitter<void>();
  /** The person asked to begin their identity here, with this name. */
  @Output() readonly beginIdentity = new EventEmitter<string>();

  readonly cardStrings = THIS_DEVICE_WORDS.card;
  /** The standing line says what the approval rests on; the trail need not guess. */
  readonly trailStrings = THIS_DEVICE_WORDS.trail;

  get cardRequest() {
    return this.state.view ?? EMPTY_REQUEST;
  }

  /** Shown once the approval is done (a code, or handed back), from the node's own count. */
  get standingLine(): string | undefined {
    const { phase, standing } = this.state;
    return phase === 'code' || phase === 'handed-back'
      ? THIS_DEVICE_WORDS.standing(standing)
      : undefined;
  }

  /** No identity on this node yet: the begin step stands in for the card. */
  get beginInPlace(): boolean {
    const phase = this.identity?.phase;
    return phase === 'begin' || phase === 'beginning';
  }

  get returnNote(): string {
    const label = this.state.view?.label;
    return label
      ? `Then you’ll come back here to approve “${label}”. Nothing has been approved yet.`
      : 'Then you’ll come back here to the approval. Nothing has been approved yet.';
  }

  /** What the person's identity rests on, said while they decide. */
  get identityLine(): string | undefined {
    return this.state.phase === 'review' && this.identity?.phase === 'standing'
      ? standingLine(this.identity.standing, { inSentence: 'this device' })
      : undefined;
  }

  /** The node signs only for its own machine, and this page is elsewhere. */
  get elsewhere(): boolean {
    return this.state.phase === 'refused' && this.state.refusalCode === NODE_CODE.callerNotLocal;
  }

  get command(): string {
    return approveCommand(this.link);
  }

  onApprove(event: Event): void {
    const detail = (event as CustomEvent<Partial<DeviceConsentApproval>>).detail;
    if (!detail || !Array.isArray(detail.agreedActs)) return;
    this.approved.emit({ agreedActs: detail.agreedActs, declinedActs: detail.declinedActs ?? [] });
  }
}
