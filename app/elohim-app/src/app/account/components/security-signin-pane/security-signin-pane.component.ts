import { CommonModule } from '@angular/common';
import { Component, inject, ChangeDetectionStrategy } from '@angular/core';

import { AccountService } from '../../services/account.service';
import { HandoffService } from '../../services/handoff.service';
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
  changeDetection: ChangeDetectionStrategy.Eager,
  styleUrl: './security-signin-pane.component.css',
})
export class SecuritySigninPaneComponent {
  readonly account = inject(AccountService);
  private readonly portalHosts = inject(PortalHostDiscoveryService);
  private readonly handoff = inject(HandoffService);

  /**
   * Display predicate for the "Manage from your steward →" redirect. True only
   * when the account is a steward AND the storage projection reports a reachable
   * portal host. Otherwise the hosted security view renders unchanged.
   */
  readonly showStewardRedirect = this.portalHosts.shouldOfferStewardRedirect;

  /**
   * Client-driven redirect to the peer-native portal host (the doorway does not
   * 302). Mints a single-use transfer code (GET /auth/session-token via
   * HandoffService) and carries THAT as `session_token` — the doorway JWT must
   * never ride a URL. The issuer origin rides along as `doorway_url` so the
   * portal's storage can redeem the code server-to-server via
   * GET {doorway_url}/auth/exchange-session. OAuth params already on the URL
   * are preserved so an in-flight authorize dance survives the hop.
   *
   * If the mint fails, the human simply stays on the hosted view — we never
   * fall back to placing the JWT on the URL.
   */
  async redirectToStewardPortal(): Promise<void> {
    const host = this.portalHosts.reachablePortalHost();
    if (!host) return;

    const minted = await this.handoff.mintHandoffToken();
    if (!minted) {
      console.warn('[security-signin-pane] handoff mint failed; staying on hosted view');
      return;
    }

    const target = new URL(host.hostUrl);

    // eslint-disable-next-line no-restricted-syntax -- SSR-safe: browser-only click handler, never SSR-rendered
    const current = new URL(globalThis.location.href);
    for (const key of OAUTH_PASSTHROUGH_PARAMS) {
      const value = current.searchParams.get(key);
      if (value !== null) target.searchParams.set(key, value);
    }

    target.searchParams.set('session_token', minted);
    // eslint-disable-next-line no-restricted-syntax -- SSR-safe: browser-only click handler, never SSR-rendered
    target.searchParams.set('doorway_url', globalThis.location.origin);

    // eslint-disable-next-line no-restricted-syntax -- SSR-safe: browser-only click handler, never SSR-rendered
    globalThis.location.href = target.toString();
  }
}
