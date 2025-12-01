import { Injectable } from '@angular/core';
import { RendererRegistryService } from './renderer-registry.service';
import { MarkdownRendererComponent } from './markdown-renderer/markdown-renderer.component';
import { IframeRendererComponent } from './iframe-renderer/iframe-renderer.component';
import { QuizRendererComponent } from './quiz-renderer/quiz-renderer.component';
import { GherkinRendererComponent } from './gherkin-renderer/gherkin-renderer.component';

@Injectable({ providedIn: 'root' })
export class RendererInitializerService {
  constructor(private readonly registry: RendererRegistryService) {
    this.registerBuiltInRenderers();
  }

  private registerBuiltInRenderers(): void {
    this.registry.register(['markdown'], MarkdownRendererComponent, 10);
    this.registry.register(['html5-app', 'video-embed'], IframeRendererComponent, 10);
    this.registry.register(['quiz-json'], QuizRendererComponent, 10);
    this.registry.register(['gherkin'], GherkinRendererComponent, 5);
  }
}
