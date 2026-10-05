/**
 * Approve a device — `<portal base>consent/device?request=<base64url GrantRequest>`.
 *
 * The native portal's mount of the shared approval page
 * (`elohim-imagodei/device-consent`, which the doorway portal mounts too).
 * Here the node serving the portal is the person's own: it holds their key,
 * and the approval rests on it. The page asks it what the device wants; if no
 * one is signed in, it sends the person to sign in and straight back.
 *
 * This component only supplies what is this host's own — the key holder is
 * "this device", the cookie client, the way to sign in and come back. Nothing
 * here asks any other server anything.
 *
 * Beside the approval it reads what the person's identity rests on. If this
 * node has none yet, the person begins it in place and comes back to the
 * approval, with nothing sent twice.
 */

import {
  ChangeDetectionStrategy,
  Component,
  EventEmitter,
  OnInit,
  Output,
  inject,
  signal,
} from '@angular/core';
import {
  DeviceConsentController,
  NODE_CODE,
  type DeviceConsentApproval,
  type DeviceConsentPageState,
} from 'elohim-imagodei/device-consent';
import {
  IdentityStandingController,
  type IdentityPageState,
} from 'elohim-imagodei/identity-standing';

import { DEVICE_APPROVAL_PORT } from './device-approval-port.js';
import { DeviceApprovalViewComponent } from './device-approval-view.component.js';

@Component({
  selector: 'imagodei-portal-device-approval',
  standalone: true,
  imports: [DeviceApprovalViewComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <imagodei-portal-device-approval-view
      [state]="state()"
      [identity]="identity()"
      [link]="link"
      (approved)="onApprove($event)"
      (beginIdentity)="onBegin($event)"
      (signInAgain)="onSignInAgain()"
      (declined)="onDecline()"
      (expired)="onExpired()"
    />
  `,
})
export class DeviceApprovalComponent implements OnInit {
  private readonly port = inject(DEVICE_APPROVAL_PORT);
  private readonly controller = new DeviceConsentController({
    requestParam: new URLSearchParams(globalThis.location.search).get('request'),
    holder: { relation: 'this-device' },
    client: this.port.client,
    signIn: () => this.port.signIn(),
    handBack: url => this.port.handBack(url),
    onChange: state => {
      this.state.set(state);
      // The node says it holds no identity for the person: offer to begin here.
      if (state.phase === 'refused' && state.refusalCode === NODE_CODE.identityUnbootstrapped) {
        this.identityController.offerBegin();
      }
    },
  });
  private readonly identityController = new IdentityStandingController({
    client: this.port.identity,
    onChange: state => {
      this.identity.set(state);
      this.identityChange.emit(state);
    },
  });

  /** What the identity rests on, for the page's header (the root's shell). */
  @Output() readonly identityChange = new EventEmitter<IdentityPageState>();

  readonly state = signal<DeviceConsentPageState>(this.controller.state);
  readonly identity = signal<IdentityPageState>(this.identityController.state);
  /** This page's own link, shown as a terminal command if it is open on another machine. */
  readonly link = globalThis.location.href;

  ngOnInit(): void {
    void this.controller.start();
    void this.identityController.read();
  }

  /**
   * Begin the identity in place, then return to the approval. Nothing was
   * signed before, and nothing is sent again until the person approves.
   */
  async onBegin({
    displayName,
    secret,
    identifier,
  }: {
    displayName: string;
    secret?: string;
    identifier?: string;
  }): Promise<void> {
    if (await this.identityController.begin(displayName, secret, identifier)) {
      await this.controller.resume();
    }
  }

  onApprove(approval: DeviceConsentApproval): void {
    void this.controller.approve(approval);
  }

  /** The person chose to sign in again; they come straight back to this approval. */
  onSignInAgain(): void {
    this.controller.signInAgain();
  }

  onDecline(): void {
    this.controller.decline();
  }

  onExpired(): void {
    this.controller.expired();
  }
}
