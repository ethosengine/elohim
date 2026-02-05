import { Injectable } from '@angular/core';

// @coverage: 93.1% (2026-02-05)

import { Observable, of, throwError } from 'rxjs';

import { ContentNode } from '../../models/content-node.model';
import { ContentIOExportInput } from '../interfaces/content-io-plugin.interface';
import { ValidationResult } from '../interfaces/validation-result.interface';

import { ContentFormatRegistryService } from './content-format-registry.service';

/**
 * ContentEditorService - High-level content editing operations.
 *
 * This service provides a facade over the ContentFormatRegistry for editing:
 * - Create new content nodes
 * - Update existing content
 * - Validate content before save
 * - Check edit permissions
 *
 * Holochain Integration:
 * In the prototype phase, this service works with in-memory/localStorage drafts.
 * In production, save operations will commit entries to the DHT.
 *
 * The service doesn't handle UI concerns (that's the editor component's job),
 * it focuses on business logic around content editing.
 */
@Injectable({
  providedIn: 'root',
})
export class ContentEditorService {
  /** Draft storage for unsaved changes (prototype) */
  private readonly drafts = new Map<string, ContentDraft>();

  constructor(private readonly registry: ContentFormatRegistryService) {}

  // ═══════════════════════════════════════════════════════════════════════════
  // Permissions
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Check if a user can edit a content node.
   *
   * In the prototype, we allow editing all content.
   * In production, this would check:
   * - User is the author
   * - User has appropriate permissions
   * - Content is not locked for governance review
   *
   * @param node - The content node to check
   * @param _currentUserId - Optional user ID (for future auth)
   * @returns Whether the user can edit this content
   */
  canEdit(node: ContentNode | null, _currentUserId?: string): boolean {
    if (!node) {
      return false;
    }

    // In prototype, all content is editable
    // Future: Implement proper permission checks when auth is added

    // Don't allow editing dynamically generated step content
    // These have IDs like "path-{pathId}-step-{index}" and are generated at runtime
    // Actual LearningPath nodes (like "path-elohim-protocol") ARE editable
    if (/^path-.*-step-\d+$/.test(node.id)) {
      // Step content is dynamically generated from LearningPath data
      // To edit, user should edit the path or the referenced content directly
      return false;
    }

    // Check if we have a format plugin that can handle this content
    const plugin = this.registry.getPlugin(node.contentFormat);
    if (!plugin) {
      // No plugin means we can't properly validate/export changes
      // Still allow editing via default editor
    }

    return true;
  }

