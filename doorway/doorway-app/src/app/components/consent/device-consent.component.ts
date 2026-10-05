/**
 * Approve a device — `/threshold/consent/device?request=<base64url GrantRequest>`.
 *
 * The doorway's mount of the shared approval page
 * (`elohim-imagodei/device-consent`, which the native portal mounts too). A
 * hosted person signs in first (the route's authGuard sends them to /login and
 * back), then the shared controller runs the page against this doorway, which
 * holds their key and signs as them.
 *
 * This component only supplies what is the doorway's own: who the key holder
 * is called, the session check, the way to sign in and come back, and the
 * bearer-token client. Everything else is the shared piece.
 */

import { ChangeDetectionStrategy, Component, OnInit, inject, signal } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';

import {
  DeviceConsentController,
  type DeviceConsentApproval,
  type DeviceConsentPageState,
} from 'elohim-imagodei/device-consent';

import { gatewayDomain } from '../../core/gateway-domain';
import { loginRedirect } from '../../core/guards/auth.guard';
import { AuthStateService } from '../../services/auth-state.service';
import { DEVICE_CONSENT_PORT } from '../../services/device-consent-port';

import { DeviceConsentViewComponent } from './device-consent-view.component';

@Component({
  selector: 'app-device-consent',
  standalone: true,
  imports: [DeviceConsentViewComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @if (state(); as s) {
      <app-device-consent-view
        [phase]="s.phase"
        [request]="s.view"
        [personLabel]="personLabel()"
        [hostLabel]="hostLabel"
        [code]="s.code"
        [expiresAt]="s.expiresAt"
        [refusalCode]="s.refusalCode"
        [trail]="s.trail"
        [standing]="s.standing"
        (approved)="onApprove($event)"
        (declined)="onDecline()"
        (expired)="onExpired()"
      />
    }
  `,
})
export class DeviceConsentComponent implements OnInit {
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly authState = inject(AuthStateService);
  private readonly port = inject(DEVICE_CONSENT_PORT);

  readonly personLabel = signal<string | undefined>(undefined);
  readonly hostLabel = gatewayDomain(globalThis.location.hostname);
  /** Null until the person is known to be signed in; nothing is shown before. */
  readonly state = signal<DeviceConsentPageState | null>(null);

  private controller: DeviceConsentController | null = null;

  ngOnInit(): void {
    // The route's guard already enforces this; holding the line here too means
    // nothing about the request is fetched or shown to a signed-out person.
    if (!this.authState.isAuthenticated()) {
      this.sendToSignIn();
      return;
    }
    this.personLabel.set(this.authState.account()?.identifier ?? undefined);

    const controller = new DeviceConsentController({
      requestParam: this.route.snapshot.queryParamMap.get('request'),
      holder: { relation: 'your-doorway', label: this.hostLabel },
      client: this.port.client,
      signIn: () => this.sendToSignIn(),
      handBack: url => this.port.handBack(url),
      onChange: state => this.state.set(state),
    });
    this.controller = controller;
    this.state.set(controller.state);
    void controller.start();
  }

  onApprove(approval: DeviceConsentApproval): void {
    void this.controller?.approve(approval);
  }

  onDecline(): void {
    this.controller?.decline();
  }

  onExpired(): void {
    this.controller?.expired();
  }

  private sendToSignIn(): void {
    void this.router.navigateByUrl(loginRedirect(this.router, this.router.url));
  }
}
