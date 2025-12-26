import { NgModule, ModuleWithProviders, inject, provideAppInitializer } from '@angular/core';
import { CommonModule } from '@angular/common';

// Core services
import { ContentFormatRegistryService } from './services/content-format-registry.service';
import { ContentEditorService } from './services/content-editor.service';
import { ContentIOService } from './services/content-io.service';

// Components
import { ContentDownloadComponent } from './components/content-download/content-download.component';
import { DefaultCodeEditorComponent } from './components/default-code-editor/default-code-editor.component';

// Unified format plugins
import { MarkdownFormatPlugin } from './plugins/markdown/markdown-format.plugin';
import { GherkinFormatPlugin } from './plugins/gherkin/gherkin-format.plugin';
// Perseus enabled for build (npm run build passes)
// Note: Tests run with Perseus disabled (Karma needs separate SVG loader config)
import { PerseusFormatPlugin } from './plugins/perseus/perseus-format.plugin';

/**
 * Initializer function to register unified format plugins.
 * This ensures plugins are registered before any component tries to use them.
 */
function initializeFormatPlugins(): void {
  const registry = inject(ContentFormatRegistryService);
  // Register built-in unified plugins
  registry.register(new MarkdownFormatPlugin());
  registry.register(new GherkinFormatPlugin());
  registry.register(new PerseusFormatPlugin());
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
  imports: [
    CommonModule,
    ContentDownloadComponent,
    DefaultCodeEditorComponent
  ],
  exports: [
    ContentDownloadComponent,
    DefaultCodeEditorComponent
  ],
  providers: [
    ContentFormatRegistryService,
    ContentEditorService,
    ContentIOService
  ]
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
        provideAppInitializer(initializeFormatPlugins)
      ]
    };
  }
}

/**
 * ContentIO module with built-in plugins pre-loaded.
 */
@NgModule({
  imports: [
    ContentIOModule
  ],
  exports: [
    ContentIOModule
  ]
})
export class ContentIOModuleWithPlugins {}
