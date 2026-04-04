import { Component, input, output } from '@angular/core';

/**
 * ContextMenuOnlyComponent - Level 0 Governance Surface
 *
 * For constitutional/settled content where no low-friction signals are shown.
 * Only a subtle kebab menu (three dots) provides access to governance actions:
 * - Flag: report content for review
 * - Challenge: file a governance challenge
 * - Open Feedback: provide unstructured feedback
 *
 * These actions emit events for the parent gateway to handle.
 * No backend wiring in this component — pure presentation + event emission.
 */
@Component({
  selector: 'qahal-context-menu-only',
  standalone: true,
  template: `
    <div class="context-menu-wrapper">
      <button
        class="kebab-trigger"
        type="button"
        (click)="toggleMenu()"
        [attr.aria-expanded]="isOpen"
        aria-haspopup="true"
        aria-label="Content actions"
      >
        <span class="kebab-icon" aria-hidden="true">&#8942;</span>
      </button>

      @if (isOpen) {
        <div class="menu-backdrop" (click)="closeMenu()"></div>
        <ul class="menu-dropdown" role="menu">
          @for (action of actions; track action.id) {
            <li role="none">
              <button
                class="menu-item"
                type="button"
                role="menuitem"
                (click)="onAction(action)"
              >
                <span class="menu-icon" aria-hidden="true">{{ action.icon }}</span>
                <div class="menu-text">
                  <span class="menu-label">{{ action.label }}</span>
                  <span class="menu-description">{{ action.description }}</span>
                </div>
              </button>
            </li>
          }
        </ul>
      }
    </div>
  `,
  styles: `
    .context-menu-wrapper {
      position: relative;
      display: inline-block;
    }

    .kebab-trigger {
      display: flex;
      align-items: center;
      justify-content: center;
      width: 32px;
      height: 32px;
      border: none;
      border-radius: 6px;
      background: transparent;
      cursor: pointer;
      color: var(--text-secondary, #999);
      transition:
        background 0.15s,
        color 0.15s;
      font-family: inherit;
      padding: 0;
    }

    .kebab-trigger:hover {
      background: var(--surface-elevated, #f5f5f5);
      color: var(--text-primary, #333);
    }

    .kebab-trigger:focus-visible {
      outline: 2px solid var(--primary, #6366f1);
      outline-offset: 2px;
    }

    .kebab-icon {
      font-size: 1.25rem;
      line-height: 1;
      letter-spacing: 1px;
    }

    .menu-backdrop {
      position: fixed;
      inset: 0;
      z-index: 9998;
    }

    .menu-dropdown {
      position: absolute;
      right: 0;
      top: 100%;
      margin-top: 4px;
      min-width: 220px;
      padding: 4px 0;
      background: var(--surface, #fff);
      border: 1px solid var(--border, #e5e5e5);
      border-radius: 8px;
      box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
      list-style: none;
      z-index: 9999;
    }

    .menu-item {
      display: flex;
      align-items: flex-start;
      gap: 0.625rem;
      width: 100%;
      padding: 0.5rem 0.75rem;
      border: none;
      background: transparent;
      cursor: pointer;
      text-align: left;
      font-family: inherit;
      color: var(--text-primary, #1a1a1a);
      transition: background 0.1s;
    }

    .menu-item:hover {
      background: var(--surface-elevated, #f5f5f5);
    }

    .menu-item:focus-visible {
      outline: 2px solid var(--primary, #6366f1);
      outline-offset: -2px;
    }

    .menu-icon {
      font-size: 1rem;
      flex-shrink: 0;
      margin-top: 1px;
    }

    .menu-text {
      display: flex;
      flex-direction: column;
      gap: 1px;
    }

    .menu-label {
      font-size: 0.8125rem;
      font-weight: 500;
      line-height: 1.4;
    }

    .menu-description {
      font-size: 0.6875rem;
      color: var(--text-secondary, #666);
      line-height: 1.3;
    }

    @media (prefers-color-scheme: dark) {
      .kebab-trigger:hover {
        background: var(--surface-elevated, #2a2a2a);
      }

      .menu-dropdown {
        background: var(--surface, #1a1a1a);
        border-color: var(--border, #333);
        box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
      }

      .menu-item:hover {
        background: var(--surface-elevated, #2a2a2a);
      }
    }
  `,
})
export class ContextMenuOnlyComponent {
  /** The type of entity this menu applies to (e.g. 'content', 'path', 'proposal'). */
  entityType = input.required<string>();

  /** The unique identifier of the entity. */
  entityId = input.required<string>();

  /** Emitted when the user selects "Flag" — report content for review. */
  flag = output<ContextMenuAction>();

  /** Emitted when the user selects "Challenge" — file a governance challenge. */
  challenge = output<ContextMenuAction>();

  /** Emitted when the user selects "Open Feedback" — provide unstructured feedback. */
  openFeedback = output<ContextMenuAction>();

  /** Whether the dropdown menu is currently open. */
  isOpen = false;

  /** The menu actions available in the dropdown. */
  readonly actions: MenuAction[] = [
    {
      id: 'flag',
      icon: '\u{1F6A9}',
      label: 'Flag',
      description: 'Report this content for review',
    },
    {
      id: 'challenge',
      icon: '\u{2696}',
      label: 'Challenge',
      description: 'File a governance challenge',
    },
    {
      id: 'open-feedback',
      icon: '\u{1F4AC}',
      label: 'Open Feedback',
      description: 'Provide unstructured feedback',
    },
  ];

  toggleMenu(): void {
    this.isOpen = !this.isOpen;
  }

  closeMenu(): void {
    this.isOpen = false;
  }

  onAction(action: MenuAction): void {
    const payload: ContextMenuAction = {
      entityType: this.entityType(),
      entityId: this.entityId(),
      action: action.id,
    };

    switch (action.id) {
      case 'flag':
        this.flag.emit(payload);
        break;
      case 'challenge':
        this.challenge.emit(payload);
        break;
      case 'open-feedback':
        this.openFeedback.emit(payload);
        break;
    }

    this.closeMenu();
  }
}

// ===========================================================================
// Types
// ===========================================================================

export interface ContextMenuAction {
  entityType: string;
  entityId: string;
  action: 'flag' | 'challenge' | 'open-feedback';
}

interface MenuAction {
  id: 'flag' | 'challenge' | 'open-feedback';
  icon: string;
  label: string;
  description: string;
}
