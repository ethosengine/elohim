import { NgModule, ModuleWithProviders } from '@angular/core';
import { CommonModule } from '@angular/common';

// Core services
import { ContentIORegistryService } from './services/content-io-registry.service';
import { ContentIOService } from './services/content-io.service';

// Components
import { ContentDownloadComponent } from './components/content-download/content-download.component';

// Built-in plugins
import { MarkdownIOModule } from './plugins/markdown/markdown-io.module';
import { GherkinIOModule } from './plugins/gherkin/gherkin-io.module';

/**
 * Content I/O Module
 *
 * Provides content import/export functionality with a plugin architecture.
 *
 * Usage:
 * ```typescript
 * @NgModule({
 *   imports: [
 *     ContentIOModule.forRoot()  // Includes built-in plugins
 *   ]
 * })
 * export class AppModule {}
 * ```
 *
 * Or import specific plugins:
 * ```typescript
 * @NgModule({
 *   imports: [
 *     ContentIOModule,
 *     MarkdownIOModule,  // Only markdown support
 *   ]
 * })
 * ```
 */
@NgModule({
  imports: [
    CommonModule,
    ContentDownloadComponent
  ],
  exports: [
    ContentDownloadComponent
  ],
  providers: [
    ContentIORegistryService,
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
        ContentIORegistryService,
        ContentIOService
      ]
    };
  }
}

/**
 * ContentIO module with built-in plugins pre-loaded.
 */
@NgModule({
  imports: [
    ContentIOModule,
    MarkdownIOModule,
    GherkinIOModule
  ],
  exports: [
    ContentIOModule
  ]
})
export class ContentIOModuleWithPlugins {}
