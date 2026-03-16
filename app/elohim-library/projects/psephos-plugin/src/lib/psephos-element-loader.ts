/**
 * Psephos Element Loader - Loads Psephos ballot plugin from external bundle.
 *
 * This module provides lazy loading of the Psephos custom element from
 * an external UMD bundle, ensuring React and Psephos are only loaded when needed.
 *
 * Mirrors the sophia-element-loader.ts pattern from:
 * app/elohim-app/src/app/lamad/content-io/plugins/sophia/sophia-element-loader.ts
 */

/**
 * Interface for the Psephos ballot custom element.
 */
export interface PsephosBallotElement extends HTMLElement {
  ballot: unknown | null;
  reviewMode: boolean;
  onRecognition: ((result: unknown) => void) | null;
  onAnswerChange: ((hasAnswer: boolean) => void) | null;
  getRecognition(): unknown | null;
}

/** Typed accessor for runtime globals. */
interface PsephosGlobals {
  __PSEPHOS_STYLES_URL__?: string;
  __PSEPHOS_PLUGIN_URL__?: string;
  __REACT_URL__?: string;
  __REACT_DOM_URL__?: string;
  React?: unknown;
  ReactDOM?: unknown;
}

const _globals = globalThis as unknown as PsephosGlobals;

// Track loading state
let loadPromise: Promise<void> | null = null;
let isRegistered = false;
let cssLoaded = false;

/** Custom element tag name for Psephos ballots */
const PSEPHOS_ELEMENT_TAG = 'psephos-ballot';

// CSS URL
const getPsephosStylesUrl = (): string => {
  return _globals.__PSEPHOS_STYLES_URL__ ?? '/assets/psephos-plugin/psephos-element.css';
};

/**
 * Load Psephos CSS stylesheet lazily.
 */
function loadPsephosCSS(): void {
  if (cssLoaded) return;

  const stylesId = 'psephos-styles';
  if (!document.getElementById(stylesId)) {
    const link = document.createElement('link');
    link.id = stylesId;
    link.rel = 'stylesheet';
    link.href = getPsephosStylesUrl();
    document.head.appendChild(link);
  }

  cssLoaded = true;
}

// Cache-bust once per page load
const CACHE_BUST = Date.now();
const getPsephosPluginUrl = (): string => {
  const customUrl = _globals.__PSEPHOS_PLUGIN_URL__;
  if (customUrl) return customUrl;
  return `/assets/psephos-plugin/psephos-element.umd.js?v=${CACHE_BUST}`;
};

/**
 * Load an external script by URL.
 */
async function loadScript(url: string): Promise<void> {
  return new Promise((resolve, reject) => {
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

// React URLs - local assets for offline support, CDN fallback for development
const getReactUrl = (): string => {
  return _globals.__REACT_URL__ ?? '/assets/react/react.production.min.js';
};

const getReactDomUrl = (): string => {
  return _globals.__REACT_DOM_URL__ ?? '/assets/react/react-dom.production.min.js';
};

// CDN fallback URLs
const REACT_CDN_URL = 'https://unpkg.com/react@18/umd/react.production.min.js';
const REACT_DOM_CDN_URL = 'https://unpkg.com/react-dom@18/umd/react-dom.production.min.js';

/**
 * Load React from local assets, falling back to CDN if local fails.
 * Shared with sophia-element — checks if window.React exists first.
 */
async function ensureReactLoaded(): Promise<void> {
  if (_globals.React && _globals.ReactDOM) {
    return;
  }

  // Try local assets first (for offline support)
  const localReactUrl = getReactUrl();
  const localReactDomUrl = getReactDomUrl();

  try {
    await loadScript(localReactUrl);
    await loadScript(localReactDomUrl);

    if (_globals.React && _globals.ReactDOM) {
      return;
    }
  } catch {
    // Local React load failed - will fallback to CDN
  }

  // Fallback to CDN
  await loadScript(REACT_CDN_URL);
  await loadScript(REACT_DOM_CDN_URL);

  if (!_globals.React || !_globals.ReactDOM) {
    throw new Error('Failed to load React from both local assets and CDN');
  }
}

/**
 * Lazily register the Psephos custom element by loading the external bundle.
 */
export async function registerPsephosElement(): Promise<void> {
  // Already registered
  if (isRegistered || customElements.get(PSEPHOS_ELEMENT_TAG)) {
    isRegistered = true;
    return;
  }

  // Loading in progress
  if (loadPromise) {
    return loadPromise;
  }

  loadPromise = (async () => {
    try {
      // Load CSS
      loadPsephosCSS();

      // Load React first (shared with sophia-element)
      await ensureReactLoaded();

      const pluginUrl = getPsephosPluginUrl();

      // Load the UMD bundle
      await loadScript(pluginUrl);

      // Element should be registered synchronously
      const elementDef = customElements.get(PSEPHOS_ELEMENT_TAG);

      if (!elementDef) {
        throw new Error('Psephos custom element not registered after bundle load');
      }

      isRegistered = true;
    } catch (error) {
      // Psephos element registration failed - re-throw to caller
      loadPromise = null;
      throw error;
    }
  })();

  return loadPromise;
}

/**
 * Check if the Psephos element is registered.
 */
export function isPsephosElementRegistered(): boolean {
  return isRegistered || !!customElements.get(PSEPHOS_ELEMENT_TAG);
}

/**
 * Get the Psephos ballot element by querying the DOM.
 */
export function getPsephosElement(container: HTMLElement): PsephosBallotElement | null {
  return container.querySelector(PSEPHOS_ELEMENT_TAG);
}
