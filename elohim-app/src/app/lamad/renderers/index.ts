/**
 * Renderer module barrel exports.
 *
 * The renderer system provides dynamic content rendering based on content format.
 * Components register with RendererRegistryService and are instantiated by
 * ContentViewerComponent via ViewContainerRef.createComponent().
 */

// Core services
export { RendererRegistryService } from './renderer-registry.service';
export { RendererInitializerService } from './renderer-initializer.service';

// Interfaces
export type {
  ContentRenderer,
  InteractiveRenderer,
  RendererCompletionEvent,
} from './renderer-registry.service';

// Built-in renderers
export { MarkdownRendererComponent } from './markdown-renderer/markdown-renderer.component';
export { QuizRendererComponent } from './quiz-renderer/quiz-renderer.component';
export { IframeRendererComponent } from './iframe-renderer/iframe-renderer.component';
export { GherkinRendererComponent } from './gherkin-renderer/gherkin-renderer.component';
