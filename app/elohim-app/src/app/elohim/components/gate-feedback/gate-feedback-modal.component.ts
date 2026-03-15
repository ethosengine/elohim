import {
  Component,
  ChangeDetectionStrategy,
  EventEmitter,
  Output,
  computed,
  input,
  viewChild,
} from '@angular/core';

import { GateArtifactCardComponent } from '../gate-artifact-card/gate-artifact-card.component';
import type { ReachTier } from '../../services/gate-interaction.service';

const TITLE_MAP: Record<string, string> = {
  flag: 'Flag Content',
  challenge: 'Challenge Content',
  feedback: 'Share Feedback',
};

const PLACEHOLDER_MAP: Record<string, string> = {
  flag: 'Describe the issue...',
  challenge: 'State your case...',
  feedback: 'Share your thoughts...',
};

@Component({
  selector: 'app-gate-feedback-modal',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [GateArtifactCardComponent],
  template: `
    <div
      class="modal-backdrop"
      role="dialog"
      aria-label="Feedback modal"
      data-testid="feedback-modal-backdrop"
      (click)="closed.emit()"
    >
      <div
        class="modal-panel"
        role="document"
        aria-label="Feedback form"
        data-testid="feedback-modal-panel"
        (click)="$event.stopPropagation()"
      >
        <div class="modal-header">
          <h3 data-testid="feedback-modal-title">{{ title() }}</h3>
          <button
            class="btn-close"
            aria-label="Close modal"
            data-testid="feedback-modal-close"
            (click)="closed.emit()"
          >
            &times;
          </button>
        </div>
        <app-gate-artifact-card
          [placeholder]="placeholder()"
          [mutationType]="feedbackType()"
          [contextMetadata]="contextMetadata()"
          (posted)="posted.emit($event)"
          (settled)="settled.emit($event)"
        />
      </div>
    </div>
  `,
  styles: [
    `
      .modal-backdrop {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.5);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 1000;
      }

      .modal-panel {
        background: var(--surface-elevated, #fff);
        border-radius: var(--radius-lg, 12px);
        padding: 1.5rem;
        width: 100%;
        max-width: 600px;
        max-height: 90vh;
        overflow-y: auto;
      }

      .modal-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin-bottom: 1rem;
      }

      .modal-header h3 {
        margin: 0;
        font-size: 1.25rem;
      }

      .btn-close {
        background: none;
        border: none;
        font-size: 1.5rem;
        cursor: pointer;
        color: var(--text-secondary, #5f6368);
        line-height: 1;
        padding: 0.25rem;
      }
    `,
  ],
})
export class GateFeedbackModalComponent {
  readonly feedbackType = input<'flag' | 'challenge' | 'feedback'>('feedback');
  readonly contentId = input('');

  @Output() readonly posted = new EventEmitter<{ reachTier: ReachTier }>();
  @Output() readonly settled = new EventEmitter<{
    boundary: string;
    appealPath: string | null;
  }>();
  @Output() readonly closed = new EventEmitter<void>();

  readonly artifactCard = viewChild.required(GateArtifactCardComponent);

  readonly title = computed(() => TITLE_MAP[this.feedbackType()] ?? 'Share Feedback');
  readonly placeholder = computed(() => PLACEHOLDER_MAP[this.feedbackType()] ?? 'Share your thoughts...');
  readonly contextMetadata = computed(() => ({
    contentId: this.contentId(),
    category: this.feedbackType(),
  }));
}
