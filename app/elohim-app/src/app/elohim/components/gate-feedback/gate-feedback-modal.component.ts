import {
  Component,
  ChangeDetectionStrategy,
  DestroyRef,
  EventEmitter,
  HostListener,
  Output,
  computed,
  effect,
  inject,
  input,
  viewChild,
} from '@angular/core';

import { Observable } from 'rxjs';

import {
  DiagnosticCollectorService,
  type DiagnosticBundle,
} from '../../services/diagnostic-collector.service';
import { IssueReportService } from '../../services/issue-report.service';
import { StorageApiService } from '../../services/storage-api.service';
import { GateArtifactCardComponent } from '../gate-artifact-card/gate-artifact-card.component';

import type { MutationContext, ReachTier } from '../../services/gate-interaction.service';

export type FeedbackType = 'flag' | 'challenge' | 'feedback' | 'report';

const TITLE_MAP: Record<string, string> = {
  flag: 'Flag Content',
  challenge: 'Challenge Content',
  feedback: 'Share Feedback',
  report: 'Report Issue',
};

const PLACEHOLDER_MAP: Record<string, string> = {
  flag: 'Describe the issue...',
  challenge: 'State your case...',
  feedback: 'Share your thoughts...',
  report: 'What happened?',
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
      aria-modal="true"
      aria-labelledby="feedback-modal-heading"
      data-testid="feedback-modal-backdrop"
      (click)="closed.emit()"
    >
      <div
        class="modal-panel"
        data-testid="feedback-modal-panel"
        (click)="$event.stopPropagation()"
      >
        <div class="modal-header">
          <h3 id="feedback-modal-heading" data-testid="feedback-modal-title">{{ title() }}</h3>
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
          [gateApiCall]="apiCall"
          (posted)="onPosted($event)"
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
        z-index: 1000; /* modal overlay — above all pillar content */
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

      .btn-close:hover {
        color: var(--text-primary, #202124);
      }

      .btn-close:focus-visible {
        outline: 2px solid var(--primary, #4285f4);
        border-radius: 4px;
      }
    `,
  ],
})
export class GateFeedbackModalComponent {
  private readonly destroyRef = inject(DestroyRef);
  private readonly storageApi = inject(StorageApiService);
  private readonly diagnosticCollector = inject(DiagnosticCollectorService);
  private readonly issueReportService = inject(IssueReportService);
  private diagnosticBundle: DiagnosticBundle | null = null;

  readonly feedbackType = input<FeedbackType>('feedback');
  readonly contentId = input('');

  @Output() readonly posted = new EventEmitter<{ reachTier: ReachTier }>();
  @Output() readonly settled = new EventEmitter<{
    boundary: string;
    appealPath: string | null;
  }>();
  @Output() readonly closed = new EventEmitter<void>();

  @HostListener('document:keydown.escape')
  onEscapeKey(): void {
    this.closed.emit();
  }

  readonly artifactCard = viewChild.required(GateArtifactCardComponent);

  readonly title = computed(() => TITLE_MAP[this.feedbackType()] ?? 'Share Feedback');
  readonly placeholder = computed(
    () => PLACEHOLDER_MAP[this.feedbackType()] ?? 'Share your thoughts...'
  );
  readonly contextMetadata = computed(() => ({
    contentId: this.contentId(),
    category: this.feedbackType(),
  }));

  constructor() {
    effect(() => {
      if (this.feedbackType() === 'report') {
        this.diagnosticCollector.collect().then(bundle => {
          this.diagnosticBundle = bundle;
        });
      }
    });
  }

  readonly apiCall = (text: string, context: MutationContext): Observable<unknown> => {
    if (context['category'] === 'report' && this.diagnosticBundle) {
      return this.issueReportService.createReport({
        description: text,
        diagnostics: this.diagnosticBundle,
        contextUrl: this.diagnosticBundle.context.url,
      });
    }
    return this.storageApi.createComment(context['contentId'] as string, text);
  };

  onPosted(event: { reachTier: ReachTier }): void {
    this.posted.emit(event);
    const timer = setTimeout(() => this.closed.emit(), 800);
    this.destroyRef.onDestroy(() => clearTimeout(timer));
  }
}
