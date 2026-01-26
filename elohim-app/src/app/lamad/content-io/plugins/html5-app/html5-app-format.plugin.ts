import { Injectable, Type } from '@angular/core';

import {
  IframeRendererComponent,
  Html5AppContent,
} from '../../../renderers/iframe-renderer/iframe-renderer.component';
import {
  ContentFormatPlugin,
  ContentRenderer,
  ContentEditorComponent,
  EditorConfig,
  DEFAULT_EDITOR_CONFIG,
} from '../../interfaces/content-format-plugin.interface';
import {
  ContentIOImportResult,
  ContentIOExportInput,
} from '../../interfaces/content-io-plugin.interface';
import { FormatMetadata } from '../../interfaces/format-metadata.interface';
import {
  ValidationResult,
  ValidationError,
  ValidationWarning,
} from '../../interfaces/validation-result.interface';

/**
 * Html5AppFormatPlugin - Plugin for HTML5 interactive applications.
 *
 * HTML5 apps are served from Doorway's /apps/ endpoint, which extracts
 * and caches zip files from the DHT. The Angular side is a thin iframe
 * wrapper that just sets the src URL.
 *
 * Content Structure:
 * ```typescript
 * {
 *   contentFormat: 'html5-app',
 *   content: {
 *     appId: 'evolution-of-trust',  // URL namespace
 *     entryPoint: 'index.html',      // File to load
 *     fallbackUrl?: 'https://...'    // External fallback
 *   }
 * }
 * ```
 *
 * @example
 * ```typescript
 * // Register in content-io module
 * registry.register(new Html5AppFormatPlugin());
 * ```
 */
@Injectable({
  providedIn: 'root',
})
export class Html5AppFormatPlugin implements ContentFormatPlugin {
  // ═══════════════════════════════════════════════════════════════════════════
  // Identity
  // ═══════════════════════════════════════════════════════════════════════════

  readonly formatId = 'html5-app';
  readonly displayName = 'HTML5 Application';
  readonly fileExtensions = ['.zip'];
  readonly mimeTypes = ['application/zip', 'application/x-zip-compressed'];

  // ═══════════════════════════════════════════════════════════════════════════
  // Capabilities
  // ═══════════════════════════════════════════════════════════════════════════

  readonly canImport = true;
  readonly canExport = true;
  readonly canValidate = true;
  readonly canRender = true;
  readonly canEdit = false; // HTML5 apps are not editable in-browser

  // ═══════════════════════════════════════════════════════════════════════════
  // Import
  // ═══════════════════════════════════════════════════════════════════════════

  async import(input: string | File): Promise<ContentIOImportResult> {
    // For HTML5 apps, import creates the content structure from a zip file
    // The actual zip is stored separately as a blob in the DHT

    if (typeof input === 'string') {
      // Parse JSON content structure
      try {
        const content = JSON.parse(input) as Html5AppContent;
        return this.createImportResult(content);
      } catch {
        throw new Error('Invalid HTML5 app JSON structure');
      }
    }

    // File upload - extract metadata from zip
    const filename = input.name.replace(/\.zip$/i, '');
    const appId = this.slugify(filename);

    return {
      content: {
        appId,
        entryPoint: 'index.html',
      } as Html5AppContent,
      contentFormat: 'html5-app',
      contentType: 'simulation',
      title: this.humanize(filename),
      description: `Interactive HTML5 application: ${this.humanize(filename)}`,
      tags: ['html5-app', 'interactive', 'simulation'],
      metadata: {
        originalFilename: input.name,
        sizeBytes: input.size,
        embedStrategy: 'iframe',
        requiredCapabilities: ['javascript'],
      },
    };
  }

