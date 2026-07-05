import { Injectable, inject } from '@angular/core';
import { Router } from '@angular/router';
import type { Route } from '@angular/router';

import { ProtocolRouteContextService } from './protocol-route-context.service';
import { SessionNavStackService } from './session-nav-stack.service';

/**
 * EPR-aware navigation: same-bundle paths go through the Angular router;
 * cross-bundle paths get a nav-handoff record then a full doorway load
 * (the URL IS the projected EPR address — spec §4).
 *
 * ownsPath derives from the LIVE router config, so when a pillar splits
 * into its own bundle (as lamad did) the same call sites flip to
 * cross-bundle automatically — no hand-maintained route list.
 */
@Injectable({ providedIn: 'root' })
export class EprNavService {
  private readonly router = inject(Router);
  private readonly navStack = inject(SessionNavStackService);
  private readonly routeCtx = inject(ProtocolRouteContextService);

  /** Test seam — defaults to a full browser navigation. */
  // eslint-disable-next-line no-restricted-syntax -- field-initializer arrow body, not evaluated until browser call (assign() is only invoked from navigate()'s cross-bundle branch, never during SSR bootstrap)
  assign: (href: string) => void = href => globalThis.location.assign(href);

  ownsPath(path: string): boolean {
    const top = path.replace(/^\//, '').split(/[/?#]/)[0] ?? '';
    if (top === '') return true; // root landing is bundle-owned
    const matches = (routes: readonly Route[] | undefined): boolean =>
      !!routes?.some(r => {
        // Pathless layout roots (pillar-bundle shape) — descend into children.
        // Note: loadChildren (lazy) roots have r.children === undefined here;
        // they fall through to false (safe: ownsPath → false → full-load handoff).
        if (r.path === '' && r.children) return matches(r.children);
        if (!r.path || r.path === '**') return false;
        return r.path.split('/')[0] === top;
      });
    return matches(this.router.config);
  }

  navigate(pathOrCommands: string | readonly unknown[]): void {
    const url = Array.isArray(pathOrCommands)
      ? this.router.createUrlTree(pathOrCommands as never[]).toString()
      : (pathOrCommands as string);
    // Defense-in-depth: this sink can location.assign — only origin-relative
    // paths are navigable (no javascript:, data:, protocol-relative //, or
    // absolute-origin URLs through this seam).
    if (!url.startsWith('/') || url.startsWith('//')) {
      console.warn('[EprNavService] refused non-origin-relative navigation target:', url);
      return;
    }
    if (this.ownsPath(url)) {
      void this.router.navigateByUrl(url);
      return;
    }
    this.recordHandoff();
    this.assign(url);
  }

  /** Write the cross-bundle handoff entry (back affordance survives the boundary). */
  recordHandoff(): void {
    this.navStack.record({
      url: this.router.url,
      cid: this.routeCtx.cid() ?? '',
      // eslint-disable-next-line no-restricted-syntax -- browser-only cross-bundle navigation handoff; recordHandoff() is only invoked from user-triggered link interception/navigate(), never during SSR bootstrap
      label: document.title,
    });
  }
}
