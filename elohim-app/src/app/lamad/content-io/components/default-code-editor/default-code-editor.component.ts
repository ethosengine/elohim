import { CommonModule } from '@angular/common';
import {
  Component,
  Input,
  Output,
  EventEmitter,
  OnInit,
  OnDestroy,
  ViewChild,
  ElementRef,
} from '@angular/core';
import { FormsModule } from '@angular/forms';

// @coverage: 1.0% (2026-01-31)

import { debounceTime, takeUntil } from 'rxjs/operators';

import { Subject } from 'rxjs';

import { ContentNode } from '../../../models/content-node.model';
import {
  ContentEditorComponent,
  ContentChangeEvent,
  ContentSaveEvent,
  EditorValidationResult,
  EditorConfig,
  DEFAULT_EDITOR_CONFIG,
} from '../../interfaces/content-format-plugin.interface';
import { ContentIOExportInput } from '../../interfaces/content-io-plugin.interface';

/**
 * DefaultCodeEditorComponent - Basic code/text editor for all content formats.
 *
 * This editor is used when a format plugin doesn't provide a specialized editor.
 * It provides:
 * - Plain text editing with optional line numbers
 * - Metadata fields (title, description, tags)
 * - Save/Cancel buttons
 * - Dirty state tracking
 * - Basic validation
 *
 * Future enhancements:
 * - Syntax highlighting (via highlight.js or CodeMirror)
 * - Live preview pane
 * - Undo/redo stack
 * - Keyboard shortcuts
 */
@Component({
  selector: 'app-default-code-editor',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './default-code-editor.component.html',
  styleUrls: ['./default-code-editor.component.css'],
})
export class DefaultCodeEditorComponent implements ContentEditorComponent, OnInit, OnDestroy {
  // ═══════════════════════════════════════════════════════════════════════════
  // ContentEditorComponent Interface
  // ═══════════════════════════════════════════════════════════════════════════

  @Input() node!: ContentNode;
  @Input() readonly = false;
  @Input() config: EditorConfig = DEFAULT_EDITOR_CONFIG;

  @Output() contentChange = new EventEmitter<ContentChangeEvent>();
  @Output() save = new EventEmitter<ContentSaveEvent>();
  @Output() cancel = new EventEmitter<void>();

  // ═══════════════════════════════════════════════════════════════════════════
  // Internal State
  // ═══════════════════════════════════════════════════════════════════════════

  /** Current title */
  title = '';

  /** Current description */
  description = '';

  /** Current content body */
  content = '';

  /** Current tags (comma-separated in UI) */
  tagsInput = '';

  /** Whether content has been modified */
  isDirty = false;

  /** Line numbers array for display */
  lineNumbers: number[] = [];

  /** Validation errors to display */
  validationErrors: string[] = [];

  /** Whether currently saving */
  isSaving = false;

  /** Error message if save failed */
  saveError = '';

  @ViewChild('contentArea') contentArea!: ElementRef<HTMLTextAreaElement>;

  private readonly destroy$ = new Subject<void>();
  private readonly contentInput$ = new Subject<string>();
  private originalContent = '';
  private originalTitle = '';
  private originalDescription = '';
  private originalTags = '';

  // ═══════════════════════════════════════════════════════════════════════════
  // Lifecycle
  // ═══════════════════════════════════════════════════════════════════════════

