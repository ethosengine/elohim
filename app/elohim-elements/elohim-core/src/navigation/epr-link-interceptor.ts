/**
 * EPR link interceptor — capture-phase document click listener that makes
 * cross-bundle anchors EPR-native.
 *
 * EPR-apps are separate SPA bundles dispatched by doorway's EprRouter. A
 * stale Angular routerLink targeting another bundle 404s on the local
 * router; a plain anchor full-reloads without recording navigation state.
 * This interceptor:
 *   - SAME-bundle anchors: always pass through untouched (Angular routerLink
 *     or default browser behavior own them).
 *   - CROSS-bundle anchors: preventDefault + stopImmediatePropagation
 *     (capture phase beats any routerLink target-phase handler that would
 *     404 on the local router), record a nav-handoff entry, then
 *     window.location.assign — the full load through doorway IS the
 *     projected EPR address.
 *
 * Fails open: any internal error falls back to default browser behavior.
 * Spec: 2026-06-05-omnibar-consolidation-epr-native-links-design.md §4.2.
 */

export interface EprLinkInterceptorOptions {
  /** Does THIS bundle's router own the given root-relative path? Default: base-href prefix heuristic. */
  ownsPath?: (path: string) => boolean;
  /** Called just before a cross-bundle navigation commits (write richer handoff state). Default: recordCrossBundleHandoff(). */
  beforeCrossBundle?: (target: string) => void;
  /**
   * Explicit installs (host-provided router-aware ownsPath, e.g. the Angular
   * shell) replace a default (page-chrome heuristic) install. Default
   * installs never replace an existing one.
   */
  explicit?: boolean;
  /** Test seam — defaults to window.location.assign. */
  assign?: (href: string) => void;
}

const NAV_STACK_KEY = 'elohim.session-nav-stack.v1';
const NAV_STACK_MAX = 32;

interface InstallRecord {
  uninstall: () => void;
  explicit: boolean;
}

declare global {
  interface Window {
    __elohimEprLinkInterceptor?: InstallRecord;
  }
}

/**
 * Append a handoff entry to the shared session-nav-stack — the same
 * sessionStorage shape elohim-app's SessionNavStackService reads, so the
 * protocol-omni back affordance survives the bundle boundary.
 */
export function recordCrossBundleHandoff(cid = ''): void {
  try {
    const raw = sessionStorage.getItem(NAV_STACK_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    const stack = Array.isArray(parsed) ? parsed : [];
    const entry = {
      url: location.pathname + location.search,
      cid,
      label: document.title,
      ts: Date.now(),
    };
    sessionStorage.setItem(NAV_STACK_KEY, JSON.stringify([...stack, entry].slice(-NAV_STACK_MAX)));
  } catch {
    // handoff is cosmetic — never block navigation
  }
}

/**
 * Default ownsPath: prefix match against this bundle's <base href>. A "/"
 * base owns everything — the shell (base "/") installs explicitly with a
 * router-aware predicate instead of relying on this heuristic.
 */
export function baseHrefOwnsPath(path: string): boolean {
  const base = new URL(document.baseURI).pathname;
  if (base === '/') return true;
  const trimmed = base.replace(/\/$/, '');
  return path === base || path === trimmed || path.startsWith(base);
}

function findAnchor(e: MouseEvent): HTMLAnchorElement | null {
  for (const t of e.composedPath()) {
    if (t instanceof HTMLAnchorElement) return t;
  }
  return null;
}

export function installEprLinkInterceptor(options: EprLinkInterceptorOptions = {}): () => void {
  if (typeof document === 'undefined') return () => undefined;

  const existing = window.__elohimEprLinkInterceptor;
  if (existing) {
    if (!options.explicit) return () => undefined; // default never disturbs the active install
    existing.uninstall();
  }

  const ownsPath = options.ownsPath ?? baseHrefOwnsPath;
  const assign = options.assign ?? ((href: string) => window.location.assign(href));

  const onClick = (e: MouseEvent): void => {
    try {
      if (e.defaultPrevented) return;
      if (e.button !== 0 || e.ctrlKey || e.metaKey || e.shiftKey || e.altKey) return;
      const a = findAnchor(e);
      if (!a) return;
      if (a.hasAttribute('download') || a.hasAttribute('data-epr-bypass')) return;
      const target = a.getAttribute('target');
      if (target && target !== '_self') return;
      const rawHref = a.getAttribute('href');
      if (!rawHref || rawHref.startsWith('#')) return;
      const url = new URL(a.href, document.baseURI);
      if (url.origin !== location.origin) return;
      if (ownsPath(url.pathname)) return; // same-bundle: routerLink/browser own it

      // Cross-bundle: beat any stale routerLink handler, record handoff, go.
      e.preventDefault();
      e.stopImmediatePropagation();
      const targetHref = url.pathname + url.search + url.hash;
      try {
        if (options.beforeCrossBundle) options.beforeCrossBundle(url.pathname + url.search);
        else recordCrossBundleHandoff();
      } catch {
        // handoff failure never blocks navigation
      }
      assign(targetHref);
    } catch {
      // Fail open: default browser behavior proceeds.
    }
  };

  document.addEventListener('click', onClick, true);
  const uninstall = (): void => {
    document.removeEventListener('click', onClick, true);
    if (window.__elohimEprLinkInterceptor?.uninstall === uninstall) {
      delete window.__elohimEprLinkInterceptor;
    }
  };
  window.__elohimEprLinkInterceptor = { uninstall, explicit: options.explicit ?? false };
  return uninstall;
}
