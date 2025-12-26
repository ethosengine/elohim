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

// Configuration - can be overridden via window globals before loading
const getPerseusPluginUrl = (): string => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  return (window as any)['__PERSEUS_PLUGIN_URL__'] as string
    || '/assets/perseus-plugin/perseus-plugin.umd.js';
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
      const pluginUrl = getPerseusPluginUrl();
      console.log('[Perseus] Loading plugin from:', pluginUrl);

      // Load the UMD bundle - it auto-registers the custom element
      await loadScript(pluginUrl);

      // Wait a moment for the element to register (async registration in bundle)
      await new Promise(resolve => setTimeout(resolve, 100));

      // Verify registration
      if (!customElements.get('perseus-question')) {
        // Try waiting a bit longer
        await new Promise(resolve => setTimeout(resolve, 500));

        if (!customElements.get('perseus-question')) {
          throw new Error('Perseus custom element not registered after bundle load');
        }
      }

      isRegistered = true;
      console.log('[Perseus] Plugin loaded successfully');
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