  ngOnInit(): void {
    this.initializeFromNode();
    this.setupContentDebounce();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // ContentEditorComponent Methods
  // ═══════════════════════════════════════════════════════════════════════════

  async validate(): Promise<EditorValidationResult> {
    const errors: { field: string; message: string }[] = [];
    const warnings: { field: string; message: string }[] = [];

    // Required fields
    if (!this.title.trim()) {
      errors.push({ field: 'title', message: 'Title is required' });
    }

    if (!this.content.trim()) {
      errors.push({ field: 'content', message: 'Content is required' });
    }

    // Update UI
    this.validationErrors = errors.map(e => e.message);

    return await Promise.resolve({
      valid: errors.length === 0,
      errors,
      warnings,
    });
  }

  getContent(): ContentIOExportInput {
    return {
      id: this.node?.id,
      title: this.title.trim(),
      description: this.description.trim(),
      content: this.content,
      contentFormat: this.node?.contentFormat ?? 'markdown',
      contentType: this.node?.contentType,
      tags: this.parseTags(this.tagsInput),
      metadata: this.node?.metadata ?? {},
      relatedNodeIds: this.node?.relatedNodeIds ?? [],
    };
  }

  hasUnsavedChanges(): boolean {
    return this.isDirty;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // UI Event Handlers
  // ═══════════════════════════════════════════════════════════════════════════

  onContentInput(): void {
    this.updateLineNumbers();
    this.contentInput$.next(this.content);
    this.checkDirty();
  }

  onMetadataChange(): void {
    this.checkDirty();
    this.emitChange('metadata');
  }

  async onSave(): Promise<void> {
    this.saveError = '';
    const validation = await this.validate();

    if (!validation.valid) {
      return;
    }

    this.isSaving = true;

    // Emit save event - parent handles actual persistence
    this.save.emit({
      content: this.getContent(),
    });

    // Note: Parent should call markSaved() on success
    this.isSaving = false;
  }

  onCancel(): void {
    if (this.isDirty) {
      // Could show confirmation dialog here
      // For now, just emit cancel
    }
    this.cancel.emit();
  }

  onKeyDown(event: KeyboardEvent): void {
    // Handle keyboard shortcuts
    if (event.ctrlKey || event.metaKey) {
      if (event.key === 's') {
        event.preventDefault();
        void this.onSave();
      } else if (event.key === 'Escape') {
        event.preventDefault();
        this.onCancel();
      }
    }

    // Handle tab key for indentation
    if (event.key === 'Tab') {
      event.preventDefault();
      this.insertAtCursor('  '); // Two spaces
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Public Methods
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Mark content as saved (call after successful save).
   */
  markSaved(): void {
    this.isDirty = false;
    this.originalContent = this.content;
    this.originalTitle = this.title;
    this.originalDescription = this.description;
    this.originalTags = this.tagsInput;
  }

  /**
   * Reset to original content.
   */
  reset(): void {
    this.content = this.originalContent;
    this.title = this.originalTitle;
    this.description = this.originalDescription;
    this.tagsInput = this.originalTags;
    this.isDirty = false;
    this.updateLineNumbers();
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Private Methods
  // ═══════════════════════════════════════════════════════════════════════════

  private initializeFromNode(): void {
    if (this.node) {
      this.title = this.node.title ?? '';
      this.description = this.node.description ?? '';
      this.content =
        typeof this.node.content === 'string'
          ? this.node.content
          : JSON.stringify(this.node.content, null, 2);
      this.tagsInput = (this.node.tags ?? []).join(', ');

      // Store originals for dirty checking
      this.originalContent = this.content;
      this.originalTitle = this.title;
      this.originalDescription = this.description;
      this.originalTags = this.tagsInput;
    }
    this.updateLineNumbers();
  }

  private setupContentDebounce(): void {
    this.contentInput$.pipe(debounceTime(300), takeUntil(this.destroy$)).subscribe(() => {
      this.emitChange('content');
    });
  }

  private updateLineNumbers(): void {
    if (this.config.showLineNumbers) {
      const lines = this.content.split('\n').length;
      this.lineNumbers = Array.from({ length: lines }, (_, i) => i + 1);
    }
  }

  private checkDirty(): void {
    this.isDirty =
      this.content !== this.originalContent ||
      this.title !== this.originalTitle ||
      this.description !== this.originalDescription ||
      this.tagsInput !== this.originalTags;
  }

  private emitChange(changeType: 'content' | 'metadata' | 'both'): void {
    this.contentChange.emit({
      content: this.getContent(),
      changeType,
      timestamp: Date.now(),
    });
  }

  private parseTags(input: string): string[] {
    return input
      .split(',')
      .map(tag => tag.trim().toLowerCase().replace(/^#/, ''))
      .filter(tag => tag.length > 0);
  }

  private insertAtCursor(text: string): void {
    const textarea = this.contentArea?.nativeElement;
    if (!textarea) return;

    const start = textarea.selectionStart;
    const end = textarea.selectionEnd;

    this.content = this.content.substring(0, start) + text + this.content.substring(end);

    // Restore cursor position
    setTimeout(() => {
      textarea.selectionStart = textarea.selectionEnd = start + text.length;
      textarea.focus();
    });

    this.onContentInput();
  }
}
