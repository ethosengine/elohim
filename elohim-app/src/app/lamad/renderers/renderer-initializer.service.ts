import { Injectable } from '@angular/core';

// @coverage: 100.0% (2026-02-04)

import { SophiaRendererComponent } from '../content-io/plugins/sophia/sophia-renderer.component';

import { GherkinRendererComponent } from './gherkin-renderer/gherkin-renderer.component';
import { IframeRendererComponent } from './iframe-renderer/iframe-renderer.component';
import { MarkdownRendererComponent } from './markdown-renderer/markdown-renderer.component';
import { RendererRegistryService } from './renderer-registry.service';

/**
 * Legacy renderer initializer - registers renderers for the older RendererRegistryService.
 *
 * Registers both legacy and new format names for backwards compatibility:
 * - perseus-quiz-json, perseus: Legacy Perseus formats → Sophia renderer
 * - sophia, sophia-quiz-json: Native Sophia formats
 */
@Injectable({ providedIn: 'root' })
export class RendererInitializerService {
  constructor(private readonly registry: RendererRegistryService) {
    this.registerBuiltInRenderers();
  }

  private registerBuiltInRenderers(): void {
    this.registry.register(['markdown'], MarkdownRendererComponent, 10);
    this.registry.register(['html5-app', 'video-embed'], IframeRendererComponent, 10);
    this.registry.register(['gherkin'], GherkinRendererComponent, 5);
    // Sophia quiz renderer - handles all quiz/assessment formats (including legacy Perseus)
    this.registry.register(
      ['perseus-quiz-json', 'perseus', 'sophia', 'sophia-quiz-json'],
      SophiaRendererComponent,
      15
    );
  }
}
