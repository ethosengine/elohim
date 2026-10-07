/**
 * Approve a device — `/threshold/consent/device?request=<base64url GrantRequest>`.
 *
 * The doorway's mount of the shared approval page
 * (`elohim-imagodei/device-consent`, which the native portal mounts too). The
 * shared controller runs the page against this doorway, which holds a hosted
 * person's key and signs as them.
 *
 * Sign-in is ordered here, not by a route guard: before a signed-out person is
 * sent to /login, the page asks this doorway whether it takes device approvals
 * at all (`POST /auth/consent/view`). If it does not (404, or no answer), the
 * page says so now, with the way through — approve on a device that is
 * already theirs with `epr device approve '<this link>'` — instead of after
 * sign-in. If it does (401, or a 200: `view` checks no session), the person
 * signs in and comes straight back, and the approval is reviewed then. Nothing
 * about the request is shown before sign-in.
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
        [command]="s.command"
        [handedBackTo]="handedBackTo(s)"
        [personLabel]="personLabel()"
        [hostLabel]="hostLabel"
        [code]="s.code"
        [expiresAt]="s.expiresAt"
        [refusalCode]="s.refusalCode"
        [refusalReason]="s.refusalReason"
        [trail]="s.trail"
        [standing]="s.standing"
        (approved)="onApprove($event)"
        (declined)="onDecline()"
        (signInAgain)="onSignInAgain()"
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
  /** Null until the person's sign-in is known; then the page, from `loading` on. */
  readonly state = signal<DeviceConsentPageState | null>(null);

  private controller: DeviceConsentController | null = null;

  async ngOnInit(): Promise<void> {
    if (this.authState.isLoading()) await this.authState.init();
    const signedIn = this.authState.isAuthenticated();
    if (signedIn) this.personLabel.set(this.authState.account()?.identifier ?? undefined);

    const controller = new DeviceConsentController({
      requestParam: this.route.snapshot.queryParamMap.get('request'),
      holder: { relation: 'your-doorway', label: this.hostLabel },
      client: this.port.client,
      signIn: () => this.sendToSignIn(),
      handBack: url => this.port.handBack(url),
      // This page's own link: the way through when this doorway takes no approvals.
      link: () => globalThis.location.href,
      onChange: state => this.state.set(state),
    });
    this.controller = controller;
    this.state.set(controller.state);
    // Signed out: ask first whether approvals are taken here at all, then sign in.
    await (signedIn ? controller.start() : controller.startBeforeSignIn());
  }

  /** Where the code went, for the card: held for the device, over the network, or here. */
  handedBackTo(state: DeviceConsentPageState): 'this-machine' | 'network' | 'held' {
    if (state.heldForDevice) return 'held';
    return state.handedBackOverNetwork ? 'network' : 'this-machine';
  }

  onApprove(approval: DeviceConsentApproval): void {
    void this.controller?.approve(approval);
  }

  /** The person chose to sign in again; they come straight back to this approval. */
  onSignInAgain(): void {
    this.controller?.signInAgain();
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
