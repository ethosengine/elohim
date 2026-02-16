import { Component, input, output } from '@angular/core';

export interface ShefaTab {
  id: string;
  label: string;
  icon?: string;
}

@Component({
  selector: 'app-shefa-tab-bar',
  standalone: true,
  template: `
    <div class="shefa-tab-bar" role="tablist" [attr.aria-label]="ariaLabel()">
      @for (tab of tabs(); track tab.id) {
        <button
          type="button"
          role="tab"
          class="shefa-tab"
          [class.active]="activeTab() === tab.id"
          [attr.aria-selected]="activeTab() === tab.id"
          (click)="selectTab.emit(tab.id)"
        >
          @if (tab.icon) {
            <span class="material-icons tab-icon">{{ tab.icon }}</span>
          }
          {{ tab.label }}
        </button>
      }
    </div>
  `,
  styles: [
    `
      .shefa-tab-bar {
        display: flex;
        gap: 0;
        border-bottom: 2px solid var(--lamad-border, rgba(99, 102, 241, 0.15));
        background: var(--lamad-surface, rgba(30, 30, 46, 0.4));
      }

      .shefa-tab {
        display: inline-flex;
        align-items: center;
        gap: 0.375rem;
        padding: 0.75rem 1.25rem;
        background: none;
        border: none;
        border-bottom: 2px solid transparent;
        margin-bottom: -2px;
        color: var(--lamad-text-muted, #64748b);
        font-size: 0.875rem;
        font-weight: 500;
        cursor: pointer;
        transition:
          color 0.2s,
          border-color 0.2s;
      }

      .shefa-tab:hover {
        color: var(--lamad-text-secondary, #e2e8f0);
      }

      .shefa-tab.active {
        color: var(--lamad-accent-primary, #6366f1);
        border-bottom-color: var(--lamad-accent-primary, #6366f1);
      }

      .tab-icon {
        font-size: 1.125rem;
      }

      @media (max-width: 768px) {
        .shefa-tab-bar {
          overflow-x: auto;
          scrollbar-width: none;
        }

        .shefa-tab-bar::-webkit-scrollbar {
          display: none;
        }

        .shefa-tab {
          white-space: nowrap;
          padding: 0.625rem 1rem;
          font-size: 0.8125rem;
        }
      }
    `,
  ],
})
export class ShefaTabBarComponent {
  readonly tabs = input.required<ShefaTab[]>();
  readonly activeTab = input.required<string>();
  readonly ariaLabel = input('Section tabs');

  readonly selectTab = output<string>();
}
