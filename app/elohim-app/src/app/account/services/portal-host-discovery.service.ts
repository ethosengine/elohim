/**
 * Portal Host Discovery Service — derives hosting context from AccountView.
 *
 * A pure computed projection over AccountService. No HTTP calls — all data
 * flows from the already-loaded AccountView. UI components use this to decide
 * whether to show the steward badge, which portal host to prefer, etc.
 */

import { Injectable, inject, computed } from '@angular/core';

import { AccountService } from './account.service';

@Injectable({ providedIn: 'root' })
export class PortalHostDiscoveryService {
  private readonly accountService = inject(AccountService);

  /** True when this account holds a stewardship allocation on the local conductor. */
  readonly isSteward = computed(() => this.accountService.account()?.isSteward ?? false);

  /** True when elohim-storage is connected to a local Holochain conductor. */
  readonly hasLocalConductor = computed(
    () => this.accountService.account()?.hasLocalConductor ?? false
  );

  /** All portal hosts registered for this account. */
  readonly portalHosts = computed(() => this.accountService.account()?.portalHosts ?? []);

  /** The first registered portal host URL, or null if none. */
  readonly preferredHost = computed(() => this.portalHosts()[0]?.hostUrl ?? null);

  /** True when at least one portal host is registered. */
  readonly hasPortalHost = computed(() => this.portalHosts().length > 0);

  // ---------------------------------------------------------------------------
  // Reachability (canonical source: AccountView.portalHosts[].lastReachableAt)
  //
  // `lastReachableAt` is the operational enrichment libp2p writes onto each
  // PortalHostView when a /healthz probe last succeeded (see portal-host-view
  // schema). It is the conductor/storage projection's reachability verdict — the
  // single source of truth. We do NOT consult ExchangeSessionResponse.portalHostUrl
  // (a session-scoped hint) here; AccountView is canonical ("no third portal").
  // ---------------------------------------------------------------------------

  /**
   * The first portal host the storage projection has marked reachable, or null.
   * "Reachable" = a non-empty `lastReachableAt` timestamp.
   */
  readonly reachablePortalHost = computed(
    () => this.portalHosts().find(h => !!h.lastReachableAt) ?? null
  );

  /** True when at least one registered portal host is currently reachable. */
  readonly hasReachablePortalHost = computed(() => this.reachablePortalHost() !== null);

  /**
   * Display predicate for the "Manage from your steward →" redirect: the account
   * is a steward AND a reachable portal host exists. When false, the hosted
   * security view falls through unchanged.
   */
  readonly shouldOfferStewardRedirect = computed(
    () => this.isSteward() && this.hasReachablePortalHost()
  );
}
