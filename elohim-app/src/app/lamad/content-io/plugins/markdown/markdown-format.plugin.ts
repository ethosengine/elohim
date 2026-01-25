import { Injectable, Type } from '@angular/core';

import { MarkdownParser } from '../../../parsers/markdown-parser';
import { MarkdownRendererComponent } from '../../../renderers/markdown-renderer/markdown-renderer.component';
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
 * MarkdownFormatPlugin - Unified plugin for Markdown content.
 *
 * Provides:
 * - Import: Parse markdown with YAML frontmatter → ContentNode data
 * - Export: ContentNode → markdown with YAML frontmatter
 * - Validate: Check frontmatter, headings, and references
 * - Render: MarkdownRendererComponent with TOC, syntax highlighting
 * - Edit: Uses default code editor (no custom editor yet)
 *
 * This unified plugin replaces the separate MarkdownIOPlugin and
 * RendererRegistryService registration.
 */
@Injectable({
  providedIn: 'root',
})
export class MarkdownFormatPlugin implements ContentFormatPlugin {
  // ═══════════════════════════════════════════════════════════════════════════
  // Identity
  // ═══════════════════════════════════════════════════════════════════════════

  readonly formatId = 'markdown';
  readonly displayName = 'Markdown';
  readonly fileExtensions = ['.md', '.markdown'];
  readonly mimeTypes = ['text/markdown', 'text/x-markdown', 'text/plain'];

  // ═══════════════════════════════════════════════════════════════════════════
  // Capabilities
  // ═══════════════════════════════════════════════════════════════════════════

  readonly canImport = true;
  readonly canExport = true;
  readonly canValidate = true;
  readonly canRender = true;
  readonly canEdit = false; // Uses default editor for now

  // ═══════════════════════════════════════════════════════════════════════════
  // Import
  // ═══════════════════════════════════════════════════════════════════════════

