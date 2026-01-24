/**
 * Sophia Plugin - Content format plugin for Sophia assessments.
 *
 * This plugin provides unified mastery and discovery/reflection assessment
 * rendering using the Sophia fork of Perseus with psychometric support.
 */

// Models
export * from './sophia-moment.model';

// Element loader
export {
  registerSophiaElement,
  isSophiaElementRegistered,
  getSophiaElement,
  type SophiaQuestionElement
} from './sophia-element-loader';

// Components
export { SophiaWrapperComponent } from './sophia-wrapper.component';
export { SophiaRendererComponent } from './sophia-renderer.component';

// Plugin
export { SophiaFormatPlugin } from './sophia-format.plugin';
