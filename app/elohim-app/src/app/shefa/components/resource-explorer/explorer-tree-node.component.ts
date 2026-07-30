import { Component, forwardRef, input, output, signal, inject } from '@angular/core';

import { LensRegistryService } from '@app/elohim/services/lens-registry.service';

import type { ExplorerNode } from '@app/shefa/models';

@Component({
  selector: 'app-explorer-tree-node',
  standalone: true,
  imports: [forwardRef(() => ExplorerTreeNodeComponent)],
  template: `
    <div
      class="tree-node"
      [class.active]="activeFolderId() === node().id"
      [style.padding-left.rem]="depth() * 1"
      role="treeitem"
      [attr.aria-expanded]="node().childIds.length > 0 ? isExpanded() : null"
    >
      <button type="button" class="tree-node-btn" (click)="onNodeClick()">
        @if (node().childIds.length > 0 || node().nodeType === 'folder') {
          <span class="material-icons expand-icon" [class.expanded]="isExpanded()">
            chevron_right
          </span>
        } @else {
          <span class="expand-spacer"></span>
        }
        <span class="material-icons node-icon">{{ node().icon || 'folder' }}</span>
        <span class="node-label">{{ node().title }}</span>
        @if (node().itemCount > 0) {
          <span class="node-count">{{ node().itemCount }}</span>
        }
      </button>
    </div>
    @if (isExpanded()) {
      @for (child of children(); track child.id) {
        <app-explorer-tree-node
          [node]="child"
          [depth]="depth() + 1"
          [activeFolderId]="activeFolderId()"
          [activeLens]="activeLens()"
          (folderSelected)="folderSelected.emit($event)"
        />
      }
    }
  `,
  styles: `
    :host {
      display: block;
    }

    .tree-node-btn {
      display: flex;
      align-items: center;
      gap: 0.25rem;
      width: 100%;
      padding: 0.375rem 0.5rem;
      background: none;
      border: none;
      cursor: pointer;
      color: var(--lamad-text-secondary, #e2e8f0);
      font-size: 0.8125rem;
      text-align: left;
      transition:
        background-color 0.15s,
        color 0.15s;
    }

    .tree-node-btn:hover {
      background: rgba(99, 102, 241, 0.08);
    }

    .tree-node.active > .tree-node-btn {
      background: rgba(99, 102, 241, 0.12);
      color: var(--lamad-accent-primary, #6366f1);
    }

    .expand-icon {
      font-size: 1rem;
      flex-shrink: 0;
      transition: transform 0.15s;
    }

    .expand-icon.expanded {
      transform: rotate(90deg);
    }

    .expand-spacer {
      width: 1rem;
      flex-shrink: 0;
    }

    .node-icon {
      font-size: 1rem;
      flex-shrink: 0;
      opacity: 0.7;
    }

    .node-label {
      flex: 1;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .node-count {
      font-size: 0.6875rem;
      color: var(--lamad-text-muted, #64748b);
      flex-shrink: 0;
    }
  `,
})
export class ExplorerTreeNodeComponent {
  private readonly lensRegistry = inject(LensRegistryService);

  readonly node = input.required<ExplorerNode>();
  readonly depth = input(0);
  readonly activeFolderId = input<string | null>(null);
  readonly activeLens = input.required<string>();
  readonly folderSelected = output<string>();

  readonly isExpanded = signal(false);
  readonly children = signal<ExplorerNode[]>([]);

  onNodeClick(): void {
    if (this.node().nodeType === 'folder') {
      this.folderSelected.emit(this.node().id);

      if (this.isExpanded()) {
        this.isExpanded.set(false);
      } else {
        // Lazy load children
        const provider = this.lensRegistry.getProvider(this.activeLens());
        if (provider) {
          provider.getChildren(this.node().id).subscribe(kids => {
            this.children.set(kids);
            this.isExpanded.set(true);
          });
        }
      }
    }
  }
}
