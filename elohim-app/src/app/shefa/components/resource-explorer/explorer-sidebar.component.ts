import { Component, input, output } from '@angular/core';

import { ExplorerTreeNodeComponent } from './explorer-tree-node.component';

import type { ExplorerNode, LensDefinition } from '@app/shefa/models';

@Component({
  selector: 'app-explorer-sidebar',
  standalone: true,
  imports: [ExplorerTreeNodeComponent],
  template: `
    <aside class="explorer-sidebar" role="tree" aria-label="Resource tree">
      <div class="sidebar-header">
        <span class="sidebar-context">Resources</span>
        <h2 class="sidebar-title">{{ activeLensLabel() }}</h2>
      </div>

      <div class="tree-container">
        @for (node of tree(); track node.id) {
          <app-explorer-tree-node
            [node]="node"
            [activeFolderId]="activeFolderId()"
            [activeLens]="activeLens()"
            (folderSelected)="folderSelected.emit($event)"
          />
        }

        @if (tree().length === 0) {
          <div class="tree-empty">
            <span class="material-icons empty-icon">account_tree</span>
            <p class="empty-text">No resources</p>
          </div>
        }
      </div>

      <button
        type="button"
        class="sidebar-collapse-btn"
        (click)="collapseClicked.emit()"
        aria-label="Collapse sidebar"
        data-testid="explorer-sidebar-collapse"
      >
        <span class="material-icons collapse-icon">chevron_left</span>
      </button>
    </aside>
  `,
  styles: `
    .explorer-sidebar {
      width: 260px;
      flex-shrink: 0;
      display: flex;
      flex-direction: column;
      background: var(--lamad-bg-secondary, #1a1a2e);
      border-right: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.15));
      overflow: hidden;
      transition:
        width 0.3s ease,
        transform 0.3s ease;
    }

    .sidebar-header {
      padding: 1rem 1.25rem;
      border-bottom: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.15));
    }

    .sidebar-context {
      display: block;
      font-size: 0.75rem;
      font-weight: 500;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--lamad-text-muted, #64748b);
      margin-bottom: 0.25rem;
    }

    .sidebar-title {
      font-size: 1rem;
      font-weight: 700;
      color: var(--lamad-text-primary, #f8fafc);
      margin: 0;
      line-height: 1.3;
    }

    .tree-container {
      flex: 1;
      overflow-y: auto;
      padding: 0.5rem 0;
      scrollbar-width: thin;
      scrollbar-color: rgba(99, 102, 241, 0.2) transparent;
    }

    .tree-empty {
      display: flex;
      flex-direction: column;
      align-items: center;
      padding: 2rem 1rem;
      color: var(--lamad-text-muted, #64748b);
    }

    .empty-icon {
      font-size: 2rem;
      opacity: 0.5;
      margin-bottom: 0.5rem;
    }

    .empty-text {
      font-size: 0.8125rem;
      margin: 0;
    }

    .sidebar-collapse-btn {
      display: flex;
      align-items: center;
      justify-content: center;
      width: 100%;
      padding: 0.75rem;
      background: transparent;
      border: none;
      border-top: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.1));
      cursor: pointer;
      color: var(--lamad-text-muted, #64748b);
      transition:
        background-color 0.2s,
        color 0.2s;
    }

    .sidebar-collapse-btn:hover {
      background-color: rgba(99, 102, 241, 0.08);
      color: var(--lamad-text-secondary, #e2e8f0);
    }

    .collapse-icon {
      font-size: 1.25rem;
    }
  `,
})
export class ExplorerSidebarComponent {
  readonly tree = input.required<ExplorerNode[]>();
  readonly activeFolderId = input<string | null>(null);
  readonly activeLens = input.required<string>();
  readonly activeLensLabel = input('Resources');
  readonly lenses = input.required<LensDefinition[]>();
  readonly folderSelected = output<string>();
  readonly collapseClicked = output<void>();
}
