import { NgModule } from '@angular/core';
import { GherkinIOPlugin } from './gherkin-io.plugin';
import { ContentIORegistryService } from '../../services/content-io-registry.service';

/**
 * Module for Gherkin I/O plugin.
 *
 * When imported, automatically registers the Gherkin plugin
 * with the ContentIORegistry.
 */
@NgModule({
  providers: [GherkinIOPlugin]
})
export class GherkinIOModule {
  constructor(
    private readonly registry: ContentIORegistryService,
    private readonly plugin: GherkinIOPlugin
  ) {
    this.registry.register(this.plugin);
  }
}
