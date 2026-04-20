import { CommonModule } from '@angular/common';
import { Component, OnInit, inject, input } from '@angular/core';
import { FormsModule } from '@angular/forms';

import { GovernanceApiService } from '@app/elohim/services/governance-api.service';

import type { StatementView, CreateStatementInputView } from '@elohim/storage-client/generated';

/**
 * ContributeStatementComponent - Polis-style One-at-a-Time Voting
 *
 * The core sensemaking UX. Surfaces statements one at a time and asks
 * the participant to agree, disagree, or pass. Once all statements are
 * voted on, invites the participant to contribute their own perspective.
 *
 * Design principle: "The goal is not to win, but to understand."
 * Each vote feeds into the clustering algorithm that discovers where
 * the community already agrees and where genuine tensions exist.
 */
@Component({
  selector: 'app-contribute-statement',
  standalone: true,
  imports: [CommonModule, FormsModule],
  template: `
    @if (loading) {
      <div class="contribute-loading" data-testid="contribute-loading">
        <p>Loading statements...</p>
      </div>
    } @else if (error) {
      <div class="contribute-error" data-testid="contribute-error">
        <p>{{ error }}</p>
        <button class="retry-btn" data-testid="retry-btn" (click)="loadStatements()">Retry</button>
      </div>
    } @else if (statements.length === 0) {
      <!-- No statements yet: show contribute input immediately -->
      <div class="contribute-empty" data-testid="contribute-empty">
        <h3>Be the first to share a perspective</h3>
        <p>No statements yet. Start the conversation by adding your viewpoint.</p>
        <ng-container *ngTemplateOutlet="contributeForm"></ng-container>
      </div>
    } @else if (currentStatement) {
      <!-- Polis-style one-at-a-time voting -->
      <div class="contribute-vote" data-testid="contribute-vote">
        <div class="progress-bar-container">
          <div class="progress-bar-fill" [style.width.%]="progressPercent"></div>
        </div>
        <p class="progress-text">Voted on {{ votedCount }} of {{ statements.length }} statements</p>

        <div class="statement-card" [class.voting]="isVoting" data-testid="statement-card">
          <p class="statement-text">{{ currentStatement.text }}</p>
        </div>

        <div class="vote-actions">
          <button
            class="vote-btn vote-agree"
            [disabled]="isVoting"
            data-testid="vote-agree"
            (click)="vote('agree')"
          >
            Agree
          </button>
          <button
            class="vote-btn vote-pass"
            [disabled]="isVoting"
            data-testid="vote-pass"
            (click)="vote('pass')"
          >
            Pass
          </button>
          <button
            class="vote-btn vote-disagree"
            [disabled]="isVoting"
            data-testid="vote-disagree"
            (click)="vote('disagree')"
          >
            Disagree
          </button>
        </div>
      </div>
    } @else {
      <!-- All voted: show contribute input -->
      <div class="contribute-done" data-testid="contribute-done">
        <div class="completion-banner">
          <p>You have voted on all {{ statements.length }} statements.</p>
        </div>

        @if (submitted) {
          <div class="confirmation" data-testid="submit-confirmation">
            <p>Your perspective has been added. Thank you for contributing.</p>
          </div>
        } @else {
          <ng-container *ngTemplateOutlet="contributeForm"></ng-container>
        }
      </div>
    }

    <ng-template #contributeForm>
      <div class="contribute-form" data-testid="contribute-form">
        <label class="contribute-label" for="new-statement">Add your own perspective</label>
        <textarea
          id="new-statement"
          [(ngModel)]="newStatementText"
          class="contribute-textarea"
          placeholder="Share a concise viewpoint for others to consider..."
          rows="3"
          data-testid="statement-input"
        ></textarea>
        <button
          class="submit-btn"
          [disabled]="!newStatementText.trim() || isSubmitting"
          data-testid="submit-statement"
          (click)="submitStatement()"
        >
          {{ isSubmitting ? 'Submitting...' : 'Submit' }}
        </button>
      </div>
    </ng-template>
  `,
  styles: [
    `
      .contribute-loading,
      .contribute-error {
        padding: 1rem;
        text-align: center;
        color: var(--text-secondary, #666);
      }
      .retry-btn {
        margin-top: 0.5rem;
        padding: 0.375rem 0.75rem;
        border: 1px solid var(--border-color, #ddd);
        border-radius: 0.25rem;
        background: transparent;
        cursor: pointer;
      }
      .contribute-empty h3 {
        margin: 0 0 0.5rem;
        font-size: 1.125rem;
      }
      .contribute-empty p {
        color: var(--text-secondary, #666);
        margin: 0 0 1rem;
      }
      .progress-bar-container {
        height: 0.375rem;
        background: var(--surface-secondary, #eee);
        border-radius: 0.25rem;
        overflow: hidden;
      }
      .progress-bar-fill {
        height: 100%;
        background: var(--primary-color, #2196f3);
        border-radius: 0.25rem;
        transition: width 0.3s ease;
      }
      .progress-text {
        font-size: 0.813rem;
        color: var(--text-secondary, #666);
        margin: 0.375rem 0 1rem;
        text-align: center;
      }
      .statement-card {
        background: var(--surface-color, #fff);
        border: 1px solid var(--border-color, #ddd);
        border-radius: 0.5rem;
        padding: 1.25rem;
        margin-bottom: 1rem;
        transition: opacity 0.2s ease;
      }
      .statement-card.voting {
        opacity: 0.6;
      }
      .statement-text {
        font-size: 1rem;
        line-height: 1.5;
        margin: 0;
      }
      .vote-actions {
        display: flex;
        gap: 0.75rem;
        justify-content: center;
      }
      .vote-btn {
        padding: 0.5rem 1.25rem;
        border: 2px solid;
        border-radius: 0.375rem;
        font-size: 0.875rem;
        font-weight: 500;
        cursor: pointer;
        transition: all 0.15s ease;
        background: transparent;
      }
      .vote-btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
      }
      .vote-agree {
        border-color: #27ae60;
        color: #27ae60;
      }
      .vote-agree:hover:not(:disabled) {
        background: #27ae60;
        color: #fff;
      }
      .vote-pass {
        border-color: #95a5a6;
        color: #95a5a6;
      }
      .vote-pass:hover:not(:disabled) {
        background: #95a5a6;
        color: #fff;
      }
      .vote-disagree {
        border-color: #e67e22;
        color: #e67e22;
      }
      .vote-disagree:hover:not(:disabled) {
        background: #e67e22;
        color: #fff;
      }
      .completion-banner {
        text-align: center;
        padding: 0.75rem;
        background: var(--success-light, #e8f5e9);
        border-radius: 0.375rem;
        margin-bottom: 1rem;
      }
      .completion-banner p {
        margin: 0;
        color: var(--success-color, #27ae60);
        font-weight: 500;
      }
      .confirmation {
        text-align: center;
        padding: 0.75rem;
        color: var(--success-color, #27ae60);
      }
      .confirmation p {
        margin: 0;
      }
      .contribute-form {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
      }
      .contribute-label {
        font-size: 0.875rem;
        font-weight: 500;
      }
      .contribute-textarea {
        width: 100%;
        padding: 0.5rem;
        border: 1px solid var(--border-color, #ddd);
        border-radius: 0.25rem;
        font-family: inherit;
        font-size: 0.875rem;
        resize: vertical;
      }
      .submit-btn {
        align-self: flex-end;
        padding: 0.5rem 1rem;
        border: none;
        border-radius: 0.25rem;
        background: var(--primary-color, #2196f3);
        color: #fff;
        cursor: pointer;
        font-size: 0.875rem;
      }
      .submit-btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
      }
    `,
  ],
})
export class ContributeStatementComponent implements OnInit {
  /** Entity type for sensemaking context (e.g. 'content', 'proposal') */
  readonly entityType = input.required<string>();

