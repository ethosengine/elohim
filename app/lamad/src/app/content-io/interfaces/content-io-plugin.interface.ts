import { FormatMetadata } from './format-metadata.interface';
import { ValidationResult } from './validation-result.interface';

/**
 * Contract for content I/O plugins.
 *
 * Plugins handle import/export for specific content formats.
 * They work alongside the existing RendererRegistryService which handles rendering.
 *
 * A plugin provides:
 * - Import: Parse source format → ContentNode data
 * - Export: ContentNode → source format string/blob
 * - Validate: Check source format validity
 */
export interface ContentIOPlugin {
  // ─────────────────────────────────────────────────────────────────────────────
  // Metadata
  // ─────────────────────────────────────────────────────────────────────────────

  /** Unique identifier matching ContentFormat (e.g., 'markdown', 'gherkin') */
  readonly formatId: string;

  /** Human-readable display name */
  readonly displayName: string;

  /** File extensions this plugin handles (e.g., ['.md', '.markdown']) */
  readonly fileExtensions: string[];

  /** MIME types this plugin handles */
  readonly mimeTypes: string[];

  // ─────────────────────────────────────────────────────────────────────────────
  // Capabilities
  // ─────────────────────────────────────────────────────────────────────────────

  /** Whether this plugin can import (parse) files */
  readonly canImport: boolean;

  /** Whether this plugin can export (serialize) content */
  readonly canExport: boolean;

  /** Whether this plugin can validate content */
  readonly canValidate: boolean;

  // ─────────────────────────────────────────────────────────────────────────────
  // Operations
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Import content from source format.
   * @param input - Raw content string or File object
   * @returns Parsed content data for creating a ContentNode
   */
  import(input: string | File): Promise<ContentIOImportResult>;

  /**
   * Export ContentNode to source format.
   * @param node - The content to export
   * @returns String content or Blob for binary formats
   */
  export(node: ContentIOExportInput): Promise<string | Blob>;

  /**
   * Validate content without fully importing it.
   * @param input - Raw content string or File object
   * @returns Validation result with errors/warnings
   */
  validate(input: string | File): Promise<ValidationResult>;

  // ─────────────────────────────────────────────────────────────────────────────
  // Optional
  // ─────────────────────────────────────────────────────────────────────────────

  /** Get full format metadata for UI display */
  getFormatMetadata(): FormatMetadata;

  /** Detect if content matches this format (confidence 0-1) */
  detectFormat?(content: string): number | null;
}

/**
 * Result of an import operation.
 */
export interface ContentIOImportResult {
  /** The parsed content body */
  content: string | object;

  /** Content format identifier */
  contentFormat: string;

  /** Suggested content type */
  contentType?: string;

  /** Title extracted from content */
  title?: string;

  /** Description extracted from content */
  description?: string;

  /** Tags extracted from content */
  tags?: string[];

  /** Additional metadata */
  metadata?: Record<string, unknown>;

  /** Related node IDs discovered during parsing */
  relatedNodeIds?: string[];

  /** Raw frontmatter if present */
  frontmatter?: Record<string, unknown>;
}

/**
 * Input for export operations.
 * Subset of ContentNode fields needed for export.
 */
export interface ContentIOExportInput {
  id?: string;
  title: string;
  description?: string;
  content: string | object;
  contentFormat: string;
  contentType?: string;
  tags?: string[];
  metadata?: Record<string, unknown>;
  relatedNodeIds?: string[];
}
