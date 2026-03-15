import {
  Component,
  EventEmitter,
  Input,
  Output,
  ChangeDetectionStrategy,
  effect,
  inject,
  signal,
} from '@angular/core';
import { Observable } from 'rxjs';

import {
  GateInteractionService,
  type ReachTier,
  type MutationContext,
} from '../../services/gate-interaction.service';

const REACH_ICONS: Record<ReachTier, string> = {
  private: '\u{1F512}',
  close: '\u{1F464}',
  community: '\u{1F465}',
  network: '\u{1F310}',
  constitutional: '\u2696\uFE0F',
};

const REACH_LABELS: Record<ReachTier, string> = {
  private: 'Private',
  close: 'Close',
  community: 'Community',
  network: 'Network',
  constitutional: 'Constitutional',
};

@Component({
  selector: 'app-gate-artifact-card',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  providers: [GateInteractionService],
  template: `
    <div
      class="artifact-card gate-comment"
      [class.evaluating]="interaction.state() === 'evaluating'"
    >
      @switch (interaction.state()) {
        @case ('draft') {
          <textarea
            class="artifact-textarea"
            [placeholder]="placeholder"
            [value]="localText()"
            (input)="onInput($event)"
            aria-label="Artifact text"
            data-testid="artifact-textarea"
          ></textarea>
          <button
            class="btn-submit"
            [disabled]="!localText()"
            (click)="onSubmit()"
            data-testid="artifact-submit"
          >
            Submit
          </button>
        }
        @case ('evaluating') {
          <div class="artifact-preview" data-testid="artifact-preview">
            {{ interaction.draftText() }}
          </div>
        }
        @case ('affirm') {
          <div class="artifact-preview" data-testid="artifact-preview">
            {{ interaction.draftText() }}
          </div>
          <div class="affirm-bar">
            <span
              class="reach-badge"
              [title]="reachLabel()"
              data-testid="reach-badge"
            >
              {{ reachIcon() }} {{ reachLabel() }}
            </span>
            <button
              class="btn-affirm"
              (click)="onAffirm()"
              data-testid="artifact-affirm"
            >
              Affirm &amp; Post
            </button>
          </div>
        }
        @case ('dialogue') {
          <div
            class="dialogue-prompt"
            data-testid="dialogue-prompt"
          >
            {{ interaction.gateResult()?.pausePrompt }}
          </div>
          <textarea
            class="artifact-textarea"
            [value]="localText()"
            (input)="onInput($event)"
            aria-label="Revise artifact text"
            data-testid="artifact-textarea"
          ></textarea>
          <ng-content select="[dialogue]"></ng-content>
          <button
            class="btn-resubmit"
            [disabled]="!localText()"
            (click)="onResubmit()"
            data-testid="artifact-resubmit"
          >
            Resubmit
          </button>
        }
        @case ('settled') {
          <div class="artifact-preview settled-text" data-testid="artifact-preview">
            {{ interaction.draftText() }}
          </div>
          <div class="settlement-info" data-testid="settlement-info">
            <span class="settlement-boundary">
              Boundary: {{ interaction.gateResult()?.settlementBoundary }}
            </span>
            @if (interaction.gateResult()?.appealPath) {
              <a
                class="settlement-link"
                [href]="interaction.gateResult()?.appealPath"
                data-testid="settlement-link"
              >
                View appeal story
              </a>
            }
          </div>
        }
        @case ('posted') {
          <div class="artifact-preview" data-testid="artifact-preview">
            {{ interaction.draftText() }}
          </div>
          <span
            class="reach-badge posted-badge"
            data-testid="reach-badge"
          >
            {{ reachIcon() }} {{ reachLabel() }}
          </span>
        }
      }
    </div>
  `,
  styles: [
    `
      .artifact-card {
        max-width: 560px;
        margin: 1rem auto;
        padding: 1.25rem;
        background: var(--surface-elevated, #fff);
        border-radius: var(--radius-lg, 12px);
        border: 2px solid var(--border-color, #e9ecef);
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
      }

      .artifact-card.evaluating {
        border-image: linear-gradient(
            90deg,
            var(--border-color, #e9ecef) 0%,
            var(--primary, #4285f4) 50%,
            var(--border-color, #e9ecef) 100%
          )
          1;
        animation: shimmer 2s ease-in-out infinite;
      }

      @keyframes shimmer {
        0%,
        100% {
          border-image-source: linear-gradient(
            90deg,
            var(--border-color, #e9ecef) 0%,
            var(--primary, #4285f4) 50%,
            var(--border-color, #e9ecef) 100%
          );
        }
        50% {
          border-image-source: linear-gradient(
            90deg,
            var(--primary, #4285f4) 0%,
            var(--border-color, #e9ecef) 50%,
            var(--primary, #4285f4) 100%
          );
        }
      }

      .artifact-textarea {
        width: 100%;
        min-height: 80px;
        padding: 0.75rem;
        font-family: inherit;
        font-size: 0.9375rem;
        color: var(--text-primary, #202124);
        background: var(--surface-secondary, #f8f9fa);
        border: 1px solid var(--border-color, #e9ecef);
        border-radius: var(--radius-md, 8px);
        resize: vertical;
        box-sizing: border-box;
      }

      .artifact-textarea:focus {
        outline: none;
        border-color: var(--primary, #4285f4);
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

      .settled-text {
        color: var(--text-secondary, #5f6368);
      }

      .affirm-bar {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 1rem;
      }

      .reach-badge {
        display: inline-flex;
        align-items: center;
        gap: 0.375rem;
        padding: 0.25rem 0.625rem;
        font-size: 0.8125rem;
        font-weight: 500;
        color: var(--text-secondary, #5f6368);
        background: var(--surface-secondary, #f8f9fa);
        border-radius: var(--radius-sm, 4px);
      }

      .posted-badge {
        align-self: flex-start;
      }

      .btn-submit,
      .btn-affirm,
      .btn-resubmit {
        padding: 0.625rem 1.25rem;
        font-size: 0.9375rem;
        font-weight: 600;
        border: none;
        border-radius: var(--radius-md, 8px);
        cursor: pointer;
        transition: all 0.15s ease;
      }

      .btn-submit {
        align-self: flex-end;
        background: var(--primary, #4285f4);
        color: white;
      }

      .btn-submit:hover:not(:disabled) {
        background: var(--primary-dark, #1a73e8);
      }

      .btn-submit:disabled {
        background: var(--surface-tertiary, #e8eaed);
        color: var(--text-disabled, #9aa0a6);
        cursor: not-allowed;
      }

      .btn-affirm {
        background: var(--success, #34a853);
        color: white;
      }

      .btn-affirm:hover {
        background: var(--success-dark, #1e8e3e);
      }

      .btn-resubmit {
        align-self: flex-end;
        background: var(--primary, #4285f4);
        color: white;
      }

      .btn-resubmit:hover:not(:disabled) {
        background: var(--primary-dark, #1a73e8);
      }

      .btn-resubmit:disabled {
        background: var(--surface-tertiary, #e8eaed);
        color: var(--text-disabled, #9aa0a6);
        cursor: not-allowed;
      }

      .dialogue-prompt {
        padding: 0.75rem;
        font-size: 0.875rem;
        font-style: italic;
        color: var(--text-secondary, #5f6368);
        background: var(--warning-surface, #fef7e0);
        border-left: 3px solid var(--warning, #fbbc04);
        border-radius: var(--radius-sm, 4px);
      }

      .settlement-info {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        padding: 0.75rem;
        font-size: 0.875rem;
        color: var(--text-secondary, #5f6368);
        background: var(--surface-secondary, #f8f9fa);
        border-radius: var(--radius-md, 8px);
      }

      .settlement-boundary {
        font-weight: 500;
      }

      .settlement-link {
        color: var(--primary, #4285f4);
        text-decoration: none;
      }

      .settlement-link:hover {
        text-decoration: underline;
      }
    `,
  ],
})
export class GateArtifactCardComponent {
  @Input() placeholder = 'Write something...';
  @Input() mutationType = '';
  @Input() contextMetadata: MutationContext = {};
  @Input() gateApiCall?: (text: string, context: MutationContext) => Observable<unknown>;

