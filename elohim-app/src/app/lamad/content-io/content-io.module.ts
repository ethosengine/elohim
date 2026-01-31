import { CommonModule } from '@angular/common';
import { NgModule, ModuleWithProviders, inject, provideAppInitializer } from '@angular/core';

// @coverage: 23.1% (2026-02-04)

import { ContentDownloadComponent } from './components/content-download/content-download.component';
import { DefaultCodeEditorComponent } from './components/default-code-editor/default-code-editor.component';
import { GherkinFormatPlugin } from './plugins/gherkin/gherkin-format.plugin';
import { Html5AppFormatPlugin } from './plugins/html5-app/html5-app-format.plugin';
import { MarkdownFormatPlugin } from './plugins/markdown/markdown-format.plugin';
import { SophiaFormatPlugin } from './plugins/sophia/sophia-format.plugin';
import { ContentEditorService } from './services/content-editor.service';
import { ContentFormatRegistryService } from './services/content-format-registry.service';
import { ContentIOService } from './services/content-io.service';

/**
 * Canonical format ID for Sophia quiz content.
 * Used to map various quiz format aliases to the Sophia renderer.
 */
const SOPHIA_QUIZ_FORMAT = 'sophia-quiz-json';

/**
 * Initializer function to register unified format plugins.
 * This ensures plugins are registered before any component tries to use them.
 *
 * Architecture: Content formats describe the DATA SCHEMA (what the content IS),
 * while renderers describe HOW to display that data. Aliases map formats to renderers.
 */
function initializeFormatPlugins(): void {
  const registry = inject(ContentFormatRegistryService);

  // Register built-in unified plugins
  registry.register(new MarkdownFormatPlugin());
  registry.register(new GherkinFormatPlugin());
  registry.register(new Html5AppFormatPlugin());
  registry.register(new SophiaFormatPlugin());

  // Register format aliases: map data formats to their renderers
  // This keeps content storage format-agnostic while providing flexible rendering
  // Sophia handles both mastery quizzes (Perseus-compatible) and discovery assessments
  registry.registerAlias('sophia', SOPHIA_QUIZ_FORMAT); // Short alias → canonical
  registry.registerAlias('perseus', SOPHIA_QUIZ_FORMAT); // Legacy Perseus format → Sophia renderer
  registry.registerAlias('perseus-quiz-json', SOPHIA_QUIZ_FORMAT); // Perseus quiz data schema → Sophia renderer
  registry.registerAlias('sophia-discovery', SOPHIA_QUIZ_FORMAT); // Discovery assessments → Sophia renderer
}

/**
 * Content I/O Module
 *
 * Provides content import/export/editing functionality with a unified plugin architecture.
 *
 * Each ContentFormatPlugin provides:
 * - Rendering (how to display content)
 * - I/O operations (import/export/validate)
 * - Editing (optional custom editor component)
 *
 * Usage:
 * ```typescript
 * @NgModule({
 *   imports: [
 *     ContentIOModule.forRoot()  // Includes built-in plugins (Markdown, Gherkin)
 *   ]
 * })
 * export class AppModule {}
 * ```
 */
@NgModule({
  imports: [CommonModule, ContentDownloadComponent, DefaultCodeEditorComponent],
  exports: [ContentDownloadComponent, DefaultCodeEditorComponent],
  providers: [ContentFormatRegistryService, ContentEditorService, ContentIOService],
})
export class ContentIOModule {
  /**
   * Import with all built-in plugins (Markdown, Gherkin)
   */
  static forRoot(): ModuleWithProviders<ContentIOModule> {
    return {
      ngModule: ContentIOModuleWithPlugins,
      providers: [
        ContentFormatRegistryService,
        ContentEditorService,
        ContentIOService,
        // Register unified plugins at environment initialization
        provideAppInitializer(initializeFormatPlugins),
      ],
    };
  }
}

/**
 * ContentIO module with built-in plugins pre-loaded.
 */
@NgModule({
  imports: [ContentIOModule],
  exports: [ContentIOModule],
})
export class ContentIOModuleWithPlugins {}
