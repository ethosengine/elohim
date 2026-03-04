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

import { Component, OnInit, inject } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';

import { parseEpr, eprToRoute } from '../../utils/epr-ref';

@Component({
  selector: 'app-epr-resolve-redirect',
  standalone: true,
  template: '<p>Resolving...</p>',
})
export class EprResolveRedirectComponent implements OnInit {
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);

  ngOnInit(): void {
    const uri = this.route.snapshot.queryParamMap.get('uri') ?? '';

    // Strip web+epr: prefix if present (added by registerProtocolHandler)
    const cleaned = uri.replace(/^web\+epr:/, 'epr:');

    const ref = parseEpr(cleaned);
    const appRoute = eprToRoute(ref) ?? ['/resource', ref.id];

    void this.router.navigate(appRoute);
  }
}