  /**
   * Check if a format supports editing.
   */
  canEditFormat(formatId: string): boolean {
    const editor = this.registry.getEditorComponent(formatId);
    return editor !== null;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Draft Management
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Create a draft from an existing content node.
   *
   * @param node - The content node to create a draft from
   * @returns The created draft
   */
  createDraft(node: ContentNode): ContentDraft {
    const draft: ContentDraft = {
      id: this.generateDraftId(),
      originalNodeId: node.id,
      content: this.nodeToExportInput(node),
      createdAt: Date.now(),
      updatedAt: Date.now(),
      isDirty: false,
    };

    this.drafts.set(draft.id, draft);
    return draft;
  }

  /**
   * Create a draft for new content.
   *
   * @param formatId - The format for the new content
   * @param initialData - Optional initial data
   * @returns The created draft
   */
  createNewDraft(formatId: string, initialData?: Partial<ContentIOExportInput>): ContentDraft {
    const draft: ContentDraft = {
      id: this.generateDraftId(),
      originalNodeId: null,
      content: {
        title: initialData?.title ?? 'Untitled',
        content: initialData?.content ?? '',
        contentFormat: formatId,
        contentType: initialData?.contentType ?? 'concept',
        description: initialData?.description ?? '',
        tags: initialData?.tags ?? [],
        metadata: initialData?.metadata ?? {},
      },
      createdAt: Date.now(),
      updatedAt: Date.now(),
      isDirty: true,
    };

    this.drafts.set(draft.id, draft);
    return draft;
  }

  /**
   * Get a draft by ID.
   */
  getDraft(draftId: string): ContentDraft | undefined {
    return this.drafts.get(draftId);
  }

  /**
   * Update a draft with new content.
   */
  updateDraft(draftId: string, changes: Partial<ContentIOExportInput>): ContentDraft | undefined {
    const draft = this.drafts.get(draftId);
    if (!draft) {
      return undefined;
    }

    draft.content = { ...draft.content, ...changes };
    draft.updatedAt = Date.now();
    draft.isDirty = true;

    return draft;
  }

  /**
   * Delete a draft.
   */
  deleteDraft(draftId: string): boolean {
    return this.drafts.delete(draftId);
  }

  /**
   * Get all drafts.
   */
  getAllDrafts(): ContentDraft[] {
    return Array.from(this.drafts.values());
  }

  /**
   * Check if a node has an unsaved draft.
   */
  hasDraft(nodeId: string): boolean {
    for (const draft of this.drafts.values()) {
      if (draft.originalNodeId === nodeId && draft.isDirty) {
        return true;
      }
    }
    return false;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Content Operations
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Validate content using the format plugin.
   *
   * @param formatId - The format to validate against
   * @param content - The content to validate
   * @returns Validation result
   */
  async validateContent(formatId: string, content: string | object): Promise<ValidationResult> {
    const plugin = this.registry.getPlugin(formatId);

    if (!plugin) {
      // No plugin - basic validation only
      return {
        valid: true,
        errors: [],
        warnings: [
          {
            code: 'NO_PLUGIN',
            message: `No format plugin for "${formatId}". Content may not render correctly.`,
            suggestion: 'Consider using a supported format like markdown or gherkin.',
          },
        ],
      };
    }

    if (!plugin.canValidate) {
      return { valid: true, errors: [], warnings: [] };
    }

    const contentStr = typeof content === 'string' ? content : JSON.stringify(content, null, 2);
    return plugin.validate(contentStr);
  }

  /**
   * Save content (prototype: stores in draft, future: commits to Holochain).
   *
   * @param draftId - The draft to save
   * @returns Save result
   */
  saveContent(draftId: string): Observable<SaveResult> {
    const draft = this.drafts.get(draftId);

    if (!draft) {
      return throwError(() => new Error(`Draft not found: ${draftId}`));
    }

    // In prototype phase, we can't actually persist to files
    // This would be where we call HolochainService in production
    // For now, mark as saved and return success

    draft.isDirty = false;
    draft.updatedAt = Date.now();

    // Future: In production, this would:
    // 1. Validate content
    // 2. Call HolochainService.createEntry() or updateEntry()
    // 3. Update cache in DataLoaderService
    // 4. Return the new/updated ContentNode

    return of({
      success: true,
      nodeId: draft.originalNodeId ?? this.generateNodeId(draft.content.title),
      message: 'Content saved to draft. Holochain persistence not yet implemented.',
      draft,
    });
  }

  /**
   * Export content to a string in its native format.
   *
   * @param content - The content to export
   * @returns Exported content string
   */
  async exportContent(content: ContentIOExportInput): Promise<string | Blob> {
    const plugin = this.registry.getPlugin(content.contentFormat);

    if (!plugin) {
      // No plugin - return content as-is or JSON
      if (typeof content.content === 'string') {
        return content.content;
      }
      return JSON.stringify(content.content, null, 2);
    }

    return plugin.export(content);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Utilities
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Convert a ContentNode to ContentIOExportInput.
   */
  private nodeToExportInput(node: ContentNode): ContentIOExportInput {
    return {
      id: node.id,
      title: node.title,
      description: node.description,
      content: node.content,
      contentFormat: node.contentFormat,
      contentType: node.contentType,
      tags: [...node.tags],
      metadata: { ...node.metadata },
      relatedNodeIds: [...node.relatedNodeIds],
    };
  }

  /**
   * Generate a unique draft ID.
   * Uses crypto.randomUUID for secure random IDs when available.
   */
  private generateDraftId(): string {
    // Use crypto.randomUUID if available (modern browsers)
    if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
      // Remove dashes and take first 8 chars for clean alphanumeric ID
      return `draft-${Date.now()}-${crypto.randomUUID().replaceAll('-', '').substring(0, 8)}`;
    }
    // Fallback: use crypto.getRandomValues
    const array = new Uint32Array(1);
    crypto.getRandomValues(array);
    return `draft-${Date.now()}-${array[0].toString(36).substring(0, 8)}`;
  }

  /**
   * Generate a node ID from title (for new content).
   */
  private generateNodeId(title: string): string {
    return title
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/(?:^-|-$)/g, '')
      .substring(0, 50);
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * A draft represents unsaved changes to content.
 */
export interface ContentDraft {
  /** Unique draft ID */
  id: string;

  /** Original node ID (null for new content) */
  originalNodeId: string | null;

  /** Current content state */
  content: ContentIOExportInput;

  /** When draft was created */
  createdAt: number;

  /** When draft was last updated */
  updatedAt: number;

  /** Whether there are unsaved changes */
  isDirty: boolean;
}

/**
 * Result of a save operation.
 */
export interface SaveResult {
  /** Whether save succeeded */
  success: boolean;

  /** The node ID (new or existing) */
  nodeId: string;

  /** Human-readable message */
  message: string;

  /** The saved draft */
  draft?: ContentDraft;

  /** Error if save failed */
  error?: string;
}

/**
 * Parameters for creating a new content node.
 */
export interface CreateNodeParams {
  /** Content format */
  formatId: string;

  /** Initial title */
  title: string;

  /** Initial content */
  content?: string | object;

  /** Content type */
  contentType?: string;

  /** Description */
  description?: string;

  /** Tags */
  tags?: string[];

  /** Additional metadata */
  metadata?: Record<string, unknown>;
}
