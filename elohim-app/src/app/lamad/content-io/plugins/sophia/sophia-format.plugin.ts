/**
 * Sophia Format Plugin - Content format plugin for Sophia assessment.
 *
 * Integrates Sophia/Perseus-based assessment rendering with the content-io
 * plugin system. Supports both mastery (graded) and reflection (discovery)
 * assessment modes.
 */

import { Type } from '@angular/core';
import {
  BaseContentFormatPlugin,
  ContentRenderer,
  DEFAULT_EDITOR_CONFIG,
  type EditorConfig
} from '../../interfaces/content-format-plugin.interface';
import type {
  ContentIOImportResult,
  ContentIOExportInput
} from '../../interfaces/content-io-plugin.interface';
import type { ValidationResult } from '../../interfaces/validation-result.interface';
import type { FormatMetadata } from '../../interfaces/format-metadata.interface';
import { SophiaRendererComponent } from './sophia-renderer.component';

/**
 * Format plugin for Sophia psychometric assessments.
 *
 * This plugin handles content with format='sophia' or format='sophia-assessment',
 * rendering questions using the Sophia custom element which supports both
 * mastery quizzes and discovery/reflection assessments.
 */
export class SophiaFormatPlugin extends BaseContentFormatPlugin {
  // ═══════════════════════════════════════════════════════════════════════════
  // Identity
  // ═══════════════════════════════════════════════════════════════════════════

  readonly formatId = 'sophia-quiz-json';
  readonly displayName = 'Sophia Assessment';
  readonly fileExtensions = ['.sophia.json', '.sophia-quiz.json'];
  readonly mimeTypes = ['application/vnd.sophia.assessment+json'];

  /** Additional format IDs this plugin handles */
  readonly aliasFormats = ['sophia', 'perseus-quiz-json'];

  // ═══════════════════════════════════════════════════════════════════════════
  // Capabilities
  // ═══════════════════════════════════════════════════════════════════════════

  override readonly canImport = true;
  override readonly canExport = true;
  override readonly canValidate = true;
  override readonly canRender = true;
  override readonly canEdit = false;

  // ═══════════════════════════════════════════════════════════════════════════
  // Rendering
  // ═══════════════════════════════════════════════════════════════════════════

  getRendererComponent(): Type<ContentRenderer> {
    return SophiaRendererComponent;
  }

  override getRendererPriority(): number {
    return 5; // Higher priority for sophia-specific content
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // I/O Operations
  // ═══════════════════════════════════════════════════════════════════════════

  async validate(input: string | File): Promise<ValidationResult> {
    try {
      let content: unknown;

      if (typeof input === 'string') {
        content = JSON.parse(input);
      } else {
        const text = await input.text();
        content = JSON.parse(text);
      }

      const isValid = this.validateContent(content);

      return {
        valid: isValid,
        errors: isValid ? [] : [{ code: 'INVALID_STRUCTURE', message: 'Invalid Sophia assessment structure' }],
        warnings: []
      };
    } catch (error) {
      return {
        valid: false,
        errors: [{ code: 'PARSE_ERROR', message: `Parse error: ${error instanceof Error ? error.message : 'Unknown error'}` }],
        warnings: []
      };
    }
  }

  async import(input: string | File): Promise<ContentIOImportResult> {
    let content: unknown;

    if (typeof input === 'string') {
      content = JSON.parse(input);
    } else {
      const text = await input.text();
      content = JSON.parse(text);
    }

    if (!this.validateContent(content)) {
      throw new Error('Invalid Sophia assessment format');
    }

    // Extract metadata from content
    const obj = content as Record<string, unknown>;
    const title = this.extractTitle(obj);
    const purpose = this.extractPurpose(obj);

    return {
      content: content as object,
      contentFormat: 'sophia-quiz-json',
      title,
      metadata: {
        format: 'sophia-quiz-json',
        assessmentPurpose: purpose
      }
    };
  }

  async export(node: ContentIOExportInput): Promise<string | Blob> {
    const json = JSON.stringify(node.content, null, 2);
    return new Blob([json], { type: 'application/json' });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Format Metadata
  // ═══════════════════════════════════════════════════════════════════════════

  getFormatMetadata(): FormatMetadata {
    return {
      formatId: this.formatId,
      displayName: this.displayName,
      description: 'Sophia assessment format for mastery and discovery quizzes',
      fileExtensions: this.fileExtensions,
      mimeTypes: this.mimeTypes,
      icon: 'quiz',
      category: 'data', // Assessment content is structured data
      supportsRoundTrip: true,
      priority: 10 // Higher priority for assessment content
    };
  }

  override getEditorConfig(): EditorConfig {
    return {
      ...DEFAULT_EDITOR_CONFIG,
      editorMode: 'code',
      supportsLivePreview: true
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Format Detection
  // ═══════════════════════════════════════════════════════════════════════════

  override detectFormat(content: string): number | null {
    try {
      const parsed = JSON.parse(content);
      if (this.validateContent(parsed)) {
        // Check for sophia-specific markers
        const obj = parsed as Record<string, unknown>;
        if (obj['purpose'] === 'reflection' || obj['purpose'] === 'discovery') {
          return 0.95; // High confidence for reflection/discovery
        }
        if (obj['purpose'] === 'mastery' && obj['content']) {
          return 0.85; // Good confidence for mastery moments
        }
        if (obj['question'] && obj['hints'] !== undefined) {
          return 0.7; // Moderate confidence for Perseus-like items
        }
        return 0.5; // Low confidence
      }
    } catch {
      return null;
    }
    return null;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Private Helpers
  // ═══════════════════════════════════════════════════════════════════════════

  private validateContent(content: unknown): boolean {
    if (!content || typeof content !== 'object') {
      return false;
    }

    const obj = content as Record<string, unknown>;

    // Valid Moment format (has purpose and content properties)
    if (obj['purpose'] && obj['content']) {
      const innerContent = obj['content'] as Record<string, unknown>;
      return typeof innerContent['content'] === 'string' &&
             typeof innerContent['widgets'] === 'object';
    }

    // Valid Perseus-compatible format (has question property)
    if (obj['question']) {
      const question = obj['question'] as Record<string, unknown>;
      return typeof question['content'] === 'string' &&
             typeof question['widgets'] === 'object';
    }

    // Array of moments or questions
    if (Array.isArray(content)) {
      return content.length > 0 && content.every(item => this.validateContent(item));
    }

    return false;
  }

  private extractTitle(obj: Record<string, unknown>): string {
    if (obj['metadata'] && typeof obj['metadata'] === 'object') {
      const metadata = obj['metadata'] as Record<string, unknown>;
      if (typeof metadata['title'] === 'string') {
        return metadata['title'];
      }
    }

    const purpose = this.extractPurpose(obj);
    if (purpose === 'reflection' || purpose === 'discovery') {
      return 'Discovery Assessment';
    }
    return 'Sophia Assessment';
  }

  private extractPurpose(obj: Record<string, unknown>): string {
    if (typeof obj['purpose'] === 'string') {
      return obj['purpose'];
    }
    if (obj['discoveryMode'] === true) {
      return 'discovery';
    }
    return 'mastery';
  }
}
