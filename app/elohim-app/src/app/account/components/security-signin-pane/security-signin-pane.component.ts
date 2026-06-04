import { CommonModule } from '@angular/common';
import { Component, inject } from '@angular/core';

import { AuthService } from '@app/imagodei';

import { AccountService } from '../../services/account.service';
import { PortalHostDiscoveryService } from '../../services/portal-host-discovery.service';

import { KeyListComponent } from './key-list/key-list.component';
import { LostKeyEntryComponent } from './lost-key-entry/lost-key-entry.component';
import { SelfRevokeComponent } from './self-revoke/self-revoke.component';
import { VoteAsEcComponent } from './vote-as-ec/vote-as-ec.component';

/** OAuth params carried through from the current URL onto the portal-host redirect. */
const OAUTH_PASSTHROUGH_PARAMS = ['client_id', 'redirect_uri', 'response_type', 'state'] as const;

@Component({
  selector: 'app-security-signin-pane',
  standalone: true,
  imports: [
    CommonModule,
    KeyListComponent,
    SelfRevokeComponent,
    VoteAsEcComponent,
    LostKeyEntryComponent,
  ],
  templateUrl: './security-signin-pane.component.html',
  styleUrl: './security-signin-pane.component.css',
})
export class SecuritySigninPaneComponent {
  readonly account = inject(AccountService);
  private readonly portalHosts = inject(PortalHostDiscoveryService);
  private readonly auth = inject(AuthService);

  /**
   * Display predicate for the "Manage from your steward →" redirect. True only
   * when the account is a steward AND the storage projection reports a reachable
   * portal host. Otherwise the hosted security view renders unchanged.
   */
  readonly showStewardRedirect = this.portalHosts.shouldOfferStewardRedirect;

  /**
   * Client-driven redirect to the peer-native portal host (the doorway does not
   * 302). Carries the current session token as `session_token` and preserves any
   * OAuth params already on the URL so an in-flight authorize dance survives the
   * hop to the steward's portal.
   */
  redirectToStewardPortal(): void {
    const host = this.portalHosts.reachablePortalHost();
    if (!host) return;

    const target = new URL(host.hostUrl);

    const current = new URL(globalThis.location.href);
    for (const key of OAUTH_PASSTHROUGH_PARAMS) {
      const value = current.searchParams.get(key);
      if (value !== null) target.searchParams.set(key, value);
    }

    const token = this.auth.token();
    if (token) target.searchParams.set('session_token', token);

    globalThis.location.href = target.toString();
  }
}
