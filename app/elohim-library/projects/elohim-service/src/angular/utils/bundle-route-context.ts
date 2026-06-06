import { InjectionToken } from '@angular/core';

import type { BundleRouteContext } from './epr-ref';

/**
 * Each bundle's composition root provides its route claims here (spec §12.3).
 * Default: nothing claimed, no universal route — every EPR target resolves
 * cross-bundle to /epr/{id}, which is always safe.
 *
 * Override at the bundle's composition-root providers (app.config), never
 * lazily — the root default is resolved once at first injection.
 */
export const BUNDLE_ROUTE_CONTEXT = new InjectionToken<BundleRouteContext>('BUNDLE_ROUTE_CONTEXT', {
  providedIn: 'root',
  factory: (): BundleRouteContext => ({ claims: [] }),
});
