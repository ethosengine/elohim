/**
 * Approve a device — `/threshold/consent/device?request=<base64url GrantRequest>`.
 *
 * A terminal on a device printed this link. The person signs in first (the
 * route's authGuard sends them to /login and back), then sees which device is
 * asking and what it wants, approves all or some, and gets a one-time code to
 * paste — or, when the terminal is on this machine, the browser hands the code
 * to it.
 *
 * This component only sequences the page. The request is checked, signed and
 * turned into a code by the runtime behind DeviceConsentService.
 */

import { ChangeDetectionStrategy, Component, OnInit, inject, signal } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';

import { gatewayDomain } from '../../core/gateway-domain';
import { loginRedirect } from '../../core/guards/auth.guard';
import type { WitnessStep } from '../../models/witness-step';
import { AuthStateService } from '../../services/auth-state.service';
import {
  DeviceConsentService,
  type ConsentViewResponse,
  type GrantRequestJson,
} from '../../services/device-consent.service';

import {
  forgetConsent,
  recallConsent,
  rememberConsent,
  type RememberedOutcome,
} from './device-consent-progress';
import {
  DeviceConsentViewComponent,
  type DeviceConsentApproval,
  type DeviceConsentPagePhase,
} from './device-consent-view.component';
import {
  REFUSAL,
  decodeConsentRequest,
  failureFor,
  isShowableView,
  outcomeForAgreement,
  signingTrail,
} from './device-consent.logic';

@Component({
  selector: 'app-device-consent',
  standalone: true,
  imports: [DeviceConsentViewComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <app-device-consent-view
      [phase]="phase()"
      [request]="view()"
      [personLabel]="personLabel()"
      [hostLabel]="hostLabel"
      [code]="code()"
      [expiresAt]="expiresAt()"
      [refusalCode]="refusalCode()"
      [trail]="trail()"
      (approved)="onApprove($event)"
      (declined)="onDecline()"
      (expired)="onExpired()"
    />
  `,
})
export class DeviceConsentComponent implements OnInit {
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly authState = inject(AuthStateService);
  private readonly consent = inject(DeviceConsentService);

  readonly phase = signal<DeviceConsentPagePhase>('loading');
  readonly view = signal<ConsentViewResponse | null>(null);
  readonly code = signal<string | undefined>(undefined);
  readonly expiresAt = signal<number | undefined>(undefined);
  readonly refusalCode = signal<string | undefined>(undefined);
  readonly personLabel = signal<string | undefined>(undefined);
  readonly hostLabel = gatewayDomain(globalThis.location.hostname);
  /** Who secured the approval: shown live while signing, settled once a code exists. */
  readonly trail = signal<WitnessStep[] | null>(null);

  /** The raw `request` parameter — the key this tab remembers the ceremony by. */
  private requestParam = '';
  private request: GrantRequestJson | null = null;

  ngOnInit(): void {
    // The route's guard already enforces this; holding the line here too means
    // nothing about the request is fetched or shown to a signed-out person.
    if (!this.authState.isAuthenticated()) {
      void this.router.navigateByUrl(loginRedirect(this.router, this.router.url));
      return;
    }
    this.personLabel.set(this.authState.account()?.identifier ?? undefined);

    this.requestParam = this.route.snapshot.queryParamMap.get('request') ?? '';
    const decoded = decodeConsentRequest(this.requestParam);
    if (!decoded.ok) {
      this.refuse(decoded.code, false);
      return;
    }
    this.request = decoded.request;

    const remembered = recallConsent(this.requestParam);
    if (remembered) {
      this.restore(remembered.view, remembered.outcome, remembered.trail ?? null);
      return;
    }
    void this.loadView();
  }

  async onApprove(approval: DeviceConsentApproval): Promise<void> {
    const request = this.request;
    if (this.phase() !== 'review' || !request || approval.agreedActs.length === 0) return;
    this.phase.set('signing');
    this.trail.set(signingTrail(this.hostLabel, 'working'));
    this.remember({ phase: 'signing' });

    try {
      const response = await this.consent.agree({ request, agreedActs: approval.agreedActs });
      // A 200 means the doorway signed; any witnesses it reports follow, as given.
      this.trail.set(signingTrail(this.hostLabel, 'done', response?.witnesses));
      const outcome = outcomeForAgreement(response);
      if (outcome.phase === 'code') {
        this.code.set(outcome.code);
        this.expiresAt.set(outcome.expiresAt);
        this.phase.set('code');
        this.remember({ phase: 'code', code: outcome.code, expiresAt: outcome.expiresAt });
      } else if (outcome.phase === 'handed-back') {
        this.phase.set('handed-back');
        this.remember({ phase: 'handed-back' });
        this.consent.handBack(outcome.url);
      } else {
        this.refuse(outcome.code);
      }
    } catch (error) {
      const failure = failureFor(error);
      if (failure.kind === 'sign-in') {
        // Nothing was signed; let the person sign in and approve again.
        this.trail.set(null);
        this.remember(null);
        this.sendToSignIn();
        return;
      }
      this.trail.set(signingTrail(this.hostLabel, 'failed'));
      this.refuse(failure.code);
    }
  }

  /** Declining signs nothing, so nothing is sent. */
  onDecline(): void {
    if (this.phase() !== 'review') return;
    this.phase.set('declined');
    this.remember({ phase: 'declined' });
  }

  /** The code ran out: the card says so; forget the code itself. */
  onExpired(): void {
    const expiresAt = this.expiresAt();
    if (this.phase() === 'code' && expiresAt !== undefined) {
      this.remember({ phase: 'code', expiresAt });
    }
  }

  private async loadView(): Promise<void> {
    if (!this.request) return;
    try {
      const view = await this.consent.view(this.request);
      if (!isShowableView(view)) {
        this.refuse(REFUSAL.consentUnavailable, false);
        return;
      }
      this.view.set(view);
      this.phase.set('review');
    } catch (error) {
      const failure = failureFor(error);
      if (failure.kind === 'sign-in') {
        this.sendToSignIn();
        return;
      }
      // A refusal before anything was agreed is not remembered: asking again
      // later (a doorway that gains the endpoint) should be able to succeed.
      this.refuse(failure.code, false);
    }
  }

  private restore(
    view: ConsentViewResponse | null,
    outcome: RememberedOutcome,
    trail: WitnessStep[] | null
  ): void {
    this.view.set(view);
    // Shown exactly as recorded; nothing is re-run to rebuild it.
    if (outcome.phase !== 'signing') this.trail.set(trail);
    switch (outcome.phase) {
      case 'code':
        this.code.set(outcome.code);
        this.expiresAt.set(outcome.expiresAt);
        this.phase.set('code');
        return;
      case 'handed-back':
      case 'declined':
        this.phase.set(outcome.phase);
        return;
      case 'refused':
        this.refuse(outcome.code, false);
        return;
      default:
        // Left while signing: whether it was signed is unknown here, and an
        // approval is never sent twice from one link.
        this.refuse(REFUSAL.approvalInterrupted, false);
    }
  }

  private refuse(code: string, remember = true): void {
    this.refusalCode.set(code);
    this.phase.set('refused');
    if (remember) this.remember({ phase: 'refused', code });
  }

  private remember(outcome: RememberedOutcome | null): void {
    if (!this.requestParam) return;
    if (outcome === null) {
      forgetConsent(this.requestParam);
      return;
    }
    rememberConsent(this.requestParam, { view: this.view(), outcome, trail: this.trail() });
  }

  private sendToSignIn(): void {
    void this.router.navigateByUrl(loginRedirect(this.router, this.router.url));
  }
}
