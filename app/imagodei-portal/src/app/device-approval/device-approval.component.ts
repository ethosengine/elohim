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
 */

import { ChangeDetectionStrategy, Component, OnInit, inject, signal } from '@angular/core';
import {
  DeviceConsentController,
  type DeviceConsentApproval,
  type DeviceConsentPageState,
} from 'elohim-imagodei/device-consent';

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
      (approved)="onApprove($event)"
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
    onChange: state => this.state.set(state),
  });

  readonly state = signal<DeviceConsentPageState>(this.controller.state);

  ngOnInit(): void {
    void this.controller.start();
  }

  onApprove(approval: DeviceConsentApproval): void {
    void this.controller.approve(approval);
  }

  onDecline(): void {
    this.controller.decline();
  }

  onExpired(): void {
    this.controller.expired();
  }
}
