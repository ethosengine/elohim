import { Injectable, Type } from '@angular/core';

// @coverage: 89.1% (2026-01-31)

import { ContentNode } from '../../models/content-node.model';
import {
  ContentFormatPlugin,
  ContentRenderer,
  ContentEditorComponent,
  EditorConfig,
  DEFAULT_EDITOR_CONFIG,
} from '../interfaces/content-format-plugin.interface';
import { FormatMetadata } from '../interfaces/format-metadata.interface';

/**
 * ContentFormatRegistryService - Unified registry for content format plugins.
 *
 * Replaces both ContentIORegistryService and RendererRegistryService by combining
 * I/O, rendering, and editing capabilities into a single plugin-based registry.
 *
 * Each plugin provides:
 * - Import/export/validate operations (I/O)
 * - Renderer component
 * - Editor component (optional - falls back to default)
 *
 * @example
 * ```typescript
 * // Register a plugin
 * registry.register(new MarkdownFormatPlugin());
 *
 * // Get renderer for a node
 * const renderer = registry.getRendererComponent(node.contentFormat);
 *
 * // Get editor for a format
 * const editor = registry.getEditorComponent('markdown');
 * ```
 */
@Injectable({
  providedIn: 'root',
})
export class ContentFormatRegistryService {
  private readonly plugins = new Map<string, ContentFormatPlugin>();
  private readonly extensionMap = new Map<string, string[]>(); // extension → formatIds
  private readonly mimeTypeMap = new Map<string, string[]>(); // mimeType → formatIds
  private readonly formatAliases = new Map<string, string>(); // alias → canonical formatId

  /** Default editor component to use when plugin doesn't provide one */
  private defaultEditorComponent: Type<ContentEditorComponent> | null = null;

