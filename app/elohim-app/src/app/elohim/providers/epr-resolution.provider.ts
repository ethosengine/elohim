/**
 * EprResolutionProvider — the browser wiring of the ambient resolution contract.
 *
 * The contract itself lives in `elohim-core` (framework-free, so Lit elements
 * import it without Angular). THIS file is the Angular/doorway implementation:
 * a thin adapter that maps the contract's three methods onto the existing
 * head-first services — the single seam where transport is chosen. A Tauri or
 * head-gossip implementation would plug in behind the same token.
 *
 *  - `resolveHead`  → `EprResolverService.resolvePreview` (anonymous-safe head plane)
 *  - `resolveRoute` → `eprToRoute` with the BUNDLE's route context baked in
 *  - `resolveBody`  → `EprResolverService.resolveBody` (reach-gated body plane)
 *
 * Provided once per bundle via {@link provideEprResolution} in the bundle's
 * `app.config`, so `resolveRoute` is claims-correct for that bundle without any
 * call site re-injecting the route context. Angular consumers inject
 * {@link EPR_RESOLUTION_PROVIDER}; Lit elements consume the same instance
 * ambiently through `eprResolutionContext` (installed at the shell root).
 *
 * Spec: genesis/docs/superpowers/specs/2026-07-02-epr-resolution-provider-design.md (I2)
 */

import { InjectionToken, type FactoryProvider } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import {
  parseEpr,
  eprToRoute,
  BUNDLE_ROUTE_CONTEXT,
  type BundleRouteContext,
} from '@elohim/service';

import { EprResolverService, type EprPreview } from '../services/epr-resolver.service';

import type {
  EprResolutionProvider,
  EprHeadPreview,
  EprHeadResolution,
  EprRouteResolution,
  EprBodyResolution,
} from 'elohim-core';

/**
 * DI token for the ambient EPR resolution provider. One binding per bundle
 * (shell / lamad), each baking that bundle's `BUNDLE_ROUTE_CONTEXT`.
 */
export const EPR_RESOLUTION_PROVIDER = new InjectionToken<EprResolutionProvider>(
  'EprResolutionProvider'
);

/**
 * Doorway/browser implementation. Delegates every transport decision to the
 * head-first `EprResolverService`; owns no fetch of its own. `resolveRoute` is
 * pure — it mints against the route context handed in at construction.
 */
class BrowserEprResolutionProvider implements EprResolutionProvider {
  constructor(
    private readonly resolver: EprResolverService,
    private readonly routeCtx: BundleRouteContext
  ) {}

  async resolveHead(ref: string): Promise<EprHeadResolution> {
    const outcome = await firstValueFrom(this.resolver.resolvePreview(ref));
    switch (outcome.state) {
      case 'resolved':
        return { state: 'resolved', head: previewToHead(outcome.preview) };
      case 'forbidden':
        // The head is anonymous-safe by design, so a `forbidden` head is rare —
        // but when it carries metadata, keep it so the element shows the title.
        return {
          state: 'forbidden',
          head: outcome.preview ? previewToHead(outcome.preview) : undefined,
        };
      case 'missing':
        return { state: 'missing' };
      default:
        return { state: 'error' };
    }
  }

  resolveRoute(ref: string, contentType?: string): EprRouteResolution | null {
    const res = eprToRoute(parseEpr(ref), this.routeCtx, contentType);
    // `null` = blob tier (no page address). Otherwise mint the in-bundle
    // commands (null when cross-bundle) alongside the always-navigable href.
    if (!res) return null;
    return { route: res.commands, href: res.href };
  }

  async resolveBody(ref: string): Promise<EprBodyResolution> {
    const outcome = await firstValueFrom(this.resolver.resolveBody(ref));
    return outcome.state === 'resolved'
      ? { state: 'resolved', body: outcome.contentBody, contentFormat: outcome.contentFormat }
      : { state: outcome.state };
  }
}

/** Project the service's head preview onto the framework-free contract shape. */
function previewToHead(preview: EprPreview): EprHeadPreview {
  return {
    id: preview.id,
    title: preview.title,
    description: preview.description,
    reach: preview.reach,
    contentType: preview.contentType,
    contentFormat: preview.contentFormat,
    tags: preview.tags,
    route: preview.route,
    href: preview.href,
  };
}

/**
 * Provide the ambient EPR resolution provider for a bundle. Add to each
 * `app.config` AFTER `BUNDLE_ROUTE_CONTEXT` is declared — the factory bakes that
 * bundle's route context in, so `resolveRoute` is claims-correct per bundle.
 */
export function provideEprResolution(): FactoryProvider {
  return {
    provide: EPR_RESOLUTION_PROVIDER,
    useFactory: (resolver: EprResolverService, routeCtx: BundleRouteContext) =>
      new BrowserEprResolutionProvider(resolver, routeCtx),
    deps: [EprResolverService, BUNDLE_ROUTE_CONTEXT],
  };
}
