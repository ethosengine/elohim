import { Component, input, output } from '@angular/core';

import type { ExplorerNode } from '@app/shefa/models';

@Component({
  selector: 'app-explorer-grid',
  standalone: true,
  template: `
    @if (folders().length === 0 && files().length === 0) {
      <div class="empty-state">
        <span class="material-icons empty-icon">folder_open</span>
        <p class="empty-text">This folder is empty</p>
      </div>
    }

    @if (folders().length > 0) {
      <section class="grid-section">
        <h3 class="grid-section-title">Folders</h3>
        <div class="folder-grid">
          @for (folder of folders(); track folder.id) {
            <button type="button" class="folder-card" (click)="folderClicked.emit(folder.id)">
              <span class="material-icons folder-icon">{{ folder.icon || 'folder' }}</span>
              <span class="folder-name">{{ folder.title }}</span>
              @if (folder.itemCount > 0) {
                <span class="folder-count">{{ folder.itemCount }} items</span>
              }
            </button>
          }
        </div>
      </section>
    }

    @if (files().length > 0) {
      <section class="grid-section">
        <h3 class="grid-section-title">Files</h3>
        <div class="file-grid">
          @for (file of files(); track file.id) {
            <button type="button" class="file-card" (click)="fileClicked.emit(file.resourceId!)">
              <span class="file-icon">{{ file.icon }}</span>
              <span class="file-name">{{ file.title }}</span>
              @if (file.contentType) {
                <span class="file-type">{{ file.contentType }}</span>
              }
            </button>
          }
        </div>
      </section>
    }
  `,
  styles: `
    .empty-state {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      padding: 4rem 2rem;
      color: var(--lamad-text-muted, #64748b);
    }

    .empty-icon {
      font-size: 3rem;
      margin-bottom: 1rem;
      opacity: 0.5;
    }

    .empty-text {
      font-size: 0.9375rem;
      margin: 0;
    }

    .grid-section {
      padding: 0 0 1.5rem;
    }

    .grid-section-title {
      font-size: 0.75rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--lamad-text-muted, #64748b);
      margin: 0 0 0.75rem;
    }

    .folder-grid,
    .file-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
      gap: 0.75rem;
    }

    .folder-card,
    .file-card {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 0.5rem;
      padding: 1.25rem 1rem;
      background: var(--lamad-bg-secondary, #1e293b);
      border: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.1));
      border-radius: 0.5rem;
      cursor: pointer;
      text-align: center;
      transition:
        background-color 0.15s,
        border-color 0.15s;
    }

    .folder-card:hover,
    .file-card:hover {
      background: rgba(99, 102, 241, 0.08);
      border-color: var(--lamad-accent-primary, #6366f1);
    }

    .folder-icon {
      font-size: 2rem;
      color: var(--lamad-accent-primary, #6366f1);
    }

    .folder-name,
    .file-name {
      font-size: 0.875rem;
      font-weight: 500;
      color: var(--lamad-text-primary, #f8fafc);
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      max-width: 100%;
    }

    .folder-count,
    .file-type {
      font-size: 0.75rem;
      color: var(--lamad-text-muted, #64748b);
    }

    .file-icon {
      font-size: 1.75rem;
    }

    @media (width <= 768px) {
      .folder-grid,
      .file-grid {
        grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
        gap: 0.5rem;
      }

      .folder-card,
      .file-card {
        padding: 1rem 0.75rem;
      }
    }

    @media (width <= 480px) {
      .folder-grid,
      .file-grid {
        grid-template-columns: 1fr 1fr;
      }
    }
  `,
})
export class ExplorerGridComponent {
  readonly folders = input.required<ExplorerNode[]>();
  readonly files = input.required<ExplorerNode[]>();
  readonly folderClicked = output<string>();
  readonly fileClicked = output<string>();
}
