import { Injectable, inject, signal } from '@angular/core';
import { ActivatedRoute, NavigationEnd, Router } from '@angular/router';

import { filter } from 'rxjs/operators';

/**
 * Watches Router events and surfaces whether the current route renders
 * protocol-content (data.protocolContent === true) and the active CID
 * (params.resourceId, or data.fallbackCid when the route has no param —
 * e.g. the root landing route).
 *
 * The ProtocolOmniComponent mounts in the app shell gated by these
 * signals so non-protocol routes (admin, settings, auth flows) don't
 * carry the chrome.
 */
@Injectable({ providedIn: 'root' })
export class ProtocolRouteContextService {
  private readonly router = inject(Router);

  private readonly _isProtocol = signal(false);
  private readonly _cid = signal<string | null>(null);

  readonly isProtocol = this._isProtocol.asReadonly();
  readonly cid = this._cid.asReadonly();

  constructor() {
    this.router.events
      .pipe(filter((e): e is NavigationEnd => e instanceof NavigationEnd))
      .subscribe(() => this.refresh());
    // Seed once at construction in case a route is already active.
    this.refresh();
  }

  private refresh(): void {
    let route: ActivatedRoute | null = this.router.routerState.root;
    while (route?.firstChild) route = route.firstChild;
    const data = route?.snapshot.data ?? {};
    const params = route?.snapshot.params ?? {};
    const isProtocol = data['protocolContent'] === true;
    const cid =
      (params['resourceId'] as string | undefined) ??
      (data['fallbackCid'] as string | undefined) ??
      null;
    this._isProtocol.set(isProtocol);
    this._cid.set(cid);
  }
}