  // ═══════════════════════════════════════════════════════════════════════════
  // Plugin Registration
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Register a plugin with the registry.
   *
   * @param plugin - The ContentFormatPlugin to register
   */
  register(plugin: ContentFormatPlugin): void {
    const formatId = plugin.formatId;

    this.plugins.set(formatId, plugin);

    // Index by file extensions
    for (const ext of plugin.fileExtensions) {
      const normalizedExt = this.normalizeExtension(ext);
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
   * Register the default editor component.
   *
   * This editor is used when a plugin doesn't provide its own editor.
   *
   * @param component - The default editor component class
   */
  registerDefaultEditor(component: Type<ContentEditorComponent>): void {
    this.defaultEditorComponent = component;
  }

  /**
   * Register a format alias.
   *
   * Allows multiple format IDs to resolve to the same plugin.
   * For example: 'quiz-json' → 'perseus' means quiz-json content uses Perseus renderer.
   *
   * @param alias - The alias format ID
   * @param canonicalFormatId - The actual plugin format ID to use
   */
  registerAlias(alias: string, canonicalFormatId: string): void {
    if (!this.plugins.has(canonicalFormatId)) {
      return;
    }
    this.formatAliases.set(alias, canonicalFormatId);
  }

  /**
   * Resolve a format ID to its canonical form (following aliases).
   *
   * @param formatId - The format ID to resolve
   * @returns The canonical format ID (same as input if no alias exists)
   */
  resolveFormat(formatId: string): string {
    return this.formatAliases.get(formatId) ?? formatId;
  }

  /**
   * Unregister a plugin from the registry.
   *
   * @param formatId - The format ID to unregister
   */
  unregister(formatId: string): void {
    const plugin = this.plugins.get(formatId);
    if (!plugin) {
      return;
    }

    // Remove from extension map
    for (const ext of plugin.fileExtensions) {
      const normalizedExt = this.normalizeExtension(ext);
      const existing = this.extensionMap.get(normalizedExt) ?? [];
      this.extensionMap.set(
        normalizedExt,
        existing.filter(id => id !== formatId)
      );
    }

    // Remove from MIME type map
    for (const mime of plugin.mimeTypes) {
      const normalizedMime = mime.toLowerCase();
      const existing = this.mimeTypeMap.get(normalizedMime) ?? [];
      this.mimeTypeMap.set(
        normalizedMime,
        existing.filter(id => id !== formatId)
      );
    }

    this.plugins.delete(formatId);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Plugin Lookup
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get a plugin by format ID (resolves aliases automatically).
   */
  getPlugin(formatId: string): ContentFormatPlugin | undefined {
    const resolved = this.resolveFormat(formatId);
    return this.plugins.get(resolved);
  }

  /**
   * Get all registered plugins.
   */
  getAllPlugins(): ContentFormatPlugin[] {
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
  getPluginsForExtension(extension: string): ContentFormatPlugin[] {
    const normalizedExt = this.normalizeExtension(extension);
    const formatIds = this.extensionMap.get(normalizedExt) ?? [];
    return formatIds
      .map(id => this.plugins.get(id))
      .filter((p): p is ContentFormatPlugin => p !== undefined);
  }

  /**
   * Get plugins that can handle a specific MIME type.
   */
  getPluginsForMimeType(mimeType: string): ContentFormatPlugin[] {
    const normalizedMime = mimeType.toLowerCase();
    const formatIds = this.mimeTypeMap.get(normalizedMime) ?? [];
    return formatIds
      .map(id => this.plugins.get(id))
      .filter((p): p is ContentFormatPlugin => p !== undefined);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Renderer Access
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get the renderer component for a format (resolves aliases automatically).
   *
   * @param formatId - The content format ID
   * @returns Component class or null if no renderer available
   */
  getRendererComponent(formatId: string): Type<ContentRenderer> | null {
    const resolved = this.resolveFormat(formatId);
    const plugin = this.plugins.get(resolved);
    if (!plugin?.canRender) {
      return null;
    }
    return plugin.getRendererComponent();
  }

  /**
   * Get the appropriate renderer for a content node.
   *
   * @param node - The ContentNode to render
   * @returns Component class or null if no match
   */
  getRenderer(node: ContentNode): Type<ContentRenderer> | null {
    return this.getRendererComponent(node.contentFormat);
  }

  /**
   * Check if any renderer can handle this format (resolves aliases automatically).
   */
  canRender(format: string): boolean {
    const resolved = this.resolveFormat(format);
    const plugin = this.plugins.get(resolved);
    return plugin?.canRender === true && plugin.getRendererComponent() !== null;
  }

  /**
   * Get all formats that have renderers (for debugging/info).
   */
  getRenderableFormats(): string[] {
    return this.getAllPlugins()
      .filter(p => p.canRender && p.getRendererComponent() !== null)
      .map(p => p.formatId);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Editor Access
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get the editor component for a format.
   *
   * If the plugin doesn't provide a custom editor, returns the default editor.
   *
   * @param formatId - The content format ID
   * @returns Component class (custom or default) or null if no editor available
   */
  getEditorComponent(formatId: string): Type<ContentEditorComponent> | null {
    const plugin = this.plugins.get(formatId);

    // Try plugin's custom editor first
    if (plugin?.canEdit) {
      const customEditor = plugin.getEditorComponent();
      if (customEditor) {
        return customEditor;
      }
    }

    // Fall back to default editor
    return this.defaultEditorComponent;
  }

  /**
   * Get editor configuration for a format.
   *
   * @param formatId - The content format ID
   * @returns Editor config from plugin or defaults
   */
  getEditorConfig(formatId: string): EditorConfig {
    const plugin = this.plugins.get(formatId);
    if (plugin) {
      return plugin.getEditorConfig();
    }
    return DEFAULT_EDITOR_CONFIG;
  }

  /**
   * Check if a format has a specialized (non-default) editor.
   */
  hasSpecializedEditor(formatId: string): boolean {
    const plugin = this.plugins.get(formatId);
    return plugin?.canEdit === true && plugin.getEditorComponent() !== null;
  }

  /**
   * Get all formats that support editing.
   */
  getEditableFormats(): string[] {
    // All formats can be edited via the default editor
    // This returns formats with ANY editor available
    if (this.defaultEditorComponent) {
      return this.getRegisteredFormats();
    }
    // Without default editor, only return formats with custom editors
    return this.getAllPlugins()
      .filter(p => p.canEdit && p.getEditorComponent() !== null)
      .map(p => p.formatId);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // I/O Access
  // ═══════════════════════════════════════════════════════════════════════════

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
   * Get exportable formats appropriate for a specific content node.
   * Returns all exportable formats (content-specific filtering can be added later).
   */
  getExportableFormatsForContent(_node: { contentFormat: string }): FormatMetadata[] {
    // For now, return all exportable formats
    // In the future, could filter based on content type compatibility
    return this.getExportableFormats();
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Format Detection
  // ═══════════════════════════════════════════════════════════════════════════

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
  detectFormatFromContent(content: string, candidates?: ContentFormatPlugin[]): string | null {
    const plugins = candidates ?? this.getAllPlugins();

    let bestMatch: { formatId: string; confidence: number } | null = null;

    for (const plugin of plugins) {
      if (plugin.detectFormat) {
        const confidence = plugin.detectFormat(content);
        if (
          confidence !== null &&
          confidence > 0 &&
          (!bestMatch || confidence > bestMatch.confidence)
        ) {
          bestMatch = { formatId: plugin.formatId, confidence };
        }
      }
    }

    return bestMatch?.formatId ?? null;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Registry Status
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Check if the registry has been initialized with plugins.
   */
  isInitialized(): boolean {
    return this.plugins.size > 0;
  }

  /**
   * Get registry statistics for debugging.
   */
  getStats(): RegistryStats {
    const plugins = this.getAllPlugins();
    return {
      totalPlugins: plugins.length,
      renderableFormats: plugins.filter(p => p.canRender).length,
      editableFormats: plugins.filter(p => p.canEdit).length,
      importableFormats: plugins.filter(p => p.canImport).length,
      exportableFormats: plugins.filter(p => p.canExport).length,
      hasDefaultEditor: this.defaultEditorComponent !== null,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Private Utilities
  // ═══════════════════════════════════════════════════════════════════════════

  private normalizeExtension(ext: string): string {
    return ext.toLowerCase().startsWith('.') ? ext.toLowerCase() : `.${ext.toLowerCase()}`;
  }

  private getFileExtension(filename: string): string | null {
    const match = /\.[^.]+$/.exec(filename);
    return match ? match[0].toLowerCase() : null;
  }

  private async readFileAsText(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result as string);
      reader.onerror = () => reject(new Error(reader.error?.message ?? 'Failed to read file'));
      reader.readAsText(file);
    });
  }
}

/**
 * Registry statistics for debugging/info.
 */
export interface RegistryStats {
  totalPlugins: number;
  renderableFormats: number;
  editableFormats: number;
  importableFormats: number;
  exportableFormats: number;
  hasDefaultEditor: boolean;
}
