import { Component, input, output } from '@angular/core';

import type { LensDefinition } from '@app/shefa/models';

@Component({
  selector: 'app-lens-switcher',
  standalone: true,
  template: `
    <div class="lens-switcher" role="tablist" aria-label="Resource view">
      @for (lens of lenses(); track lens.type) {
        <button
          type="button"
          class="lens-tab"
          role="tab"
          [class.active]="activeLens() === lens.type"
          [attr.aria-selected]="activeLens() === lens.type"
          [title]="lens.description"
          (click)="lensChanged.emit(lens.type)"
        >
          <span class="material-icons lens-icon">{{ lens.icon }}</span>
          <span class="lens-label">{{ lens.label }}</span>
        </button>
      }
    </div>
  `,
  styles: `
    .lens-switcher {
      display: flex;
      gap: 0.25rem;
      padding: 0.25rem;
      background: var(--lamad-bg-tertiary, rgba(51, 65, 85, 0.5));
      border-radius: 0.5rem;
      flex-shrink: 0;
    }

    .lens-tab {
      display: flex;
      align-items: center;
      gap: 0.375rem;
      padding: 0.375rem 0.75rem;
      background: none;
      border: none;
      border-radius: 0.375rem;
      color: var(--lamad-text-muted, #64748b);
      font-size: 0.8125rem;
      font-weight: 500;
      cursor: pointer;
      white-space: nowrap;
      transition:
        color 0.15s,
        background-color 0.15s;
    }

    .lens-tab:hover {
      color: var(--lamad-text-secondary, #e2e8f0);
      background: rgba(99, 102, 241, 0.08);
    }

    .lens-tab.active {
      color: var(--lamad-text-primary, #f8fafc);
      background: var(--lamad-bg-secondary, #1e293b);
    }

    .lens-icon {
      font-size: 1rem;
    }

    @media (width <= 768px) {
      .lens-label {
        display: none;
      }

      .lens-tab {
        padding: 0.375rem 0.5rem;
      }
    }
  `,
})
export class LensSwitcherComponent {
  readonly lenses = input.required<LensDefinition[]>();
  readonly activeLens = input.required<string>();
  readonly lensChanged = output<string>();
}
