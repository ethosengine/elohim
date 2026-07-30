import { Component, input, output } from '@angular/core';

import type { ExplorerBreadcrumb } from '@app/shefa/models';

@Component({
  selector: 'app-explorer-breadcrumb',
  standalone: true,
  template: `
    <nav class="breadcrumb-trail" aria-label="Resource explorer breadcrumb">
      @for (crumb of breadcrumbs(); track crumb.id; let last = $last) {
        @if (!last) {
          <button type="button" class="breadcrumb-link" (click)="navigate.emit(crumb.id)">
            {{ crumb.title }}
          </button>
          <span class="material-icons breadcrumb-sep">chevron_right</span>
        } @else {
          <span class="breadcrumb-current">{{ crumb.title }}</span>
        }
      }
    </nav>
  `,
  styles: `
    .breadcrumb-trail {
      display: flex;
      align-items: center;
      gap: 0.25rem;
      font-size: 0.875rem;
      overflow-x: auto;
      white-space: nowrap;
      scrollbar-width: none;
    }

    .breadcrumb-link {
      background: none;
      border: none;
      color: var(--lamad-text-muted, #64748b);
      cursor: pointer;
      padding: 0.25rem 0.375rem;
      border-radius: 0.25rem;
      font-size: inherit;
      transition:
        color 0.15s,
        background-color 0.15s;
    }

    .breadcrumb-link:hover {
      color: var(--lamad-accent-primary, #6366f1);
      background: rgba(99, 102, 241, 0.08);
    }

    .breadcrumb-sep {
      font-size: 1rem;
      color: var(--lamad-text-muted, #64748b);
      flex-shrink: 0;
    }

    .breadcrumb-current {
      color: var(--lamad-text-primary, #f8fafc);
      font-weight: 500;
      padding: 0.25rem 0.375rem;
    }
  `,
})
export class ExplorerBreadcrumbComponent {
  readonly breadcrumbs = input.required<ExplorerBreadcrumb[]>();
  readonly navigate = output<string | null>();
}
