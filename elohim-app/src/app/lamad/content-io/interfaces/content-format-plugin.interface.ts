/**
 * ContentFormatPlugin - Unified plugin for a content format.
 *
 * Each format provides its own:
 * - Renderer (how to display)
 * - I/O operations (import/export/validate)
 * - Editor (how to edit) - optional, falls back to default
 *
 * This keeps "what this is" and "how we create it" coupled.
 *
 * @example
 * ```typescript
 * class MarkdownFormatPlugin implements ContentFormatPlugin {
 *   readonly formatId = 'markdown';
 *   readonly canRender = true;
 *   readonly canEdit = false; // Uses default editor
 *
 *   getRendererComponent() { return MarkdownRendererComponent; }
 *   getEditorComponent() { return null; } // Falls back to default
 * }
 * ```
 */

import { Type, EventEmitter } from '@angular/core';

import { ContentNode } from '../../models/content-node.model';

import { ContentIOImportResult, ContentIOExportInput } from './content-io-plugin.interface';
import { FormatMetadata } from './format-metadata.interface';
import { ValidationResult } from './validation-result.interface';

// ─────────────────────────────────────────────────────────────────────────────
// Main Plugin Interface
// ─────────────────────────────────────────────────────────────────────────────

export interface ContentFormatPlugin {
  // ═══════════════════════════════════════════════════════════════════════════
  // Identity
  // ═══════════════════════════════════════════════════════════════════════════

  /** Unique identifier matching ContentFormat (e.g., 'markdown', 'gherkin', 'path') */
  readonly formatId: string;

  /** Human-readable display name (e.g., 'Markdown', 'Gherkin BDD') */
  readonly displayName: string;

  /** File extensions this plugin handles (e.g., ['.md', '.markdown']) */
  readonly fileExtensions: string[];

  /** MIME types this plugin handles */
  readonly mimeTypes: string[];

  // ═══════════════════════════════════════════════════════════════════════════
  // Capabilities
  // ═══════════════════════════════════════════════════════════════════════════

  /** Whether this plugin can import (parse) files */
  readonly canImport: boolean;

  /** Whether this plugin can export (serialize) content */
  readonly canExport: boolean;

  /** Whether this plugin can validate content */
  readonly canValidate: boolean;

  /** Whether this plugin provides a custom renderer */
  readonly canRender: boolean;

  /** Whether this plugin provides a custom editor */
  readonly canEdit: boolean;

  // ═══════════════════════════════════════════════════════════════════════════
  // I/O Operations
  // ═══════════════════════════════════════════════════════════════════════════

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

  // ═══════════════════════════════════════════════════════════════════════════
  // Rendering
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get the renderer component for this format.
   * @returns Angular component class that implements ContentRenderer, or null if no custom renderer
   */
  getRendererComponent(): Type<ContentRenderer> | null;

  /**
   * Get renderer priority for format conflicts.
   * Higher priority = checked first when multiple plugins match.
   * @returns Priority number (default 0)
   */
  getRendererPriority(): number;

  // ═══════════════════════════════════════════════════════════════════════════
  // Editing
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get the editor component for this format.
   * @returns Angular component class that implements ContentEditorComponent, or null to use default
   */
  getEditorComponent(): Type<ContentEditorComponent> | null;

  /**
   * Get editor configuration for this format.
   * @returns Editor config including mode, toolbar, etc.
   */
  getEditorConfig(): EditorConfig;

  // ═══════════════════════════════════════════════════════════════════════════
  // Format Detection & Metadata
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Detect if content matches this format.
   * @param content - Raw content string
   * @returns Confidence score 0-1, or null if not this format
   */
  detectFormat?(content: string): number | null;

  /**
   * Get full format metadata for UI display.
   */
  getFormatMetadata(): FormatMetadata;
}

// ─────────────────────────────────────────────────────────────────────────────
// Renderer Interface (from RendererRegistryService)
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Interface for content renderers.
 * Each renderer is an Angular component that can display a specific content format.
 */
export interface ContentRenderer {
  /** The content node to render - set via component input */
  node: ContentNode;
}

/**
 * Extended interface for interactive renderers that emit completion events.
 */
export interface InteractiveRenderer extends ContentRenderer {
  /** Emitted when the user completes an interactive element */
  complete: EventEmitter<RendererCompletionEvent>;
}

/**
 * Event emitted when a renderer completes an interactive action.
 */
export interface RendererCompletionEvent {
  type: 'quiz' | 'simulation' | 'video' | 'exercise';
  passed: boolean;
  score: number;
  details?: Record<string, unknown>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Editor Interfaces
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Base interface for editor components.
 *
 * Editor components receive a ContentNode and allow users to modify it.
 * They emit events for save/cancel and track dirty state.
 */
export interface ContentEditorComponent {
  /** The content node being edited */
  node: ContentNode;

  /** Whether the editor is in read-only mode */
  readonly: boolean;

  /** Emitted when content changes (for autosave, preview update) */
  contentChange: EventEmitter<ContentChangeEvent>;

  /** Emitted when user wants to save */
  save: EventEmitter<ContentSaveEvent>;

  /** Emitted when user cancels editing */
  cancel: EventEmitter<void>;

  /**
   * Validate current content before save.
   * @returns Validation result with errors/warnings
   */
  validate(): Promise<EditorValidationResult>;

