import { Injectable, Type } from '@angular/core';

import {
  ContentFormatPlugin,
  ContentRenderer,
  ContentEditorComponent,
  EditorConfig,
  DEFAULT_EDITOR_CONFIG,
  InteractiveRenderer,
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

import { PerseusRendererComponent } from './perseus-renderer.component';

import type { PerseusItem, PerseusWidgetType } from './perseus-item.model';

/**
 * PerseusFormatPlugin - Unified plugin for Perseus quiz content.
 *
 * Provides:
 * - Import: Parse Perseus JSON format → ContentNode data
 * - Export: ContentNode → Perseus JSON format
 * - Validate: Check Perseus item structure
 * - Render: PerseusRendererComponent with React integration
 *
 * This replaces the stub quiz-renderer with Khan Academy's Perseus system.
 *
 * @example
 * ```typescript
 * // Register in content-io module
 * registry.register(new PerseusFormatPlugin());
 *
 * // Use for quiz content
 * const plugin = registry.getPlugin('perseus');
 * const imported = await plugin.import(perseusJson);
 * ```
 */
@Injectable({
  providedIn: 'root',
})
export class PerseusFormatPlugin implements ContentFormatPlugin {
  // ═══════════════════════════════════════════════════════════════════════════
  // Identity
  // ═══════════════════════════════════════════════════════════════════════════

  readonly formatId = 'perseus';
  readonly displayName = 'Perseus Quiz';
  readonly fileExtensions = ['.perseus.json', '.quiz.json'];
  readonly mimeTypes = ['application/vnd.perseus+json', 'application/json'];

  // ═══════════════════════════════════════════════════════════════════════════
  // Capabilities
  // ═══════════════════════════════════════════════════════════════════════════

  readonly canImport = true;
  readonly canExport = true;
  readonly canValidate = true;
  readonly canRender = true;
  readonly canEdit = false; // Future: Perseus Editor integration

  // ═══════════════════════════════════════════════════════════════════════════
  // Import
  // ═══════════════════════════════════════════════════════════════════════════

  async import(input: string | File): Promise<ContentIOImportResult> {
    const content = typeof input === 'string' ? input : await this.readFile(input);
    const sourcePath = typeof input === 'string' ? 'imported.perseus.json' : input.name;

    try {
      const parsed = JSON.parse(content) as PerseusItem | PerseusItem[];
      const items = Array.isArray(parsed) ? parsed : [parsed];

      // Extract metadata from first item (or aggregate for multiple)
      const primaryItem = items[0];
      const metadata = primaryItem?.metadata;

      return {
        content: items,
        contentFormat: 'perseus',
        contentType: 'quiz',
        title: this.extractTitle(items, sourcePath),
        description: this.extractDescription(items),
        tags: metadata?.tags ?? [],
        metadata: {
          itemCount: items.length,
          widgetTypes: this.extractWidgetTypes(items),
          bloomsLevels: this.extractBloomsLevels(items),
          difficulty: this.aggregateDifficulty(items),
          estimatedTimeMinutes: this.calculateEstimatedTime(items),
          sourceDoc: metadata?.sourceDoc,
        },
      };
    } catch (error) {
      throw new Error(
        `Failed to parse Perseus JSON: ${error instanceof Error ? error.message : 'Unknown error'}`
      );
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Export
  // ═══════════════════════════════════════════════════════════════════════════

  async export(node: ContentIOExportInput): Promise<string> {
    // Content should already be Perseus item(s)
    const items = node.content as PerseusItem | PerseusItem[];

    // Ensure items have metadata populated
    const itemsArray = Array.isArray(items) ? items : [items];
    const updatedItems = itemsArray.map(item => ({
      ...item,
      metadata: {
        ...item.metadata,
        updatedAt: new Date().toISOString(),
      },
    }));

    return JSON.stringify(updatedItems.length === 1 ? updatedItems[0] : updatedItems, null, 2);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Validation
  // ═══════════════════════════════════════════════════════════════════════════

  async validate(input: string | File): Promise<ValidationResult> {
    const content = typeof input === 'string' ? input : await this.readFile(input);
    const errors: ValidationError[] = [];
    const warnings: ValidationWarning[] = [];

    // Try to parse as JSON
    let parsed: unknown;
    try {
      parsed = JSON.parse(content);
    } catch (error) {
      errors.push({
        code: 'INVALID_JSON',
        message: `Invalid JSON: ${error instanceof Error ? error.message : 'Parse error'}`,
        line: 1,
      });
      return { valid: false, errors, warnings };
    }

    // Validate structure
    const items = Array.isArray(parsed) ? parsed : [parsed];

    for (let i = 0; i < items.length; i++) {
      const item = items[i] as Partial<PerseusItem>;
      const prefix = items.length > 1 ? `Item ${i + 1}: ` : '';

      this.validateItem(item, prefix, errors, warnings);
    }

    // Calculate stats
    const stats = {
      itemCount: items.length,
      widgetCount: this.countWidgets(items as PerseusItem[]),
      hasHints: items.some((item: Partial<PerseusItem>) => item.hints && item.hints.length > 0),
    };

    return {
      valid: errors.length === 0,
      errors,
      warnings,
      stats,
    };
  }

  private validateItem(
    item: Partial<PerseusItem>,
    prefix: string,
    errors: ValidationError[],
    warnings: ValidationWarning[]
  ): void {
    // Check required fields
    if (!item.id) {
      errors.push({
        code: 'MISSING_ID',
        message: `${prefix}Missing required field: id`,
      });
    }

    this.validateQuestion(item, prefix, errors, warnings);
    this.validateItemMetadata(item, prefix, warnings);

    // Validate version
    if (!item.version) {
      warnings.push({
        code: 'NO_VERSION',
        message: `${prefix}No version specified. Using default 1.0.`,
        suggestion: 'Add version: { major: 1, minor: 0 }',
      });
    }
  }

  private validateQuestion(
    item: Partial<PerseusItem>,
    prefix: string,
    errors: ValidationError[],
    warnings: ValidationWarning[]
  ): void {
    if (!item.question) {
      errors.push({
        code: 'MISSING_QUESTION',
        message: `${prefix}Missing required field: question`,
      });
      return;
    }

    // Validate question structure
    if (typeof item.question.content !== 'string') {
      errors.push({
        code: 'INVALID_CONTENT',
        message: `${prefix}question.content must be a string`,
      });
    }

    if (!item.question.widgets || typeof item.question.widgets !== 'object') {
      warnings.push({
        code: 'NO_WIDGETS',
        message: `${prefix}No widgets defined. Question may be display-only.`,
        suggestion: 'Add widgets for interactive elements',
      });
    } else {
      // Validate widgets
      this.validateWidgets(item.question.widgets, errors, warnings, prefix);
    }
  }

  private validateItemMetadata(
    item: Partial<PerseusItem>,
    prefix: string,
    warnings: ValidationWarning[]
  ): void {
    if (!item.metadata) {
      warnings.push({
        code: 'NO_METADATA',
        message: `${prefix}No metadata. Lamad integration features may not work.`,
        suggestion: 'Add metadata with assessesContentId, bloomsLevel, etc.',
      });
      return;
    }

    if (!item.metadata.assessesContentId) {
      warnings.push({
        code: 'NO_CONTENT_LINK',
        message: `${prefix}No assessesContentId. Question won't be linked to content.`,
        suggestion: 'Set metadata.assessesContentId to the content node ID',
      });
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Rendering
  // ═══════════════════════════════════════════════════════════════════════════

  getRendererComponent(): Type<ContentRenderer & InteractiveRenderer> {
    return PerseusRendererComponent;
  }

  getRendererPriority(): number {
    return 20; // Higher than markdown for quiz content
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Editing
  // ═══════════════════════════════════════════════════════════════════════════

  getEditorComponent(): Type<ContentEditorComponent> | null {
    // Future: return PerseusEditorComponent for visual editing
    return null; // Use default JSON editor for now
  }

  getEditorConfig(): EditorConfig {
    return {
      ...DEFAULT_EDITOR_CONFIG,
      editorMode: 'code',
      supportsLivePreview: true,
      showLineNumbers: true,
      wordWrap: false, // JSON is easier with no wrap
      toolbar: {
        enabled: true,
        position: 'top',
        actions: [
          { id: 'format', label: 'Format JSON', icon: 'code', type: 'button' },
          { id: 'validate', label: 'Validate', icon: 'check_circle', type: 'button' },
          { id: 'save', label: 'Save', icon: 'save', shortcut: 'Ctrl+S', type: 'button' },
          { id: 'cancel', label: 'Cancel', icon: 'close', shortcut: 'Escape', type: 'button' },
        ],
      },
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Format Detection
  // ═══════════════════════════════════════════════════════════════════════════

  detectFormat(content: string): number | null {
    let confidence = 0;

    // Must be valid JSON
    try {
      const parsed = JSON.parse(content);
      const items = Array.isArray(parsed) ? parsed : [parsed];

      // Check for Perseus-specific structure
      for (const item of items) {
        if (item.question?.widgets) {
          confidence += 0.4;
        }
        if (item.question?.content?.includes('[[☃')) {
          confidence += 0.3; // Widget placeholder syntax
        }
        if (item.metadata?.assessesContentId) {
          confidence += 0.2;
        }
        if (item.hints) {
          confidence += 0.1;
        }
      }
    } catch {
      return null; // Not valid JSON
    }

    return confidence > 0 ? Math.min(confidence, 1) : null;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Metadata
  // ═══════════════════════════════════════════════════════════════════════════

  getFormatMetadata(): FormatMetadata {
    return {
      formatId: this.formatId,
      displayName: this.displayName,
      description: 'Khan Academy Perseus quiz format with interactive widgets',
      fileExtensions: this.fileExtensions,
      mimeTypes: this.mimeTypes,
      icon: 'quiz',
      category: 'data', // Perseus quizzes are structured data
      supportsRoundTrip: true,
      priority: 20,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Private Helpers
  // ═══════════════════════════════════════════════════════════════════════════

  private async readFile(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result as string);
      reader.onerror = () => reject(new Error(reader.error?.message ?? 'Failed to read file'));
      reader.readAsText(file);
    });
  }

  private extractTitle(items: PerseusItem[], fallback: string): string {
    // Try to get title from first item's metadata or question content
    const firstItem = items[0];
    if (firstItem?.metadata?.sourceDoc) {
      // Extract title from source doc path
      const parts = firstItem.metadata.sourceDoc.split('/');
      const filename = parts[parts.length - 1];
      return filename.replace(/\.[^.]+$/, '').replace(/-/g, ' ');
    }

    if (items.length === 1 && firstItem?.question?.content) {
      // Use first line of content as title
      const firstLine = firstItem.question.content.split('\n')[0];
      if (firstLine.length < 100) {
        return firstLine.replace(/[#*_]/g, '').trim();
      }
    }

    return `Quiz (${items.length} question${items.length > 1 ? 's' : ''})`;
  }

  private extractDescription(items: PerseusItem[]): string {
    const types = this.extractWidgetTypes(items);
    const levels = this.extractBloomsLevels(items);

    return `${items.length} question${items.length > 1 ? 's' : ''} with ${types.join(', ')} widgets. Bloom's levels: ${levels.join(', ')}.`;
  }

  private extractWidgetTypes(items: PerseusItem[]): PerseusWidgetType[] {
    const types = new Set<PerseusWidgetType>();

    for (const item of items) {
      if (item.question?.widgets) {
        for (const widget of Object.values(item.question.widgets)) {
          types.add(widget.type);
        }
      }
    }

    return Array.from(types);
  }

  private extractBloomsLevels(items: PerseusItem[]): string[] {
    const levels = new Set<string>();

    for (const item of items) {
      if (item.metadata?.bloomsLevel) {
        levels.add(item.metadata.bloomsLevel);
      }
    }

    return levels.size > 0 ? Array.from(levels) : ['understand'];
  }

  private aggregateDifficulty(items: PerseusItem[]): string {
    const difficulties: Record<string, number> = { easy: 0, medium: 0, hard: 0 };

    for (const item of items) {
      const diff = item.metadata?.difficulty ?? 'medium';
      difficulties[diff]++;
    }

    // Return most common difficulty
    return Object.entries(difficulties).sort((a, b) => b[1] - a[1])[0][0];
  }

  private calculateEstimatedTime(items: PerseusItem[]): number {
    let totalSeconds = 0;

    for (const item of items) {
      totalSeconds += item.metadata?.estimatedTimeSeconds ?? 60;
    }

    return Math.ceil(totalSeconds / 60);
  }

  private validateWidgets(
    widgets: Record<string, unknown>,
    errors: ValidationError[],
    warnings: ValidationWarning[],
    prefix: string
  ): void {
    const validTypes: PerseusWidgetType[] = [
      'radio',
      'numeric-input',
      'expression',
      'input-number',
      'interactive-graph',
      'image',
      'transformer',
      'number-line',
      'sorter',
      'categorizer',
      'matcher',
      'orderer',
      'graded-group',
      'graded-group-set',
      'iframe',
      'definition',
      'dropdown',
      'explanation',
      'passage',
      'passage-ref',
      'phet-simulation',
      'plotter',
      'table',
      'grapher',
      'measurer',
      'matrix',
      'cs-program',
      'video',
      'label-image',
    ];

    for (const [key, widget] of Object.entries(widgets)) {
      const w = widget as { type?: string; options?: unknown };

      if (!w.type) {
        errors.push({
          code: 'WIDGET_NO_TYPE',
          message: `${prefix}Widget "${key}" has no type`,
        });
        continue;
      }

      if (!validTypes.includes(w.type as PerseusWidgetType)) {
        warnings.push({
          code: 'UNKNOWN_WIDGET_TYPE',
          message: `${prefix}Unknown widget type: ${w.type}`,
          suggestion: `Valid types: ${validTypes.slice(0, 5).join(', ')}...`,
        });
      }

      if (!w.options) {
        warnings.push({
          code: 'WIDGET_NO_OPTIONS',
          message: `${prefix}Widget "${key}" has no options`,
          suggestion: 'Add options appropriate for the widget type',
        });
      }
    }
  }

  private countWidgets(items: PerseusItem[]): number {
    let count = 0;
    for (const item of items) {
      if (item.question?.widgets) {
        count += Object.keys(item.question.widgets).length;
      }
    }
    return count;
  }
}
