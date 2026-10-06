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
  approvePendingCommand,
  type DeviceConsentApproval,
  type DeviceConsentPageState,
} from 'elohim-imagodei/device-consent';
import { standingLine, type IdentityPageState } from 'elohim-imagodei/identity-standing';
import { isSessionProofRefusal } from 'elohim-imagodei/session-key';

import { IdentityViewComponent } from '../identity/identity-view.component.js';
import { NODE_SIDE } from '../identity/standing-words.js';

/**
 * The page may be open on another machine, signed in over a session, where
 * "this device" would read as the browser's own machine: the key holder is
 * named from the node's side, which is true wherever the page is open.
 */
const HOLDER = 'The device that holds your key';
const THIS_DEVICE_WORDS = approvalWords({
  name: HOLDER,
  inSentence: 'the device that holds your key',
});
const CARD_STRINGS = {
  ...THIS_DEVICE_WORDS.card,
  signerOwnNode: () => `${HOLDER} will sign this as you when you approve.`,
  signingOwnNode: () => `${HOLDER} is signing this as you. One moment.`,
};

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
          [attr.handed-back-to]="state.handedBackOverNetwork ? 'network' : 'this-machine'"
          [request]="cardRequest"
          [phase]="state.phase"
          [personLabel]="cardPersonLabel"
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

        @if (reauthentication) {
          <div class="device-approval__reauth" data-testid="device-consent-reauth">
            @if (state.refusalReason; as reason) {
              <p class="device-approval__reason-label">The reason given:</p>
              <p class="device-approval__reason" data-testid="device-consent-reauth-reason">
                {{ reason }}
              </p>
            }
            <button
              type="button"
              class="device-approval__action"
              data-testid="device-consent-sign-in-again"
              (click)="signInAgain.emit()"
            >
              Sign in again
            </button>
          </div>
        }

        @if (elsewhere) {
          <div class="device-approval__elsewhere" data-testid="device-consent-elsewhere">
            <button
              type="button"
              class="device-approval__action"
              data-testid="device-consent-sign-in"
              (click)="signInAgain.emit()"
            >
              Sign in
            </button>
            @if (pendingNumber !== undefined) {
              <p>Or, on the machine that holds your key, run this in its terminal:</p>
            } @else {
              <p>
                Or, on the machine that holds your key, open this same link in a browser there, or
                run this in its terminal:
              </p>
            }
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

      .device-approval__reauth {
        display: grid;
        gap: 0.5rem;
      }

      .device-approval__reauth p {
        margin: 0;
        line-height: 1.5;
      }

      .device-approval__reason {
        padding-inline-start: 0.75rem;
        border-inline-start: 3px solid color-mix(in srgb, currentColor 30%, transparent);
        overflow-wrap: anywhere;
      }

      .device-approval__action {
        justify-self: end;
        font: inherit;
        font-weight: 600;
        min-block-size: 44px;
        padding-inline: 1.25rem;
        color: ButtonText;
        background: ButtonFace;
        border: 1px solid ButtonText;
        border-radius: 6px;
      }

      .device-approval__action:focus-visible {
        outline: 2px solid currentColor;
        outline-offset: 2px;
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
  /** Say what the identity rests on under the card (off where the page already says it). */
  @Input() showIdentityLine = true;
  /** For an ask listed on the node: its number, which the terminal command names. */
  @Input() pendingNumber?: number;
  /** This page's own link, for the terminal command when it is open elsewhere. */
  @Input() link = '';
  /** How long the live trail waits before showing (the dev preview pins it to 0). */
  @Input() trailRevealAfterMs = 400;

  @Output() readonly approved = new EventEmitter<DeviceConsentApproval>();
  @Output() readonly declined = new EventEmitter<void>();
  @Output() readonly expired = new EventEmitter<void>();
  /** The person chose to sign in again, as a witness attending them asked. */
  @Output() readonly signInAgain = new EventEmitter<void>();
  /** The person asked to begin their identity here, with this name. */
  @Output() readonly beginIdentity = new EventEmitter<{
    displayName: string;
    secret?: string;
    identifier?: string;
  }>();

  readonly cardStrings = CARD_STRINGS;
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
    return this.showIdentityLine &&
      this.state.phase === 'review' &&
      this.identity?.phase === 'standing'
      ? standingLine(this.identity.standing, NODE_SIDE, this.identity.displayName)
      : undefined;
  }

  /**
   * Who is signed in, as the card names them: the host's own label, else the
   * sign-in word the node gave with the standing — a claim shown, never the
   * record id.
   */
  get cardPersonLabel(): string | undefined {
    return this.personLabel ?? (this.identity?.standing?.identifier?.trim() || undefined);
  }

  /**
   * A witness attending the person asked them to sign in again, or this
   * browser's sign-in can no longer be confirmed. Not a fault: a step.
   */
  get reauthentication(): boolean {
    const code = this.state.refusalCode;
    return (
      this.state.phase === 'refused' &&
      (code === NODE_CODE.reauthenticationAsked || isSessionProofRefusal(code))
    );
  }

  /** The node signs only for its own machine, and this page is elsewhere. */
  get elsewhere(): boolean {
    return this.state.phase === 'refused' && this.state.refusalCode === NODE_CODE.callerNotLocal;
  }

  /** The same approval from that machine's terminal: the ask's number, or this page's link. */
  get command(): string {
    return this.pendingNumber !== undefined
      ? approvePendingCommand(this.pendingNumber)
      : approveCommand(this.link);
  }

  onApprove(event: Event): void {
    const detail = (event as CustomEvent<Partial<DeviceConsentApproval>>).detail;
    if (!detail || !Array.isArray(detail.agreedActs)) return;
    this.approved.emit({ agreedActs: detail.agreedActs, declinedActs: detail.declinedActs ?? [] });
  }
}
