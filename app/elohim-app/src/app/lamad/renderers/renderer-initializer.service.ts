import { Injectable, Type, inject } from '@angular/core';

import { SophiaRendererComponent } from '../content-io/plugins/sophia/sophia-renderer.component';
import { LAMAD_RENDERER_MAP } from '../generated/manifest-types';

import { GherkinRendererComponent } from './gherkin-renderer/gherkin-renderer.component';
import { IframeRendererComponent } from './iframe-renderer/iframe-renderer.component';
import { MarkdownRendererComponent } from './markdown-renderer/markdown-renderer.component';
import { ContentRenderer, RendererRegistryService } from './renderer-registry.service';

/**
 * Maps manifest component name strings to actual Angular components.
 * To add a new renderer: add the format to the manifest + add the component here.
 */
const RENDERER_COMPONENTS: Record<string, Type<ContentRenderer>> = {
  MarkdownRendererComponent: MarkdownRendererComponent,
  GherkinRendererComponent: GherkinRendererComponent,
  SophiaRendererComponent: SophiaRendererComponent,
  IframeRendererComponent: IframeRendererComponent,
  // PathViewerComponent: PathViewerComponent, // TODO: wire when path renderer is built
};

/**
 * Manifest-driven renderer initializer.
 *
 * Reads LAMAD_RENDERER_MAP (generated from manifest.json) and registers
 * each format → component mapping. No hard-coded format lists.
 *
 * To register a new renderer:
 * 1. Add format + renderer declaration to app/lamad/manifest.json
 * 2. Add component to RENDERER_COMPONENTS above
 * 3. Run `pnpm run lamad:codegen`
 */
@Injectable({ providedIn: 'root' })
export class RendererInitializerService {
  private readonly registry = inject(RendererRegistryService);

  constructor() {
    this.registerFromManifest();
  }

  private registerFromManifest(): void {
    // Invert the map: component name → format list
    const componentFormats = new Map<string, string[]>();
    for (const [format, componentName] of Object.entries(LAMAD_RENDERER_MAP)) {
      const existing = componentFormats.get(componentName) ?? [];
      existing.push(format);
      componentFormats.set(componentName, existing);
    }

    // Register each component with its format list
    for (const [componentName, formats] of componentFormats) {
      const component = RENDERER_COMPONENTS[componentName];
      if (component) {
        this.registry.register(formats, component);
      }
    }
  }
}
