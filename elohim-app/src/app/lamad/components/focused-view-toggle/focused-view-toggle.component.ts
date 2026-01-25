import { CommonModule } from '@angular/common';
import { Component, Input, Output, EventEmitter } from '@angular/core';

/**
 * Toggle button for focused/fullscreen content view.
 * Displays a floating button to expand content for immersive viewing.
 * Hidden on mobile viewports (< 768px).
 */
@Component({
  selector: 'app-focused-view-toggle',
  standalone: true,
  imports: [CommonModule],
  template: `
    <button
      class="focused-view-btn"
      [class.active]="isActive"
      (click)="toggle()"
      [attr.aria-label]="isActive ? 'Exit focused view' : 'Enter focused view'"
      [attr.aria-pressed]="isActive"
      type="button"
    >
      <span class="icon">{{ isActive ? '⤢' : '⤡' }}</span>
      <span class="label">{{ isActive ? 'Exit' : 'Focus' }}</span>
    </button>
  `,
  styles: [
    `
      .focused-view-btn {
        display: none; /* Hidden on mobile by default */
        align-items: center;
        gap: 0.5rem;
        padding: 0.5rem 1rem;
        background: var(--lamad-surface, rgba(255, 255, 255, 0.95));
        border: 1px solid var(--lamad-border, rgba(148, 163, 184, 0.2));
        border-radius: 8px;
        color: var(--lamad-text-secondary, #334155);
        font-size: 0.875rem;
        font-weight: 500;
        cursor: pointer;
        transition: all 0.2s ease;
        backdrop-filter: blur(8px);
        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
      }

      .focused-view-btn:hover {
        background: var(--lamad-bg-secondary, #f8fafc);
        border-color: var(--lamad-accent-primary, #6366f1);
        color: var(--lamad-accent-primary, #6366f1);
        transform: scale(1.02);
      }

      .focused-view-btn:active {
        transform: scale(0.98);
      }

      .focused-view-btn.active {
        background: var(--lamad-accent-primary, #6366f1);
        border-color: var(--lamad-accent-primary, #6366f1);
        color: white;
      }

      .focused-view-btn.active:hover {
        background: var(--lamad-accent-secondary, #8b5cf6);
        border-color: var(--lamad-accent-secondary, #8b5cf6);
        color: white;
      }

      .icon {
        font-size: 1.1rem;
        line-height: 1;
      }

      .label {
        font-size: 0.8rem;
        text-transform: uppercase;
        letter-spacing: 0.05em;
      }

      /* Show on tablet and desktop (>= 768px) */
      @media (min-width: 768px) {
        .focused-view-btn {
          display: flex;
        }
      }
    `,
  ],
})
export class FocusedViewToggleComponent {
  @Input() isActive = false;
  @Output() toggled = new EventEmitter<boolean>();

  toggle(): void {
    this.toggled.emit(!this.isActive);
  }
}
