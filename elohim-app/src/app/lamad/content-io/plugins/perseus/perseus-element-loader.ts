/**
 * Perseus Element Loader - Lazy loads React and Perseus.
 *
 * This module provides lazy loading of the Perseus custom element,
 * ensuring React and Perseus are only bundled/loaded when needed.
 *
 * The actual React component is in perseus-element.tsx which will
 * be dynamically imported when registerPerseusElement is called.
 */

import type { PerseusItem, PerseusScoreResult } from './perseus-item.model';

// Re-export types
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

/**
 * Lazily register the Perseus custom element.
 *
 * This function dynamically imports React and the Perseus element,
 * ensuring they're only loaded when actually needed.
 *
 * @returns Promise that resolves when the element is registered
 */
export async function registerPerseusElement(): Promise<void> {
  if (isRegistered) {
    return;
  }

  if (loadPromise) {
    return loadPromise;
  }

  loadPromise = (async () => {
    try {
      // Dynamically import the TSX module which contains React
      const module = await import('./perseus-element');
      module.registerPerseusElement();
      isRegistered = true;
    } catch (error) {
      console.error('[Perseus] Failed to load Perseus element:', error);
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
