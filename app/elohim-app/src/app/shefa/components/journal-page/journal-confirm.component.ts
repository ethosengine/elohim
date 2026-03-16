import { Component, ChangeDetectionStrategy, EventEmitter, input, Output } from '@angular/core';

@Component({
  selector: 'app-journal-confirm',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @if (analyzing()) {
      <div class="shimmer" data-testid="confirm-shimmer"></div>
      <button class="btn-confirm" disabled data-testid="confirm-btn">Looks good</button>
      <button class="btn-edit" (click)="editRequested.emit()" data-testid="edit-btn">Edit</button>
    } @else {
      <div class="artifact-preview" data-testid="confirm-text">{{ text() }}</div>
      <div class="intent-summary" data-testid="intent-summary">{{ intentSummary() }}</div>
      <div class="confirm-actions">
        <button class="btn-confirm" (click)="confirmed.emit()" data-testid="confirm-btn">
          Looks good
        </button>
        <button class="btn-edit" (click)="editRequested.emit()" data-testid="edit-btn">Edit</button>
      </div>
    }
  `,
  styles: [
    `
      :host {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
      }

      .artifact-preview {
        padding: 0.75rem;
        font-size: 0.9375rem;
        line-height: 1.5;
        color: var(--text-primary, #202124);
        background: var(--surface-secondary, #f8f9fa);
        border-radius: var(--radius-md, 8px);
        white-space: pre-wrap;
      }

      .intent-summary {
        padding: 0.75rem;
        font-size: 0.875rem;
        font-style: italic;
        color: var(--text-secondary, #5f6368);
        border-left: 3px solid var(--primary, #4285f4);
        border-radius: var(--radius-sm, 4px);
      }

      .shimmer {
        background: linear-gradient(
          90deg,
          var(--surface-secondary, #f8f9fa) 25%,
          var(--surface-elevated, #fff) 50%,
          var(--surface-secondary, #f8f9fa) 75%
        );
        background-size: 200% 100%;
        animation: shimmer 2s ease-in-out infinite;
        min-height: 120px;
        border-radius: var(--radius-md, 8px);
      }

      @keyframes shimmer {
        0% {
          background-position: 200% 0;
        }
        100% {
          background-position: -200% 0;
        }
      }

      .confirm-actions {
        display: flex;
        align-items: center;
        gap: 0.75rem;
      }

      .btn-confirm,
      .btn-edit {
        padding: 0.625rem 1.25rem;
        font-size: 0.9375rem;
        font-weight: 600;
        border: none;
        border-radius: var(--radius-md, 8px);
        cursor: pointer;
        transition: all 0.15s ease;
      }

      .btn-confirm {
        background: var(--success, #34a853);
        color: white;
      }

      .btn-confirm:hover:not(:disabled) {
        background: var(--success-dark, #1e8e3e);
      }

      .btn-confirm:disabled {
        background: var(--surface-tertiary, #e8eaed);
        color: var(--text-disabled, #9aa0a6);
        cursor: not-allowed;
      }

      .btn-edit {
        background: transparent;
        color: var(--text-secondary, #5f6368);
        border: 1px solid var(--border-color, #e9ecef);
      }

      .btn-edit:hover {
        background: var(--surface-secondary, #f8f9fa);
        color: var(--text-primary, #202124);
      }
    `,
  ],
})
export class JournalConfirmComponent {
  readonly text = input.required<string>();
  readonly intentSummary = input.required<string>();
  readonly analyzing = input.required<boolean>();

  @Output() readonly confirmed = new EventEmitter<void>();
  @Output() readonly editRequested = new EventEmitter<void>();
}
