/**
 * Perseus Element Loader - Loads Perseus plugin from external bundle.
 *
 * This module provides lazy loading of the Perseus custom element from
 * an external UMD bundle, ensuring React and Perseus are only loaded when needed.
 *
 * The actual Perseus React component is in the @elohim/perseus-plugin package,
 * which is built separately and loaded at runtime via script tag.
 */

import type { PerseusItem, PerseusScoreResult } from './perseus-item.model';

// Re-export types for consumers
export type { PerseusItem, PerseusScoreResult };

/**
 * Interface for the Perseus custom element.
 * Used by Angular wrapper without needing React types.
 */
export interface PerseusQuestionElement extends HTMLElement {
  item: PerseusItem | null;
  reviewMode: boolean;
  onScore: ((result: PerseusScoreResult) => void) | null;
  onAnswerChange: ((hasAnswer: boolean) => void) | null;
  score(): PerseusScoreResult | null;
  focusInput(): void;
  getState(): unknown;
  restoreState(state: unknown): void;
}

// Track loading state
let loadPromise: Promise<void> | null = null;
let isRegistered = false;
let cssLoaded = false;

// Configuration for CSS URLs
const getPerseusStylesUrl = (): string => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  return (window as any)['__PERSEUS_STYLES_URL__'] as string
    || '/assets/perseus-plugin/perseus.css';
};

const getPerseusThemeOverridesUrl = (): string => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  return (window as any)['__PERSEUS_THEME_OVERRIDES_URL__'] as string
    || '/assets/perseus-plugin/perseus-theme-overrides.css';
};

/**
 * Load Perseus CSS stylesheets lazily.
 * Loads both the base Perseus styles and theme overrides.
 * Only called when first quiz is rendered.
 */
function loadPerseusCSS(): void {
  if (cssLoaded) return;

  // Load base Perseus styles
  const baseStylesId = 'perseus-styles';
  if (!document.getElementById(baseStylesId)) {
    const baseLink = document.createElement('link');
    baseLink.id = baseStylesId;
    baseLink.rel = 'stylesheet';
    baseLink.href = getPerseusStylesUrl();
    document.head.appendChild(baseLink);
    console.log('[Perseus] Base CSS loaded from:', baseLink.href);
  }

  // Load theme overrides (must come after base styles for proper cascade)
  const themeOverridesId = 'perseus-theme-overrides';
  if (!document.getElementById(themeOverridesId)) {
    const themeLink = document.createElement('link');
    themeLink.id = themeOverridesId;
    themeLink.rel = 'stylesheet';
    themeLink.href = getPerseusThemeOverridesUrl();
    document.head.appendChild(themeLink);
    console.log('[Perseus] Theme overrides CSS loaded from:', themeLink.href);
  }

  cssLoaded = true;
}

// Configuration - can be overridden via window globals before loading
// Cache-bust once per page load, not per call
const CACHE_BUST = Date.now();
const getPerseusPluginUrl = (): string => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const customUrl = (window as any)['__PERSEUS_PLUGIN_URL__'] as string;
  if (customUrl) return customUrl;
  return `/assets/perseus-plugin/perseus-plugin.umd.js?v=${CACHE_BUST}`;
};

/**
 * Load an external script by URL.
 */
function loadScript(url: string): Promise<void> {
  return new Promise((resolve, reject) => {
    // Check if already loaded
    if (document.querySelector(`script[src="${url}"]`)) {
      resolve();
      return;
    }

    const script = document.createElement('script');
    script.src = url;
    script.async = true;
    script.onload = () => resolve();
    script.onerror = () => reject(new Error(`Failed to load script: ${url}`));
    document.head.appendChild(script);
  });
}

/**
 * Load React from CDN if not already available.
 */
async function ensureReactLoaded(): Promise<void> {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const win = window as any;

  if (win.React && win.ReactDOM) {
    console.log('[Perseus] React already loaded');
    return;
  }

  console.log('[Perseus] Loading React from CDN...');

  // Load React first, then ReactDOM
  await loadScript('https://unpkg.com/react@18/umd/react.production.min.js');
  await loadScript('https://unpkg.com/react-dom@18/umd/react-dom.production.min.js');

  if (!win.React || !win.ReactDOM) {
    throw new Error('Failed to load React from CDN');
  }

  // Verify createRoot is available (React 18)
  console.log('[Perseus] React loaded from CDN');
  console.log('[Perseus] React version:', win.React.version);
  console.log('[Perseus] ReactDOM version:', win.ReactDOM.version);
  console.log('[Perseus] ReactDOM.createRoot:', typeof win.ReactDOM.createRoot);
}