  private createImportResult(content: Html5AppContent): ContentIOImportResult {
    return {
      content,
      contentFormat: 'html5-app',
      contentType: 'simulation',
      title: this.humanize(content.appId),
      description: `Interactive HTML5 application: ${this.humanize(content.appId)}`,
      tags: ['html5-app', 'interactive'],
      metadata: {
        embedStrategy: 'iframe',
        requiredCapabilities: ['javascript'],
      },
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Export
  // ═══════════════════════════════════════════════════════════════════════════

  async export(node: ContentIOExportInput): Promise<string> {
    // Export the content structure as JSON
    // The actual zip blob is handled separately by the blob service
    const content = node.content as Html5AppContent;

    return JSON.stringify(
      {
        appId: content.appId,
        entryPoint: content.entryPoint,
        fallbackUrl: content.fallbackUrl,
      },
      null,
      2
    );
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Validation
  // ═══════════════════════════════════════════════════════════════════════════

  async validate(input: string | File): Promise<ValidationResult> {
    const errors: ValidationError[] = [];
    const warnings: ValidationWarning[] = [];

    if (typeof input === 'string') {
      // Validate JSON structure
      let parsed: unknown;
      try {
        parsed = JSON.parse(input);
      } catch (e) {
        errors.push({
          code: 'INVALID_JSON',
          message: `Invalid JSON: ${e instanceof Error ? e.message : 'Parse error'}`,
          line: 1,
        });
        return { valid: false, errors, warnings };
      }

      const content = parsed as Partial<Html5AppContent>;

      if (!content.appId || typeof content.appId !== 'string') {
        errors.push({
          code: 'MISSING_APP_ID',
          message: 'Missing required field: appId (string)',
        });
      } else if (!/^[a-z0-9-]+$/.test(content.appId)) {
        errors.push({
          code: 'INVALID_APP_ID',
          message: 'appId must be lowercase alphanumeric with hyphens only',
        });
      }

      if (!content.entryPoint || typeof content.entryPoint !== 'string') {
        errors.push({
          code: 'MISSING_ENTRY_POINT',
          message: 'Missing required field: entryPoint (string)',
        });
      } else if (!content.entryPoint.endsWith('.html')) {
        warnings.push({
          code: 'ENTRY_POINT_NOT_HTML',
          message: 'entryPoint should typically be an HTML file',
          suggestion: 'Use index.html as the entry point',
        });
      }

      if (!content.fallbackUrl) {
        warnings.push({
          code: 'NO_FALLBACK',
          message: 'No fallbackUrl specified. App will fail if doorway is unavailable.',
          suggestion: 'Add fallbackUrl pointing to the original source',
        });
      }
    } else {
      // Validate zip file
      if (!input.name.endsWith('.zip')) {
        errors.push({
          code: 'NOT_ZIP',
          message: 'File must be a .zip archive',
        });
      }

      if (input.size > 100 * 1024 * 1024) {
        // 100MB limit
        warnings.push({
          code: 'LARGE_FILE',
          message: 'Zip file is larger than 100MB. Consider optimizing assets.',
          suggestion: 'Compress images, remove unused files',
        });
      }
    }

    return {
      valid: errors.length === 0,
      errors,
      warnings,
      stats: {
        formatId: this.formatId,
        isFile: input instanceof File,
      },
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Rendering
  // ═══════════════════════════════════════════════════════════════════════════

  getRendererComponent(): Type<ContentRenderer> {
    return IframeRendererComponent;
  }

  getRendererPriority(): number {
    return 15; // Medium priority - specific format
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Editing
  // ═══════════════════════════════════════════════════════════════════════════

  getEditorComponent(): Type<ContentEditorComponent> | null {
    return null; // HTML5 apps are not editable in-browser
  }

  getEditorConfig(): EditorConfig {
    return {
      ...DEFAULT_EDITOR_CONFIG,
      editorMode: 'code',
      supportsLivePreview: false,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Format Detection
  // ═══════════════════════════════════════════════════════════════════════════

  detectFormat(content: string): number | null {
    try {
      const parsed = JSON.parse(content);

      // Check for Html5AppContent structure
      if (typeof parsed.appId === 'string' && typeof parsed.entryPoint === 'string') {
        return 0.9; // High confidence
      }

      // Check for ContentNode with html5-app format
      if (parsed.contentFormat === 'html5-app') {
        return 0.95;
      }
    } catch {
      return null;
    }

    return null;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Metadata
  // ═══════════════════════════════════════════════════════════════════════════

  getFormatMetadata(): FormatMetadata {
    return {
      formatId: this.formatId,
      displayName: this.displayName,
      description:
        'Interactive HTML5 applications served from Doorway. Content is extracted from zip archives and cached for fast access.',
      fileExtensions: this.fileExtensions,
      mimeTypes: this.mimeTypes,
      icon: 'web',
      category: 'media', // HTML5 apps are interactive media content
      supportsRoundTrip: false, // Zip content isn't editable
      priority: 15,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Private Helpers
  // ═══════════════════════════════════════════════════════════════════════════

  private slugify(text: string): string {
    return text
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '');
  }

  private humanize(slug: string): string {
    return slug.replace(/-/g, ' ').replace(/\b\w/g, c => c.toUpperCase());
  }
}
