/**
 * Devices asking this device over the private network, on the signed-in
 * root: each with its name, the key fingerprint to compare with the asking
 * device's terminal, what it asks, how long is left, what the asking node
 * says of itself, and whose identity an approval here is for.
 *
 * Reviewing an ask opens the same card and decision as a link: the shared
 * device approval controller, with a review client that decides the ask on
 * this node. Declining is a real act here: the node signs nothing, the ask
 * leaves the list, and the asking device is told so its terminal stops
 * waiting. A node that speaks for nobody gets one line and no list; with no
 * private network, nothing.
 */

import {
  ChangeDetectionStrategy,
  Component,
  EventEmitter,
  Input,
  OnInit,
  Output,
  signal,
} from '@angular/core';
import {
  DeviceConsentController,
  type DeviceConsentApproval,
  type DeviceConsentPageState,
} from 'elohim-imagodei/device-consent';
import type { IdentityPageState } from 'elohim-imagodei/identity-standing';
import {
  PendingAsksController,
  approvalIsFor,
  askingDeviceLine,
  listsNothingLine,
  noticeAsksSignIn,
  noticeLine,
  pendingAskConsentClient,
  pendingAskRequestParam,
  timeLeft,
  type PendingAskView,
  type PendingAsksClient,
  type PendingPageState,
} from 'elohim-imagodei/pending-asks';

import { DeviceApprovalViewComponent } from '../device-approval/device-approval-view.component.js';

/** The card's names for the acts, so the list and the card say the same. */
const ACT_NAMES: Record<string, string> = {
  'device.enroll': 'enroll this device',
  'device.bind-root': 'bind this device’s root key',
};

