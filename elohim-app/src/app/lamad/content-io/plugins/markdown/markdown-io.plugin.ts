import { Injectable } from '@angular/core';
import { ContentIOPlugin, ContentIOImportResult, ContentIOExportInput } from '../../interfaces/content-io-plugin.interface';
import { FormatMetadata } from '../../interfaces/format-metadata.interface';
import { ValidationResult, ValidationError, ValidationWarning } from '../../interfaces/validation-result.interface';
import { MarkdownParser } from '../../../parsers/markdown-parser';

/**
 * Markdown I/O Plugin
 *
 * Handles import, export, and validation of Markdown content.
 * Rendering is handled separately by the RendererRegistryService.
 *
 * - Import: Uses MarkdownParser to parse markdown with YAML frontmatter
 * - Export: Generates markdown with YAML frontmatter from ContentNode
 * - Validate: Checks frontmatter, headings, and references
 */
@Injectable({
  providedIn: 'root'
})
export class MarkdownIOPlugin implements ContentIOPlugin {
  readonly formatId = 'markdown';
  readonly displayName = 'Markdown';
  readonly fileExtensions = ['.md', '.markdown'];
  readonly mimeTypes = ['text/markdown', 'text/x-markdown', 'text/plain'];

  readonly canImport = true;
  readonly canExport = true;
  readonly canValidate = true;

  // ─────────────────────────────────────────────────────────────────────────────
  // Import
  // ─────────────────────────────────────────────────────────────────────────────

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
      frontmatter: this.extractFrontmatter(content)
    };
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Export
  // ─────────────────────────────────────────────────────────────────────────────

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

  // ─────────────────────────────────────────────────────────────────────────────
  // Validation
  // ─────────────────────────────────────────────────────────────────────────────

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
          tags: imported.tags
        };
      } catch {
        // Preview generation failed, but that's okay
      }
    }

    // Calculate stats
    const stats = {
      wordCount: this.countWords(content),
      lineCount: content.split('\n').length,
      sectionCount: (content.match(/^#{1,6}\s+/gm) ?? []).length
    };

    return {
      valid: errors.length === 0,
      errors,
      warnings,
      parsedPreview,
      stats
    };
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Format Detection
  // ─────────────────────────────────────────────────────────────────────────────

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

  // ─────────────────────────────────────────────────────────────────────────────
  // Metadata
  // ─────────────────────────────────────────────────────────────────────────────

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
      priority: 10
    };
  }

  getDefaultOptions(): Record<string, unknown> {
    return {
      preserveFrontmatter: true,
      generateToc: false
    };
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private Helpers
  // ─────────────────────────────────────────────────────────────────────────────

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

    return lines.slice(endIndex + 1).join('\n').trimStart();
  }

  private generateFrontmatter(node: ContentIOExportInput): string | null {
    const fields: string[] = [];

    // Core fields
    if (node.title) {
      fields.push(`title: "${node.title.replace(/"/g, '\\"')}"`);
    }
    if (node.description) {
      // Truncate long descriptions in frontmatter
      const desc = node.description.length > 200
        ? node.description.substring(0, 197) + '...'
        : node.description;
      fields.push(`description: "${desc.replace(/"/g, '\\"').replace(/\n/g, ' ')}"`);
    }
    if (node.contentType) {
      fields.push(`contentType: ${node.contentType}`);
    }
    if (node.tags && node.tags.length > 0) {
      fields.push(`tags: [${node.tags.join(', ')}]`);
    }

    // Metadata fields
    if (node.metadata) {
      const skipKeys = ['sections', 'wordCount', 'headingCount']; // Internal metadata
      for (const [key, value] of Object.entries(node.metadata)) {
        if (skipKeys.includes(key)) continue;
        if (value === null || value === undefined) continue;

        if (Array.isArray(value)) {
          fields.push(`${key}: [${value.join(', ')}]`);
        } else if (value !== null && typeof value === 'object') {
          // Skip complex objects in frontmatter
          continue;
        } else {
          // Primitive values (string, number, boolean)
          fields.push(`${key}: ${String(value)}`);
        }
      }
    }

    return fields.length > 0 ? fields.join('\n') : null;
  }

  private validateFrontmatter(content: string): { errors: ValidationError[]; warnings: ValidationWarning[] } {
    const errors: ValidationError[] = [];
    const warnings: ValidationWarning[] = [];

    if (!content.startsWith('---')) {
      warnings.push({
        code: 'NO_FRONTMATTER',
        message: 'No YAML frontmatter found. Consider adding metadata.',
        suggestion: 'Add a --- delimited frontmatter block at the start of the file'
      });
      return { errors, warnings };
    }

    const lines = content.split('\n');
    const endIndex = lines.findIndex((line, i) => i > 0 && line.trim() === '---');

    if (endIndex === -1) {
      errors.push({
        code: 'UNCLOSED_FRONTMATTER',
        message: 'Frontmatter block is not closed. Missing closing ---',
        line: 1
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
          suggestion: 'Add a title field to frontmatter'
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
        suggestion: 'Add a # Title at the start of content'
      });
    } else if (h1Count > 1) {
      warnings.push({
        code: 'MULTIPLE_H1',
        message: 'Multiple H1 headings found. Document should have a single main title.',
        suggestion: 'Use H2 (##) for subsections'
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
        suggestion: 'Fill in the reference identifier'
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
