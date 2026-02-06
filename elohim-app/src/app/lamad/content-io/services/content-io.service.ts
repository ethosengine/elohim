import { Injectable } from '@angular/core';

// @coverage: 73.5% (2026-02-05)

import {
  ContentIOImportResult,
  ContentIOExportInput,
} from '../interfaces/content-io-plugin.interface';
import { FormatMetadata } from '../interfaces/format-metadata.interface';
import { ValidationResult } from '../interfaces/validation-result.interface';

import { ContentFormatRegistryService } from './content-format-registry.service';

/**
 * High-level service for content import/export operations.
 *
 * Orchestrates operations across plugins registered in the ContentFormatRegistry.
 * Provides convenience methods for common operations like download and clipboard.
 */
@Injectable({
  providedIn: 'root',
})
export class ContentIOService {
  constructor(private readonly registry: ContentFormatRegistryService) {}

  // ─────────────────────────────────────────────────────────────────────────────
  // Import Operations
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Import a file, auto-detecting format.
   * @throws Error if format cannot be detected or no suitable plugin found
   */
  async importFile(file: File): Promise<ContentIOImportResult> {
    const formatId = await this.registry.detectFormat(file);

    if (!formatId) {
      throw new Error(`Cannot detect format for file: ${file.name}`);
    }

    return this.importFileAs(file, formatId);
  }

  /**
   * Import a file using a specific format.
   * @throws Error if plugin not found or import fails
   */
  async importFileAs(file: File, formatId: string): Promise<ContentIOImportResult> {
    const plugin = this.registry.getPlugin(formatId);

    if (!plugin) {
      throw new Error(`No plugin found for format: ${formatId}`);
    }

    if (!plugin.canImport) {
      throw new Error(`Plugin '${formatId}' does not support import`);
    }

    return plugin.import(file);
  }

  /**
   * Import content from a string.
   * @throws Error if plugin not found or import fails
   */
  async importString(content: string, formatId: string): Promise<ContentIOImportResult> {
    const plugin = this.registry.getPlugin(formatId);

    if (!plugin) {
      throw new Error(`No plugin found for format: ${formatId}`);
    }

    if (!plugin.canImport) {
      throw new Error(`Plugin '${formatId}' does not support import`);
    }

    return plugin.import(content);
  }

