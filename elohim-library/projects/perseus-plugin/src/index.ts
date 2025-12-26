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

// Export registration functions
export {
  registerPerseusElement,
  isPerseusElementRegistered
} from './perseus-element';

// Export element type
export type { PerseusQuestionElement } from './perseus-element';

// Auto-register when loaded as UMD in browser
if (typeof window !== 'undefined' && typeof customElements !== 'undefined') {
  // Use dynamic import to ensure the element is registered after the module loads
  import('./perseus-element').then(({ registerPerseusElement }) => {
    registerPerseusElement();
  }).catch((err) => {
    console.error('[Perseus Plugin] Failed to auto-register:', err);
  });
}