  /** Entity ID for sensemaking context */
  readonly entityId = input.required<string>();

  /** Human ID of the current participant */
  readonly humanId = input<string>('current-user');

  private readonly governanceApi = inject(GovernanceApiService);

  // State
  statements: StatementView[] = [];
  votedIds = new Set<string>();
  loading = true;
  error = '';
  isVoting = false;
  isSubmitting = false;
  submitted = false;
  newStatementText = '';

  /** The next unvoted statement to show, or null if all voted. */
  get currentStatement(): StatementView | null {
    return this.statements.find(s => !this.votedIds.has(s.id)) ?? null;
  }

  get votedCount(): number {
    return this.votedIds.size;
  }

  get progressPercent(): number {
    return this.statements.length > 0 ? (this.votedCount / this.statements.length) * 100 : 0;
  }

  ngOnInit(): void {
    this.loadStatements();
  }

  async loadStatements(): Promise<void> {
    this.loading = true;
    this.error = '';

    try {
      this.statements = await this.governanceApi.getStatements(this.entityType(), this.entityId());
    } catch {
      this.error = 'Failed to load statements. Please try again.';
    } finally {
      this.loading = false;
    }
  }

  async vote(value: 'agree' | 'pass' | 'disagree'): Promise<void> {
    const statement = this.currentStatement;
    if (!statement || this.isVoting) return;

    this.isVoting = true;

    try {
      await this.governanceApi.voteOnStatement(statement.id, {
        humanId: this.humanId(),
        vote: value,
      });
      this.votedIds.add(statement.id);
    } catch {
      // Optimistically advance even on failure to keep the UX flowing.
      // The vote can be retried if the API recovers.
      this.votedIds.add(statement.id);
    } finally {
      this.isVoting = false;
    }
  }

  async submitStatement(): Promise<void> {
    const text = this.newStatementText.trim();
    if (!text || this.isSubmitting) return;

    this.isSubmitting = true;

    try {
      const input: CreateStatementInputView = {
        entityType: this.entityType(),
        entityId: this.entityId(),
        humanId: this.humanId(),
        text,
      };
      const created = await this.governanceApi.submitStatement(input);
      this.statements.push(created);
      this.newStatementText = '';
      this.submitted = true;
    } catch {
      // Submission failed — leave the text so the user can retry
    } finally {
      this.isSubmitting = false;
    }
  }
}