  async import(input: string | File): Promise<ContentIOImportResult> {
    const content = typeof input === 'string' ? input : await this.readFile(input);
    const sourcePath = typeof input === 'string' ? 'imported.md' : input.name;

    // Use existing MarkdownParser
    const parsed = MarkdownParser.parseContent(content, sourcePath);

    return {
      content: parsed.content,
      contentFormat: 'markdown',
      contentType: parsed.contentType,
      title: parsed.title,
      description: parsed.description,
      tags: parsed.tags,
      metadata: parsed.metadata,
      relatedNodeIds: parsed.relatedNodeIds,
      frontmatter: this.extractFrontmatter(content),
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Export
  // ═══════════════════════════════════════════════════════════════════════════

  async export(node: ContentIOExportInput): Promise<string> {
    const lines: string[] = [];

    // Generate YAML frontmatter
    const frontmatter = this.generateFrontmatter(node);
    if (frontmatter) {
      lines.push('---');
      lines.push(frontmatter);
      lines.push('---');
      lines.push('');
    }

    // Add content
    if (typeof node.content === 'string') {
      // Strip existing frontmatter from content if present
      const contentWithoutFrontmatter = this.stripFrontmatter(node.content);
      lines.push(contentWithoutFrontmatter);
    } else {
      // Content is not a string, try to serialize
      lines.push(`# ${node.title}`);
      lines.push('');
      if (node.description) {
        lines.push(node.description);
        lines.push('');
      }
      lines.push('```json');
      lines.push(JSON.stringify(node.content, null, 2));
      lines.push('```');
    }

    return lines.join('\n');
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Validation
  // ═══════════════════════════════════════════════════════════════════════════

  async validate(input: string | File): Promise<ValidationResult> {
    const content = typeof input === 'string' ? input : await this.readFile(input);
    const errors: ValidationError[] = [];
    const warnings: ValidationWarning[] = [];

    // Check for frontmatter validity
    const frontmatterResult = this.validateFrontmatter(content);
    errors.push(...frontmatterResult.errors);
    warnings.push(...frontmatterResult.warnings);

    // Check for heading structure
    const headingResult = this.validateHeadings(content);
    warnings.push(...headingResult.warnings);

    // Check for broken references
    const referenceResult = this.validateReferences(content);
    warnings.push(...referenceResult.warnings);

    // Generate preview
    let parsedPreview: ContentIOImportResult['metadata'] | undefined;
    if (errors.length === 0) {
      try {
        const imported = await this.import(content);
        parsedPreview = {
          title: imported.title,
          description: imported.description,
          contentType: imported.contentType,
          tags: imported.tags,
        };
      } catch {
        // Preview generation failed, but that's okay
      }
    }

    // Calculate stats
    const stats = {
      wordCount: this.countWords(content),
      lineCount: content.split('\n').length,
      sectionCount: (content.match(/^#{1,6}\s+/gm) ?? []).length,
    };

    return {
      valid: errors.length === 0,
      errors,
      warnings,
      parsedPreview,
      stats,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Rendering
  // ═══════════════════════════════════════════════════════════════════════════

  getRendererComponent(): Type<ContentRenderer> {
    return MarkdownRendererComponent;
  }

  getRendererPriority(): number {
    return 10; // High priority for markdown
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Editing
  // ═══════════════════════════════════════════════════════════════════════════

  getEditorComponent(): Type<ContentEditorComponent> | null {
    return null; // Use default code editor
  }

  getEditorConfig(): EditorConfig {
    return {
      ...DEFAULT_EDITOR_CONFIG,
      editorMode: 'code',
      supportsLivePreview: true,
      showLineNumbers: true,
      wordWrap: true,
      toolbar: {
        enabled: true,
        position: 'top',
        actions: [
          { id: 'bold', label: 'Bold', icon: 'format_bold', shortcut: 'Ctrl+B', type: 'button' },
          {
            id: 'italic',
            label: 'Italic',
            icon: 'format_italic',
            shortcut: 'Ctrl+I',
            type: 'button',
          },
          {
            id: 'heading',
            label: 'Heading',
            icon: 'title',
            type: 'dropdown',
            children: [
              { id: 'h1', label: 'Heading 1', icon: 'looks_one', type: 'button' },
              { id: 'h2', label: 'Heading 2', icon: 'looks_two', type: 'button' },
              { id: 'h3', label: 'Heading 3', icon: 'looks_3', type: 'button' },
            ],
          },
          { id: 'link', label: 'Link', icon: 'link', shortcut: 'Ctrl+K', type: 'button' },
          { id: 'code', label: 'Code', icon: 'code', shortcut: 'Ctrl+`', type: 'button' },
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

    // Check for YAML frontmatter (strong indicator)
    if (content.startsWith('---')) {
      confidence += 0.3;
    }

    // Check for markdown headings
    if (/^#{1,6}\s+.+$/m.test(content)) {
      confidence += 0.3;
    }

    // Check for markdown links
    if (/\[.+\]\(.+\)/.test(content)) {
      confidence += 0.2;
    }

    // Check for markdown emphasis
    if (/\*\*.+\*\*|\*[^*]+\*/.test(content)) {
      confidence += 0.1;
    }

    // Check for markdown lists
    if (/^[-*]\s+.+$/m.test(content)) {
      confidence += 0.1;
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
      description: 'Markdown documents with optional YAML frontmatter',
      fileExtensions: this.fileExtensions,
      mimeTypes: this.mimeTypes,
      icon: 'description',
      category: 'document',
      supportsRoundTrip: true,
      priority: 10,
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

  private extractFrontmatter(content: string): Record<string, unknown> | undefined {
    if (!content.startsWith('---')) return undefined;

    const lines = content.split('\n');
    const endIndex = lines.findIndex((line, i) => i > 0 && line.trim() === '---');
    if (endIndex === -1) return undefined;

    const frontmatterLines = lines.slice(1, endIndex);
    const frontmatter: Record<string, unknown> = {};

    for (const line of frontmatterLines) {
      const match = /^(\w+):\s*(.+)$/.exec(line);
      if (match) {
        const [, key, value] = match;
        if (value.includes(',') || value.startsWith('[')) {
          frontmatter[key] = value
            .replace(/[[\]]/g, '')
            .split(',')
            .map(v => v.trim());
        } else {
          frontmatter[key] = value.trim();
        }
      }
    }

    return frontmatter;
  }

  private stripFrontmatter(content: string): string {
    if (!content.startsWith('---')) return content;

    const lines = content.split('\n');
    const endIndex = lines.findIndex((line, i) => i > 0 && line.trim() === '---');
    if (endIndex === -1) return content;

    return lines
      .slice(endIndex + 1)
      .join('\n')
      .trimStart();
  }

  private generateFrontmatter(node: ContentIOExportInput): string | null {
    const fields: string[] = [];

    this.addCoreFields(fields, node);
    this.addMetadataFields(fields, node.metadata);

    return fields.length > 0 ? fields.join('\n') : null;
  }

  private addCoreFields(fields: string[], node: ContentIOExportInput): void {
    if (node.title) {
      fields.push(`title: "${node.title.replace(/"/g, '\\"')}"`);
    }
    if (node.description) {
      const desc = this.truncateDescription(node.description, 200);
      fields.push(`description: "${desc.replace(/"/g, '\\"').replace(/\n/g, ' ')}"`);
    }
    if (node.contentType) {
      fields.push(`contentType: ${node.contentType}`);
    }
    if (node.tags && node.tags.length > 0) {
      fields.push(`tags: [${node.tags.join(', ')}]`);
    }
  }

  private truncateDescription(desc: string, maxLength: number): string {
    return desc.length > maxLength ? desc.substring(0, maxLength - 3) + '...' : desc;
  }

  private addMetadataFields(fields: string[], metadata?: Record<string, unknown>): void {
    if (!metadata) return;

    const skipKeys = ['sections', 'wordCount', 'headingCount'];
    for (const [key, value] of Object.entries(metadata)) {
      if (skipKeys.includes(key) || value === null || value === undefined) continue;
      const formattedValue = this.formatMetadataValue(value);
      if (formattedValue !== null) {
        fields.push(`${key}: ${formattedValue}`);
      }
    }
  }

  private formatMetadataValue(value: unknown): string | null {
    if (Array.isArray(value)) {
      return `[${value.join(', ')}]`;
    }
    if (typeof value === 'object') {
      return null; // Skip complex objects
    }
    if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') {
      return String(value);
    }
    return null;
  }

  private validateFrontmatter(content: string): {
    errors: ValidationError[];
    warnings: ValidationWarning[];
  } {
    const errors: ValidationError[] = [];
    const warnings: ValidationWarning[] = [];

    if (!content.startsWith('---')) {
      warnings.push({
        code: 'NO_FRONTMATTER',
        message: 'No YAML frontmatter found. Consider adding metadata.',
        suggestion: 'Add a --- delimited frontmatter block at the start of the file',
      });
      return { errors, warnings };
    }

    const lines = content.split('\n');
    const endIndex = lines.findIndex((line, i) => i > 0 && line.trim() === '---');

    if (endIndex === -1) {
      errors.push({
        code: 'UNCLOSED_FRONTMATTER',
        message: 'Frontmatter block is not closed. Missing closing ---',
        line: 1,
      });
      return { errors, warnings };
    }

    // Check for required fields
    const frontmatter = this.extractFrontmatter(content);
    if (frontmatter) {
      if (!frontmatter['title']) {
        warnings.push({
          code: 'NO_TITLE',
          message: 'No title in frontmatter. Title will be extracted from first heading.',
          suggestion: 'Add a title field to frontmatter',
        });
      }
    }

    return { errors, warnings };
  }

  private validateHeadings(content: string): { warnings: ValidationWarning[] } {
    const warnings: ValidationWarning[] = [];
    const headings = content.match(/^(#{1,6})\s+.+$/gm) ?? [];

    // Check for H1
    const h1Count = headings.filter(h => h.startsWith('# ')).length;
    if (h1Count === 0) {
      warnings.push({
        code: 'NO_H1',
        message: 'No H1 heading found. Document should have a main title.',
        suggestion: 'Add a # Title at the start of content',
      });
    } else if (h1Count > 1) {
      warnings.push({
        code: 'MULTIPLE_H1',
        message: 'Multiple H1 headings found. Document should have a single main title.',
        suggestion: 'Use H2 (##) for subsections',
      });
    }

    return { warnings };
  }

  private validateReferences(content: string): { warnings: ValidationWarning[] } {
    const warnings: ValidationWarning[] = [];

    // Check for broken-looking references
    const brokenRefPattern = /\[(?:Feature|Scenario|Epic):\s*\]/g;
    const matches = content.match(brokenRefPattern);
    if (matches) {
      warnings.push({
        code: 'EMPTY_REFERENCE',
        message: `Found ${matches.length} empty reference(s): ${matches.join(', ')}`,
        suggestion: 'Fill in the reference identifier',
      });
    }

    return { warnings };
  }

  private countWords(content: string): number {
    // Strip frontmatter for word count
    const contentOnly = this.stripFrontmatter(content);
    return contentOnly.split(/\s+/).filter(word => word.length > 0).length;
  }
}
