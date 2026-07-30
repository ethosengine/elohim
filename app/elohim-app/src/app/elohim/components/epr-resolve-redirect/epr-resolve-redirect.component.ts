/**
 * EprResolveRedirectComponent — Handle web+epr:// protocol redirects.
 *
 * When the browser receives a web+epr: link (from email, another app, etc.),
 * registerProtocolHandler redirects to /resolve?uri=web+epr:{epr-uri}.
 * This component strips the prefix and navigates to the appropriate route.
 *
 * This is the only part of EPR resolution that uses an actual browser primitive.
 * Everything else (context-aware resolution, popover inspection) is our polyfill.
 */

import { Component, OnInit, inject, ChangeDetectionStrategy } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';

import { parseEpr, eprToRoute, BUNDLE_ROUTE_CONTEXT } from '@elohim/service';

@Component({
  selector: 'app-epr-resolve-redirect',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.Eager,
  template: '<p>Resolving...</p>',
})
export class EprResolveRedirectComponent implements OnInit {
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly routeCtx = inject(BUNDLE_ROUTE_CONTEXT);

  ngOnInit(): void {
    const uri = this.route.snapshot.queryParamMap.get('uri') ?? '';

    // Strip web+epr: prefix if present (added by registerProtocolHandler)
    const cleaned = uri.replace(/^web\+epr:/, 'epr:');

    const ref = parseEpr(cleaned);
    const res = eprToRoute(ref, this.routeCtx);
    void this.router.navigate(res?.commands ?? ['/epr', ref.id]);
  }
}
