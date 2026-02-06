/**
 * @elohim/perseus-plugin
 *
 * Perseus quiz plugin for Elohim learning platform.
 * Provides Khan Academy's Perseus quiz renderer as a Web Component.
 *
 * Usage:
 * 1. Load the UMD bundle - the custom element auto-registers
 * 2. Use <perseus-question> element in your HTML
 * 3. Set the `item` property with Perseus JSON data
 * 4. Call `score()` to get the result
 *
 * @example
 * ```html
 * <script src="perseus-plugin.umd.js"></script>
 * <perseus-question id="quiz"></perseus-question>
 * <script>
 *   const quiz = document.getElementById('quiz');
 *   quiz.item = perseusItemData;
 *   quiz.onScore = (result) => console.log(result);
 * </script>
 * ```
 */

// Export all types
export * from './perseus-item.model';

// Import registration functions with alias to avoid rollup naming conflicts
import {
  registerPerseusElement as doRegister,
  isPerseusElementRegistered as checkRegistered
} from './perseus-element';

// Re-export for consumers
export const registerPerseusElement = doRegister;
export const isPerseusElementRegistered = checkRegistered;

// Export element type
export type { PerseusQuestionElement } from './perseus-element';

// Auto-register when loaded as UMD in browser
console.log('[Perseus Plugin] Index module loaded, checking environment...');
console.log('[Perseus Plugin] window:', typeof window);
console.log('[Perseus Plugin] customElements:', typeof customElements);
console.log('[Perseus Plugin] doRegister function:', typeof doRegister);

if (typeof window !== 'undefined' && typeof customElements !== 'undefined') {
  console.log('[Perseus Plugin] Environment OK, calling doRegister...');
  // Register synchronously - module is already loaded
  try {
    doRegister();
    console.log('[Perseus Plugin] doRegister completed');
  } catch (err) {
    console.error('[Perseus Plugin] Failed to auto-register:', err);
  }
} else {
  console.warn('[Perseus Plugin] Skipping auto-register - not in browser environment');
}