@Component({
  selector: 'imagodei-portal-pending-asks',
  standalone: true,
  imports: [DeviceApprovalViewComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @if (reviewing(); as review) {
      <div class="pending" data-testid="pending-review">
        <imagodei-portal-device-approval-view
          [state]="review"
          [identity]="identity"
          [showIdentityLine]="false"
          [pendingNumber]="reviewedNumber"
          (approved)="onApprove($event)"
          (declined)="onDecline()"
          (expired)="onExpired()"
          (signInAgain)="onSignInAgain()"
        />
        @if (state().notice; as notice) {
          <p class="pending__notice" role="status" data-testid="pending-notice">
            {{ lineFor(notice) }}
          </p>
          @if (asksSignIn) {
            <button
              type="button"
              class="pending__action"
              data-testid="pending-sign-in-again"
              (click)="signInNeeded.emit()"
            >
              Sign in again
            </button>
          }
        }
        @if (review.phase !== 'signing' && state().declining === undefined) {
          <button
            type="button"
            class="pending__action pending__action--quiet"
            data-testid="pending-back"
            (click)="backToList()"
          >
            Back to the list
          </button>
        }
      </div>
    } @else {
      @switch (state().phase) {
        @case ('listed') {
          <section class="pending" aria-labelledby="pending-title" data-testid="pending-asks">
            <h2 id="pending-title" class="pending__title">Devices asking to join</h2>
            @if (state().notice; as notice) {
              <p class="pending__notice" role="status" data-testid="pending-notice">
                {{ lineFor(notice) }}
              </p>
            }
            <p class="pending__muted">
              These devices are asking over the private network. Compare each key with the one its
              terminal shows before reviewing it.
            </p>
            <p data-testid="pending-for">{{ approvalFor }}</p>
            @for (ask of state().view?.asks ?? []; track ask.number) {
              <article class="pending__ask" [attr.data-testid]="'pending-ask-' + ask.number">
                <h3 class="pending__label">{{ ask.label }}</h3>
                <p class="pending__key">
                  <span class="pending__key-label">Device key</span>
                  <span class="pending__fingerprint">{{ ask.deviceFingerprint }}</span>
                </p>
                @if (ask.deviceRootFingerprint) {
                  <p class="pending__key">
                    <span class="pending__key-label">Root key</span>
                    <span class="pending__fingerprint">{{ ask.deviceRootFingerprint }}</span>
                  </p>
                }
                <p>Asks to {{ actsOf(ask) }}.</p>
                <p class="pending__muted">{{ askingDevice(ask) }}</p>
                <p class="pending__muted">{{ left(ask) }}</p>
                <button
                  type="button"
                  class="pending__action"
                  [attr.data-testid]="'pending-review-' + ask.number"
                  [disabled]="ask.secondsLeft <= 0"
                  (click)="review(ask)"
                >
                  Review
                </button>
              </article>
            } @empty {
              <p class="pending__muted" data-testid="pending-none">
                No device is asking right now.
              </p>
            }
          </section>
        }
        @case ('lists-nothing') {
          <p class="pending__line" data-testid="pending-lists-nothing">{{ nothingLine }}</p>
        }
      }
    }
  `,
  styles: [
    `
      :host {
        display: block;
      }

      .pending {
        display: grid;
        gap: 0.75rem;
        min-inline-size: 0;
      }

      .pending p,
      .pending__line {
        margin: 0;
        line-height: 1.5;
      }

      .pending__title {
        margin: 0;
        font-size: 1.125rem;
      }

      .pending__muted {
        opacity: 0.85;
      }

      .pending__notice {
        padding-inline-start: 0.75rem;
        border-inline-start: 3px solid color-mix(in srgb, currentColor 30%, transparent);
      }

      .pending__ask {
        display: grid;
        gap: 0.375rem;
        padding: 0.75rem 1rem;
        border: 1px solid color-mix(in srgb, currentColor 18%, transparent);
        border-radius: 6px;
      }

      .pending__label {
        margin: 0;
        font-size: 1rem;
      }

      .pending__key {
        display: flex;
        flex-wrap: wrap;
        gap: 0.5rem;
      }

      .pending__key-label {
        font-weight: 600;
      }

      .pending__fingerprint {
        font-family: ui-monospace, monospace;
        overflow-wrap: anywhere;
      }

      .pending__action {
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

      .pending__action--quiet {
        justify-self: start;
        font-weight: 400;
      }

      .pending__action:disabled {
        opacity: 0.6;
      }

      .pending__action:focus-visible {
        outline: 2px solid currentColor;
        outline-offset: 2px;
      }
    `,
  ],
})
export class PendingAsksComponent implements OnInit {
  // Decorator inputs, like the rest of this bundle.
  @Input({ required: true }) client!: PendingAsksClient;
  /** What the signed-in person's identity rests on, for the card's lines. */
  @Input() identity?: IdentityPageState;

  /** A decision asked the person to sign in (again); the root's sign-in brings them back. */
  @Output() readonly signInNeeded = new EventEmitter<void>();

  readonly state = signal<PendingPageState>({ phase: 'loading', view: null });
  readonly reviewing = signal<DeviceConsentPageState | null>(null);

  private list?: PendingAsksController;
  private decision?: DeviceConsentController;
  private reviewed?: PendingAskView;

  ngOnInit(): void {
    this.list = new PendingAsksController({
      client: this.client,
      onChange: state => this.state.set(state),
    });
    void this.list.read();
  }

  get approvalFor(): string {
    const speaks = this.state().view?.speaksFor;
    return approvalIsFor(speaks?.kind === 'person' ? speaks : undefined);
  }

  lineFor(notice: NonNullable<PendingPageState['notice']>): string {
    return noticeLine(notice);
  }

  get asksSignIn(): boolean {
    return noticeAsksSignIn(this.state().notice);
  }

  get nothingLine(): string | undefined {
    const view = this.state().view;
    return view ? listsNothingLine(view) : undefined;
  }

  actsOf(ask: PendingAskView): string {
    return ask.askedActs.map(act => ACT_NAMES[act] ?? act).join(' and ');
  }

  /** The node's predicate as a sentence: "The asking device has no identity of its own." */
  askingDevice(ask: PendingAskView): string {
    return askingDeviceLine(ask.stateWords);
  }

  get reviewedNumber(): number | undefined {
    return this.reviewed?.number;
  }

  left(ask: PendingAskView): string {
    return timeLeft(ask.secondsLeft);
  }

  /** Open the same card and decision as a link, for this ask. */
  review(ask: PendingAskView): void {
    this.list?.dismiss();
    this.reviewed = ask;
    this.decision = new DeviceConsentController({
      requestParam: pendingAskRequestParam(ask),
      holder: { relation: 'this-device' },
      client: pendingAskConsentClient(this.client, ask),
      signIn: () => this.signInNeeded.emit(),
      handBack: () => undefined,
      onChange: state => this.reviewing.set(state),
    });
    this.reviewing.set(this.decision.state);
    void this.decision.start();
  }

  onApprove(approval: DeviceConsentApproval): void {
    void this.decision?.approve(approval);
  }

  /**
   * Decline the ask on the node: nothing is signed, it leaves the list, and
   * the asking device is told. Back to the list with that said; a decline
   * the node did not take stays here, with why.
   */
  async onDecline(): Promise<void> {
    const ask = this.reviewed;
    if (!ask || !this.list) return;
    if (await this.list.decline(ask)) {
      this.decision = undefined;
      this.reviewed = undefined;
      this.reviewing.set(null);
    }
  }

  onSignInAgain(): void {
    this.decision?.signInAgain();
  }

  onExpired(): void {
    this.decision?.expired();
  }

  backToList(): void {
    this.decision = undefined;
    this.reviewed = undefined;
    this.list?.dismiss();
    this.reviewing.set(null);
    void this.list?.read();
  }
}