/**
 * Lazily register the Perseus custom element by loading the external bundle.
 *
 * This function loads the Perseus plugin UMD bundle from /assets/perseus-plugin/
 * which contains React and all Khan Academy dependencies.
 *
 * @returns Promise that resolves when the element is registered
 */
export async function registerPerseusElement(): Promise<void> {
  // Already registered
  if (isRegistered || customElements.get('perseus-question')) {
    isRegistered = true;
    return;
  }

  // Loading in progress
  if (loadPromise) {
    return loadPromise;
  }

  loadPromise = (async () => {
    try {
      // Load Perseus CSS (lazy, only on first quiz render)
      loadPerseusCSS();

      // Load React from CDN first (Perseus bundle expects it as global)
      await ensureReactLoaded();

      const pluginUrl = getPerseusPluginUrl();
      console.log('[Perseus] Loading plugin from:', pluginUrl);

      // Double-check globals are set right before loading bundle
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const g = globalThis as any;
      console.log('[Perseus] Pre-load check - globalThis.React:', typeof g.React, g.React?.version);
      console.log('[Perseus] Pre-load check - globalThis.ReactDOM:', typeof g.ReactDOM, g.ReactDOM?.version);

      // Load the UMD bundle - it auto-registers the custom element synchronously
      await loadScript(pluginUrl);
      console.log('[Perseus] Script loaded');

      // Element should be registered synchronously during script execution
      const elementDef = customElements.get('perseus-question');
      console.log('[Perseus] Element registered:', !!elementDef);

      if (!elementDef) {
        throw new Error('Perseus custom element not registered after bundle load');
      }

      isRegistered = true;
      console.log('[Perseus] Plugin loaded successfully');

      // Dark mode disabled temporarily for debugging
      // setTimeout(() => {
      //   try {
      //     initPerseusDarkMode();
      //   } catch (e) {
      //     console.error('[Perseus] Dark mode init error:', e);
      //   }
      // }, 0);
      console.log('[Perseus] Dark mode initialization skipped (debugging)');
    } catch (error) {
      console.error('[Perseus] Failed to load plugin:', error);
      loadPromise = null;
      throw error;
    }
  })();

  return loadPromise;
}

/**
 * Check if the Perseus element is registered.
 */
export function isPerseusElementRegistered(): boolean {
  return isRegistered || !!customElements.get('perseus-question');
}

/**
 * Get the Perseus element by querying the DOM.
 * Type-safe helper for Angular components.
 */
export function getPerseusElement(container: HTMLElement): PerseusQuestionElement | null {
  return container.querySelector('perseus-question') as PerseusQuestionElement | null;
}

// ─────────────────────────────────────────────────────────────────────────────
// Dark Mode System (CSS-based)
// ─────────────────────────────────────────────────────────────────────────────
// Dark mode theming is now handled entirely via CSS in:
// /assets/perseus-plugin/perseus-theme-overrides.css
//
// This file uses CSS custom properties and high-specificity selectors to
// override Aphrodite's dynamically-generated styles. No JavaScript-based
// inline style application is needed.
//
// The CSS handles:
// - System dark mode preference via @media (prefers-color-scheme: dark)
// - Manual theme selection via body[data-theme="dark"]
// - Wonder Blocks CSS variable mappings
// - Aphrodite override with !important selectors
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Initialize Perseus dark mode support.
 *
 * @deprecated Dark mode is now handled entirely via CSS. This function is
 * kept for backwards compatibility but does nothing. The CSS file
 * perseus-theme-overrides.css handles all theming automatically.
 */
export function initPerseusDarkMode(): void {
  console.log('[Perseus] Dark mode handled via CSS (perseus-theme-overrides.css)');
}

/**
 * Refresh dark mode styles on Perseus elements.
 *
 * @deprecated Dark mode is now handled entirely via CSS. This function is
 * kept for backwards compatibility but does nothing.
 */
export function refreshPerseusDarkMode(): void {
  // CSS handles theming automatically - no JS refresh needed
}

/**
 * Clean up dark mode resources.
 *
 * @deprecated Dark mode is now handled entirely via CSS. This function is
 * kept for backwards compatibility but does nothing.
 */
export function destroyPerseusDarkMode(): void {
  // No resources to clean up - CSS handles everything
}
