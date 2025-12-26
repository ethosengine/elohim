import { Injectable } from '@angular/core';
import { RendererRegistryService } from './renderer-registry.service';
import { MarkdownRendererComponent } from './markdown-renderer/markdown-renderer.component';
import { IframeRendererComponent } from './iframe-renderer/iframe-renderer.component';
import { GherkinRendererComponent } from './gherkin-renderer/gherkin-renderer.component';
import { PerseusRendererComponent } from '../content-io/plugins/perseus/perseus-renderer.component';

/**
 * Legacy renderer initializer - registers renderers for the older RendererRegistryService.
 *
 * Registers both legacy and new format names for backwards compatibility:
 * - perseus-quiz-json: New unified format for all quiz/assessment content
 * - perseus: Plugin format ID for direct renderer access
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
    // Perseus quiz renderer - handles perseus-quiz-json format
    this.registry.register(['perseus-quiz-json', 'perseus'], PerseusRendererComponent, 15);
  }
}