  /**
   * Import content from a string, auto-detecting format.
   */
  async importStringAutoDetect(content: string): Promise<ContentIOImportResult> {
    const formatId = this.registry.detectFormatFromContent(content);

    if (!formatId) {
      throw new Error('Cannot detect format from content');
    }

    return this.importString(content, formatId);
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Export Operations
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Export content to a specific format, returning a Blob.
   */
  async exportToFormat(node: ContentIOExportInput, formatId: string): Promise<Blob> {
    const plugin = this.registry.getPlugin(formatId);

    if (!plugin) {
      throw new Error(`No plugin found for format: ${formatId}`);
    }

    if (!plugin.canExport) {
      throw new Error(`Plugin '${formatId}' does not support export`);
    }

    const result = await plugin.export(node);

    // Convert string result to Blob
    if (typeof result === 'string') {
      const mimeType = plugin.mimeTypes[0] ?? 'text/plain';
      return new Blob([result], { type: mimeType });
    }

    return result;
  }

  /**
   * Export content to a specific format, returning a string.
   * Only works for text-based formats.
   */
  async exportToString(node: ContentIOExportInput, formatId: string): Promise<string> {
    const plugin = this.registry.getPlugin(formatId);

    if (!plugin) {
      throw new Error(`No plugin found for format: ${formatId}`);
    }

    if (!plugin.canExport) {
      throw new Error(`Plugin '${formatId}' does not support export`);
    }

    const result = await plugin.export(node);

    if (typeof result === 'string') {
      return result;
    }

    // Convert Blob to string
    return this.blobToString(result);
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Validation Operations
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Validate a file, auto-detecting format.
   */
  async validateFile(file: File): Promise<ValidationResult> {
    const formatId = await this.registry.detectFormat(file);

    if (!formatId) {
      return {
        valid: false,
        errors: [
          { code: 'UNKNOWN_FORMAT', message: `Cannot detect format for file: ${file.name}` },
        ],
        warnings: [],
      };
    }

    return this.validateFileAs(file, formatId);
  }

  /**
   * Validate a file using a specific format.
   */
  async validateFileAs(file: File, formatId: string): Promise<ValidationResult> {
    const plugin = this.registry.getPlugin(formatId);

    if (!plugin) {
      return {
        valid: false,
        errors: [{ code: 'NO_PLUGIN', message: `No plugin found for format: ${formatId}` }],
        warnings: [],
      };
    }

    if (!plugin.canValidate) {
      return {
        valid: true,
        errors: [],
        warnings: [
          { code: 'NO_VALIDATION', message: `Plugin '${formatId}' does not support validation` },
        ],
      };
    }

    return plugin.validate(file);
  }

  /**
   * Validate content string using a specific format.
   */
  async validateString(content: string, formatId: string): Promise<ValidationResult> {
    const plugin = this.registry.getPlugin(formatId);

    if (!plugin) {
      return {
        valid: false,
        errors: [{ code: 'NO_PLUGIN', message: `No plugin found for format: ${formatId}` }],
        warnings: [],
      };
    }

    if (!plugin.canValidate) {
      return {
        valid: true,
        errors: [],
        warnings: [
          { code: 'NO_VALIDATION', message: `Plugin '${formatId}' does not support validation` },
        ],
      };
    }

    return plugin.validate(content);
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Convenience Operations
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Download content as a file in the specified format.
   */
  async downloadAsFormat(
    node: ContentIOExportInput,
    formatId: string,
    filename?: string
  ): Promise<void> {
    const blob = await this.exportToFormat(node, formatId);
    const plugin = this.registry.getPlugin(formatId);
    const extension = plugin?.fileExtensions[0] ?? '';

    const downloadFilename =
      filename ?? this.sanitizeFilename(node.title ?? node.id ?? 'content') + extension;

    this.downloadBlob(blob, downloadFilename);
  }

  /**
   * Copy content to clipboard in the specified format.
   * Only works for text-based formats.
   */
  async copyToClipboard(node: ContentIOExportInput, formatId: string): Promise<void> {
    const text = await this.exportToString(node, formatId);
    await navigator.clipboard.writeText(text);
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Format Discovery (pass-through to registry)
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Get all formats that support import.
   */
  getImportableFormats(): FormatMetadata[] {
    return this.registry.getImportableFormats();
  }

  /**
   * Get all formats that support export.
   */
  getExportableFormats(): FormatMetadata[] {
    return this.registry.getExportableFormats();
  }

  /**
   * Get exportable formats appropriate for a specific content node.
   */
  getExportableFormatsForContent(node: ContentIOExportInput): FormatMetadata[] {
    return this.registry.getExportableFormatsForContent(node);
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Bulk Operations
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Check if a content node can be exported (has a registered plugin).
   */
  canExport(node: ContentIOExportInput): boolean {
    const plugin = this.registry.getPlugin(node.contentFormat);
    return !!plugin?.canExport;
  }

  /**
   * Get the source format for a content node.
   * Returns null if no export plugin is available.
   */
  getSourceFormat(node: ContentIOExportInput): string | null {
    const plugin = this.registry.getPlugin(node.contentFormat);
    return plugin?.canExport ? node.contentFormat : null;
  }

  /**
   * Export multiple content nodes as a single downloadable archive.
   * Each file is named based on the node's title/id.
   *
   * Note: For now, this downloads files sequentially.
   * Future: Could create a ZIP archive.
   */
  async downloadMultiple(nodes: ContentIOExportInput[], formatId?: string): Promise<void> {
    for (const node of nodes) {
      const targetFormat = formatId ?? node.contentFormat;
      if (this.registry.getPlugin(targetFormat)?.canExport) {
        await this.downloadAsFormat(node, targetFormat);
      }
    }
  }

  /**
   * Export content as a string (for programmatic use).
   * Useful for bulk retrieval operations.
   */
  async getExportedContent(
    node: ContentIOExportInput,
    formatId?: string
  ): Promise<{ content: string; filename: string; mimeType: string } | null> {
    const targetFormat = formatId ?? node.contentFormat;
    const plugin = this.registry.getPlugin(targetFormat);

    if (!plugin?.canExport) {
      return null;
    }

    const content = await this.exportToString(node, targetFormat);
    const extension = plugin.fileExtensions[0] ?? '';
    const filename = this.sanitizeFilename(node.title ?? node.id ?? 'content') + extension;
    const mimeType = plugin.mimeTypes[0] ?? 'text/plain';

    return { content, filename, mimeType };
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Utilities
  // ─────────────────────────────────────────────────────────────────────────────

  private downloadBlob(blob: Blob, filename: string): void {
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = filename;
    document.body.appendChild(link);
    link.click();
    link.remove();
    URL.revokeObjectURL(url);
  }

  private async blobToString(blob: Blob): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result as string);
      reader.onerror = () => reject(new Error(reader.error?.message ?? 'Failed to read blob'));
      reader.readAsText(blob);
    });
  }

  private sanitizeFilename(name: string): string {
    return name
      .replaceAll(/[^a-z0-9\s-]/gi, '')
      .replaceAll(/\s+/g, '-')
      .toLowerCase()
      .substring(0, 100);
  }
}
