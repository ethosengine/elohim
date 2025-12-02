import { Injectable } from '@angular/core';
import {
  ContentIOPlugin,
  ContentIOExportInput
} from '../interfaces/content-io-plugin.interface';
import { FormatMetadata } from '../interfaces/format-metadata.interface';

/**
 * Central registry for content I/O plugins.
 *
 * Plugins self-register when their modules are loaded, and the registry
 * provides discovery methods for finding appropriate plugins based on
 * format, file extension, or MIME type.
 *
 * NOTE: Rendering is handled separately by the existing RendererRegistryService.
 * This registry focuses only on import/export/validate operations.
 */
@Injectable({
  providedIn: 'root'
})
export class ContentIORegistryService {
  private readonly plugins = new Map<string, ContentIOPlugin>();
  private readonly extensionMap = new Map<string, string[]>(); // extension → formatIds
  private readonly mimeTypeMap = new Map<string, string[]>(); // mimeType → formatIds

  // ─────────────────────────────────────────────────────────────────────────────
  // Plugin Registration
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Register a plugin with the registry.
   * Called by plugin modules during initialization.
   */
  register(plugin: ContentIOPlugin): void {
    const formatId = plugin.formatId;

    this.plugins.set(formatId, plugin);

    // Index by file extensions
    for (const ext of plugin.fileExtensions) {
      const normalizedExt = ext.toLowerCase().startsWith('.') ? ext.toLowerCase() : `.${ext.toLowerCase()}`;
      const existing = this.extensionMap.get(normalizedExt) ?? [];
      if (!existing.includes(formatId)) {
        this.extensionMap.set(normalizedExt, [...existing, formatId]);
      }
    }

    // Index by MIME types
    for (const mime of plugin.mimeTypes) {
      const normalizedMime = mime.toLowerCase();
      const existing = this.mimeTypeMap.get(normalizedMime) ?? [];
      if (!existing.includes(formatId)) {
        this.mimeTypeMap.set(normalizedMime, [...existing, formatId]);
      }
    }
  }

  /**
   * Unregister a plugin from the registry.
   */
  unregister(formatId: string): void {
    const plugin = this.plugins.get(formatId);
    if (!plugin) {
      return;
    }

    // Remove from extension map
    for (const ext of plugin.fileExtensions) {
      const normalizedExt = ext.toLowerCase().startsWith('.') ? ext.toLowerCase() : `.${ext.toLowerCase()}`;
      const existing = this.extensionMap.get(normalizedExt) ?? [];
      this.extensionMap.set(normalizedExt, existing.filter(id => id !== formatId));
    }

    // Remove from MIME type map
    for (const mime of plugin.mimeTypes) {
      const normalizedMime = mime.toLowerCase();
      const existing = this.mimeTypeMap.get(normalizedMime) ?? [];
      this.mimeTypeMap.set(normalizedMime, existing.filter(id => id !== formatId));
    }

    this.plugins.delete(formatId);
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Plugin Discovery
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Get a plugin by format ID.
   */
  getPlugin(formatId: string): ContentIOPlugin | undefined {
    return this.plugins.get(formatId);
  }

  /**
   * Get all registered plugins.
   */
  getAllPlugins(): ContentIOPlugin[] {
    return Array.from(this.plugins.values());
  }

  /**
   * Get all registered format IDs.
   */
  getRegisteredFormats(): string[] {
    return Array.from(this.plugins.keys());
  }

  /**
   * Get plugins that can handle a specific file extension.
   */
  getPluginsForExtension(extension: string): ContentIOPlugin[] {
    const normalizedExt = extension.toLowerCase().startsWith('.')
      ? extension.toLowerCase()
      : `.${extension.toLowerCase()}`;

    const formatIds = this.extensionMap.get(normalizedExt) ?? [];
    return formatIds
      .map(id => this.plugins.get(id))
      .filter((p): p is ContentIOPlugin => p !== undefined);
  }

  /**
   * Get plugins that can handle a specific MIME type.
   */
  getPluginsForMimeType(mimeType: string): ContentIOPlugin[] {
    const normalizedMime = mimeType.toLowerCase();
    const formatIds = this.mimeTypeMap.get(normalizedMime) ?? [];
    return formatIds
      .map(id => this.plugins.get(id))
      .filter((p): p is ContentIOPlugin => p !== undefined);
  }

  /**
   * Get metadata for all formats that support import.
   */
  getImportableFormats(): FormatMetadata[] {
    return this.getAllPlugins()
      .filter(p => p.canImport)
      .map(p => p.getFormatMetadata())
      .sort((a, b) => (b.priority ?? 0) - (a.priority ?? 0));
  }

  /**
   * Get metadata for all formats that support export.
   */
  getExportableFormats(): FormatMetadata[] {
    return this.getAllPlugins()
      .filter(p => p.canExport)
      .map(p => p.getFormatMetadata())
      .sort((a, b) => (b.priority ?? 0) - (a.priority ?? 0));
  }

  /**
   * Get exportable formats for a specific content node.
   * Filters based on what formats can actually export this content.
   */
  getExportableFormatsForContent(node: ContentIOExportInput): FormatMetadata[] {
    // For now, return all exportable formats
    // In future, could filter based on contentFormat compatibility
    return this.getExportableFormats();
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Format Detection
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Detect the format of a file based on extension and MIME type.
   */
  async detectFormat(file: File): Promise<string | null> {
    // Try by extension first
    const ext = this.getFileExtension(file.name);
    if (ext) {
      const plugins = this.getPluginsForExtension(ext);
      if (plugins.length === 1) {
        return plugins[0].formatId;
      }
      if (plugins.length > 1) {
        // Multiple plugins, try content detection
        const content = await this.readFileAsText(file);
        return this.detectFormatFromContent(content, plugins);
      }
    }

    // Try by MIME type
    if (file.type) {
      const plugins = this.getPluginsForMimeType(file.type);
      if (plugins.length === 1) {
        return plugins[0].formatId;
      }
      if (plugins.length > 1) {
        const content = await this.readFileAsText(file);
        return this.detectFormatFromContent(content, plugins);
      }
    }

    // Try content detection on all plugins
    const content = await this.readFileAsText(file);
    return this.detectFormatFromContent(content);
  }

  /**
   * Detect format from content string using plugin detection methods.
   */
  detectFormatFromContent(content: string, candidates?: ContentIOPlugin[]): string | null {
    const plugins = candidates || this.getAllPlugins();

    let bestMatch: { formatId: string; confidence: number } | null = null;

    for (const plugin of plugins) {
      if (plugin.detectFormat) {
        const confidence = plugin.detectFormat(content);
        if (confidence !== null && confidence > 0) {
          if (!bestMatch || confidence > bestMatch.confidence) {
            bestMatch = { formatId: plugin.formatId, confidence };
          }
        }
      }
    }

    return bestMatch?.formatId ?? null;
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Utilities
  // ─────────────────────────────────────────────────────────────────────────────

  private getFileExtension(filename: string): string | null {
    const match = /\.[^.]+$/.exec(filename);
    return match ? match[0].toLowerCase() : null;
  }

  private readFileAsText(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result as string);
      reader.onerror = () => reject(new Error(reader.error?.message ?? 'Failed to read file'));
      reader.readAsText(file);
    });
  }
}
