import { CommonModule } from '@angular/common';
import {
  Component,
  OnInit,
  OnDestroy,
  ViewChild,
  ViewContainerRef,
  Type,
  ComponentRef,
} from '@angular/core';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';

import { takeUntil, switchMap, catchError } from 'rxjs/operators';

import { Subject, of } from 'rxjs';

import { DataLoaderService } from '@app/elohim/services/data-loader.service';

import { DefaultCodeEditorComponent } from '../../content-io/components/default-code-editor';
import {
  ContentEditorComponent,
  ContentSaveEvent,
} from '../../content-io/interfaces/content-format-plugin.interface';
import { ContentEditorService, SaveResult } from '../../content-io/services/content-editor.service';
import { ContentFormatRegistryService } from '../../content-io/services/content-format-registry.service';
import { ContentNode } from '../../models/content-node.model';

/**
 * ContentEditorPageComponent - Standalone page for editing content.
 *
 * Route: /lamad/resource/:id/edit
 *
 * This component:
 * 1. Loads the content node by ID
 * 2. Checks if user can edit
 * 3. Dynamically loads the appropriate editor component (specialized or default)
 * 4. Handles save/cancel events
 * 5. Manages navigation and unsaved changes
 *
 * The actual editing is delegated to the editor component, which implements
 * ContentEditorComponent interface.
 */
@Component({
  selector: 'app-content-editor-page',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './content-editor-page.component.html',
  styleUrls: ['./content-editor-page.component.css'],
})
export class ContentEditorPageComponent implements OnInit, OnDestroy {
  /** The content node being edited */
  node: ContentNode | null = null;

  /** Resource ID from route */
  resourceId = '';

  /** Loading state */
  isLoading = true;

  /** Error message if loading failed */
  error = '';

  /** Whether current user can edit this content */
  canEdit = false;

  /** Save in progress */
  isSaving = false;

  /** Save error message */
  saveError = '';

  /** Save success message */
  saveSuccess = '';

  @ViewChild('editorHost', { read: ViewContainerRef, static: true })
  editorHost!: ViewContainerRef;

  private editorRef: ComponentRef<ContentEditorComponent> | null = null;
  private readonly destroy$ = new Subject<void>();

  constructor(
    private readonly route: ActivatedRoute,
    private readonly router: Router,
    private readonly dataLoader: DataLoaderService,
    private readonly registry: ContentFormatRegistryService,
    private readonly editorService: ContentEditorService
  ) {}

  ngOnInit(): void {
    this.route.params
      .pipe(
        takeUntil(this.destroy$),
        switchMap(params => {
          this.resourceId = params['resourceId'];
          this.isLoading = true;
          this.error = '';
          return this.dataLoader.getContent(this.resourceId).pipe(
            catchError(err => {
              this.error = err.message ?? 'Failed to load content';
              return of(null);
            })
          );
        })
      )
      .subscribe(node => {
        this.node = node;
        this.isLoading = false;

        if (node) {
          this.canEdit = this.editorService.canEdit(node);
          if (this.canEdit) {
            this.loadEditor();
          }
        }
      });
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
    this.cleanupEditor();
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Navigation
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Navigate back to the resource view.
   */
  navigateBack(): void {
    if (this.hasUnsavedChanges()) {
      // Could show confirmation dialog here
      // For now, just navigate
    }
    this.router.navigate(['/lamad/resource', this.resourceId]);
  }

  /**
   * Check for unsaved changes.
   */
  hasUnsavedChanges(): boolean {
    if (this.editorRef?.instance) {
      return this.editorRef.instance.hasUnsavedChanges();
    }
    return false;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Editor Management
  // ═══════════════════════════════════════════════════════════════════════════

  private loadEditor(): void {
    if (!this.node) return;

    this.cleanupEditor();

    // Get the editor component for this format
    const formatId = this.node.contentFormat;
    let EditorComponent: Type<ContentEditorComponent> | null =
      this.registry.getEditorComponent(formatId);

    // Fall back to default if no specialized editor
    EditorComponent ??= DefaultCodeEditorComponent;

    // Create the editor component
    this.editorRef = this.editorHost.createComponent(EditorComponent);

    // Set inputs
    this.editorRef.setInput('node', this.node);
    this.editorRef.setInput('readonly', false);

    // Get editor config from plugin if available
    const config = this.registry.getEditorConfig(formatId);
    if ('config' in this.editorRef.instance) {
      this.editorRef.setInput('config', config);
    }

    // Subscribe to events
    this.editorRef.instance.save.subscribe((event: ContentSaveEvent) => {
      this.handleSave(event);
    });

    this.editorRef.instance.cancel.subscribe(() => {
      this.navigateBack();
    });
  }

  private cleanupEditor(): void {
    if (this.editorRef) {
      this.editorRef.destroy();
      this.editorRef = null;
    }
    this.editorHost?.clear();
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Save Handling
  // ═══════════════════════════════════════════════════════════════════════════

  private async handleSave(event: ContentSaveEvent): Promise<void> {
    this.isSaving = true;
    this.saveError = '';
    this.saveSuccess = '';

    try {
      // Create a draft and save it
      const draft = this.editorService.createNewDraft(event.content.contentFormat, event.content);

      this.editorService.saveContent(draft.id).subscribe({
        next: (result: SaveResult) => {
          this.isSaving = false;
          if (result.success) {
            this.saveSuccess = result.message;
            // Mark editor as saved
            if (this.editorRef?.instance && 'markSaved' in this.editorRef.instance) {
              (this.editorRef.instance as DefaultCodeEditorComponent).markSaved();
            }
            // Clear success message after a delay
            setTimeout(() => {
              this.saveSuccess = '';
            }, 3000);
          } else {
            this.saveError = result.error ?? 'Failed to save';
          }
        },
        error: (err: Error) => {
          this.isSaving = false;
          this.saveError = err.message ?? 'Failed to save';
        },
      });
    } catch (err) {
      this.isSaving = false;
      this.saveError = (err as Error).message ?? 'Failed to save';
    }
  }
}
