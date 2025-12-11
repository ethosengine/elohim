import { NgModule } from '@angular/core';
import { MarkdownIOPlugin } from './markdown-io.plugin';
import { ContentIORegistryService } from '../../services/content-io-registry.service';

/**
 * Module for Markdown I/O plugin.
 *
 * When imported, automatically registers the Markdown plugin
 * with the ContentIORegistry.
 */
@NgModule({
  providers: [MarkdownIOPlugin]
})
export class MarkdownIOModule {
  constructor(
    private readonly registry: ContentIORegistryService,
    private readonly plugin: MarkdownIOPlugin
  ) {
    this.registry.register(this.plugin);
  }
}
