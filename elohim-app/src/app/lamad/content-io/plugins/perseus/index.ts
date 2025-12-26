/**
 * Perseus Quiz Plugin - Khan Academy Perseus integration for lamad.
 *
 * This module provides:
 * - Perseus item model and types
 * - React-to-Angular bridge via custom elements
 * - Content format plugin for content-io system
 * - Interactive quiz renderer with mastery tracking
 *
 * @example
 * ```typescript
 * import { PerseusFormatPlugin } from './plugins/perseus';
 *
 * // Register in content-io
 * registry.register(new PerseusFormatPlugin());
 * ```
 */

// Models
export * from './perseus-item.model';

// Components
export { PerseusWrapperComponent } from './perseus-wrapper.component';
export { PerseusRendererComponent } from './perseus-renderer.component';

// Custom Element Loader (loads external bundle)
export {
  registerPerseusElement,
  isPerseusElementRegistered,
  getPerseusElement
} from './perseus-element-loader';
export type { PerseusQuestionElement } from './perseus-element-loader';

// Plugin
export { PerseusFormatPlugin } from './perseus-format.plugin';
