import { CommonModule } from '@angular/common';
import { Component, Input } from '@angular/core';

// @coverage: 46.7% (2026-02-04)

import { ContentNode } from '../../../models/content-node.model';
import { ContentFormatRegistryService } from '../../services/content-format-registry.service';
import { ContentIOService } from '../../services/content-io.service';

/**
 * Simple download button for content.
 *
 * Downloads content in its source format (contentFormat field).
 * Only visible if a plugin exists that can export this format.
 *
 * Usage:
 * <app-content-download [node]="contentNode"></app-content-download>
 */
@Component({
  selector: 'app-content-download',
  standalone: true,
  imports: [CommonModule],
  template: `
    <button
      *ngIf="canDownload"
      class="download-btn"
      (click)="download()"
      [disabled]="downloading"
      [title]="'Download as ' + formatLabel"
    >
      <span class="icon">{{ downloading ? '⏳' : '⬇️' }}</span>
      <span class="label">{{ formatLabel }}</span>
    </button>
  `,
  styles: [
    `
      .download-btn {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.5rem 1rem;
        background: var(--lamad-bg-secondary, #f3f4f6);
        color: var(--lamad-text-primary, #1f2937);
        border: 1px solid var(--lamad-border, #e5e7eb);
        border-radius: 0.5rem;
        font-size: 0.875rem;
        cursor: pointer;
        transition: all 0.15s ease;
      }

      .download-btn:hover:not(:disabled) {
        background: var(--lamad-bg-tertiary, #e5e7eb);
        border-color: var(--lamad-accent-primary, #6366f1);
      }

      .download-btn:disabled {
        opacity: 0.6;
        cursor: not-allowed;
      }

      .icon {
        font-size: 1rem;
      }

      .label {
        font-weight: 500;
      }
    `,
  ],
})
export class ContentDownloadComponent {
  @Input() node!: ContentNode;

  downloading = false;

  constructor(
    private readonly contentIO: ContentIOService,
    private readonly registry: ContentFormatRegistryService
  ) {}

  /**
   * Whether download is available for this content.
   */
  get canDownload(): boolean {
    if (!this.node?.contentFormat) return false;
    const plugin = this.registry.getPlugin(this.node.contentFormat);
    return !!plugin?.canExport;
  }

  /**
   * Human-readable format label.
   */
  get formatLabel(): string {
    if (!this.node?.contentFormat) return '';
    const plugin = this.registry.getPlugin(this.node.contentFormat);
    return plugin?.displayName ?? this.node.contentFormat;
  }

  /**
   * Download the content in its source format.
   */
  async download(): Promise<void> {
    if (!this.canDownload || this.downloading) return;

    this.downloading = true;

    try {
      await this.contentIO.downloadAsFormat(
        {
          id: this.node.id,
          title: this.node.title,
          description: this.node.description,
          content: this.node.content,
          contentFormat: this.node.contentFormat,
          contentType: this.node.contentType,
          tags: this.node.tags,
          metadata: this.node.metadata,
          relatedNodeIds: this.node.relatedNodeIds,
        },
        this.node.contentFormat
      );
    } catch {
      // Download failed - browser download or format export error, silently handled
    } finally {
      this.downloading = false;
    }
  }
}