  @Output() posted = new EventEmitter<{ reachTier: ReachTier }>();
  @Output() settled = new EventEmitter<{
    boundary: string;
    appealPath: string | null;
  }>();

  readonly interaction = inject(GateInteractionService);
  readonly localText = signal('');

  constructor() {
    effect(() => {
      const state = this.interaction.state();
      if (state === 'posted') {
        this.posted.emit({ reachTier: this.interaction.reachTier() });
      } else if (state === 'settled') {
        const result = this.interaction.gateResult();
        this.settled.emit({
          boundary: result?.settlementBoundary ?? '',
          appealPath: result?.appealPath ?? null,
        });
      }
    });
  }

  reachIcon(): string {
    return REACH_ICONS[this.interaction.reachTier()];
  }

  reachLabel(): string {
    return REACH_LABELS[this.interaction.reachTier()];
  }

  onInput(event: Event): void {
    const value = (event.target as HTMLTextAreaElement).value;
    this.localText.set(value);
  }

  onSubmit(): void {
    const text = this.localText().trim();
    if (!text) return;
    if (this.gateApiCall) {
      this.interaction.submitWithApi(text, this.mutationType, this.contextMetadata, this.gateApiCall);
    } else {
      this.interaction.submit(text, this.mutationType, this.contextMetadata);
    }
  }

  onAffirm(): void {
    this.interaction.affirm();
  }

  protected onResubmit(): void {
    const text = this.localText().trim();
    if (!text) return;
    if (this.gateApiCall) {
      this.interaction.submitWithApi(text, this.mutationType, this.contextMetadata, this.gateApiCall);
    } else {
      this.interaction.submit(text, this.mutationType, this.contextMetadata);
    }
  }
}
