import { InjectionToken } from '@angular/core';

/** Resolves true once the identity elements are registered, false if they could not load. */
export type IdentityElementsLoader = () => Promise<boolean>;

let loading: Promise<boolean> | null = null;

/** Load (once) the identity elements bundle. A failed load can be retried. */
export function loadIdentityElements(): Promise<boolean> {
  loading ??= import('./register-identity-elements').then(
    () => true,
    () => {
      loading = null;
      return false;
    }
  );
  return loading;
}

/**
 * How a page asks for the identity elements on demand. Injected so specs can
 * answer "loaded", "not yet" or "never" without touching Lit.
 */
export const IDENTITY_ELEMENTS = new InjectionToken<IdentityElementsLoader>('IDENTITY_ELEMENTS', {
  providedIn: 'root',
  factory: () => loadIdentityElements,
});