  /**
   * Get current content for export/save.
   * @returns Content in export format
   */
  getContent(): ContentIOExportInput;

  /**
   * Check if there are unsaved changes.
   * Used for navigation guards.
   */
  hasUnsavedChanges(): boolean;
}

/**
 * Event emitted when content changes in the editor.
 */
export interface ContentChangeEvent {
  /** Current content */
  content: ContentIOExportInput;

  /** Type of change */
  changeType: 'content' | 'metadata' | 'both';

  /** Unix timestamp of change */
  timestamp: number;
}

/**
 * Event emitted when user saves content.
 */
export interface ContentSaveEvent {
  /** Content to save */
  content: ContentIOExportInput;

  /** Optional commit message for versioned content */
  commitMessage?: string;
}

/**
 * Validation result from editor.
 */
export interface EditorValidationResult {
  /** Whether content is valid */
  valid: boolean;

  /** Validation errors */
  errors: EditorValidationError[];

  /** Validation warnings */
  warnings: EditorValidationWarning[];
}

export interface EditorValidationError {
  field: string;
  message: string;
  code?: string;
}

export interface EditorValidationWarning {
  field: string;
  message: string;
  suggestion?: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Editor Configuration
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Configuration for an editor component.
 */
export interface EditorConfig {
  /** Editor mode: visual (WYSIWYG), code (text), or hybrid (side-by-side) */
  editorMode: 'visual' | 'code' | 'hybrid';

  /** Whether the editor supports live preview */
  supportsLivePreview: boolean;

  /** Toolbar configuration */
  toolbar?: ToolbarConfig;

  /** Auto-save interval in milliseconds (0 = disabled) */
  autoSaveIntervalMs?: number;

  /** Show line numbers (for code mode) */
  showLineNumbers?: boolean;

  /** Wrap long lines */
  wordWrap?: boolean;

  /** Editor theme */
  theme?: 'light' | 'dark' | 'system';

  /** Minimum height in pixels */
  minHeight?: number;

  /** Maximum height in pixels (0 = unlimited) */
  maxHeight?: number;
}

/**
 * Toolbar configuration for editors.
 */
export interface ToolbarConfig {
  /** Show the toolbar */
  enabled: boolean;

  /** Toolbar position */
  position?: 'top' | 'bottom' | 'floating';

  /** Available actions */
  actions?: ToolbarAction[];
}

/**
 * A toolbar action (button/dropdown).
 */
export interface ToolbarAction {
  /** Action identifier */
  id: string;

  /** Display label */
  label: string;

  /** Material icon name */
  icon?: string;

  /** Keyboard shortcut (e.g., 'Ctrl+B') */
  shortcut?: string;

  /** Action type */
  type: 'button' | 'dropdown' | 'toggle';

  /** For dropdowns: sub-actions */
  children?: ToolbarAction[];
}

// ─────────────────────────────────────────────────────────────────────────────
// Default Editor Config
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Default editor configuration used when a plugin doesn't provide one.
 */
export const DEFAULT_EDITOR_CONFIG: EditorConfig = {
  editorMode: 'code',
  supportsLivePreview: false,
  showLineNumbers: true,
  wordWrap: true,
  theme: 'system',
  autoSaveIntervalMs: 0,
  minHeight: 300,
  maxHeight: 0,
  toolbar: {
    enabled: true,
    position: 'top',
    actions: [
      { id: 'save', label: 'Save', icon: 'save', shortcut: 'Ctrl+S', type: 'button' },
      { id: 'cancel', label: 'Cancel', icon: 'close', shortcut: 'Escape', type: 'button' },
    ],
  },
};

// ─────────────────────────────────────────────────────────────────────────────
// Abstract Base Class
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Abstract base class for ContentFormatPlugin implementations.
 *
 * Provides default implementations for optional methods.
 * Subclasses must implement the abstract methods.
 *
 * @example
 * ```typescript
 * class MarkdownFormatPlugin extends BaseContentFormatPlugin {
 *   readonly formatId = 'markdown';
 *   readonly displayName = 'Markdown';
 *   // ... implement abstract methods
 * }
 * ```
 */
export abstract class BaseContentFormatPlugin implements ContentFormatPlugin {
  // Abstract properties - must be defined by subclass
  abstract readonly formatId: string;
  abstract readonly displayName: string;
  abstract readonly fileExtensions: string[];
  abstract readonly mimeTypes: string[];

  // Default capabilities - can be overridden
  readonly canImport = true;
  readonly canExport = true;
  readonly canValidate = true;
  readonly canRender = true;
  readonly canEdit = false; // Most plugins use default editor

  // Abstract I/O methods - must be implemented
  abstract import(input: string | File): Promise<ContentIOImportResult>;
  abstract export(node: ContentIOExportInput): Promise<string | Blob>;
  abstract validate(input: string | File): Promise<ValidationResult>;

  // Abstract renderer method - must return component or null
  abstract getRendererComponent(): Type<ContentRenderer> | null;

  // Default implementations
  getRendererPriority(): number {
    return 0;
  }

  getEditorComponent(): Type<ContentEditorComponent> | null {
    return null; // Use default editor
  }

  getEditorConfig(): EditorConfig {
    return DEFAULT_EDITOR_CONFIG;
  }

  detectFormat?(content: string): number | null;

  abstract getFormatMetadata(): FormatMetadata;
}
