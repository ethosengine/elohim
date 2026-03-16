/**
 * RespondToChallengeComponent - Form for responding to a governance challenge.
 *
 * Standalone, signal-based component with inline template.
 * Collects outcome, reasoning, actions taken, and precedent flag,
 * then calls GovernanceApiService.respondToChallenge().
 */

import { Component, computed, inject, input, output, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';

import type { ChallengeView, RespondToChallengeInputView } from '@elohim/storage-client/generated';

import { GovernanceApiService } from '@app/elohim/services/governance-api.service';

/** Outcome radio options. */
const OUTCOME_OPTIONS = [
  { value: 'upheld', label: 'Upheld' },
  { value: 'partially-upheld', label: 'Partially Upheld' },
  { value: 'rejected', label: 'Rejected' },
] as const;

@Component({
  selector: 'qahal-respond-to-challenge',
  standalone: true,
  imports: [FormsModule],
  template: `
    <form class="respond-form" (ngSubmit)="onSubmit()">
      <h3 class="form-title">Respond to Challenge</h3>

      <!-- Outcome radio buttons -->
      <fieldset class="field">
        <legend class="field-label">Outcome</legend>
        <div class="radio-group">
          @for (opt of outcomeOptions; track opt.value) {
            <label class="radio-option">
              <input
                type="radio"
                name="outcome"
                [value]="opt.value"
                [ngModel]="outcome()"
                (ngModelChange)="outcome.set($event)"
                data-testid="outcome-radio" />
              {{ opt.label }}
            </label>
          }
        </div>
      </fieldset>

      <!-- Reasoning -->
      <label class="field">
        <span class="field-label">Reasoning</span>
        <textarea
          [ngModel]="reasoning()"
          (ngModelChange)="reasoning.set($event)"
          name="reasoning"
          required
          [minlength]="200"
          rows="6"
          placeholder="Provide detailed reasoning for your response (minimum 200 characters)..."
          data-testid="reasoning-textarea"
        ></textarea>
        @if (reasoning().length > 0 && reasoning().length < 200) {
          <span class="field-hint field-hint--warning">
            {{ 200 - reasoning().length }} more characters required
          </span>
        }
      </label>

      <!-- Actions taken (required if upheld or partially-upheld) -->
      @if (actionsRequired()) {
        <label class="field">
          <span class="field-label">Actions Taken</span>
          <textarea
            [ngModel]="actions()"
            (ngModelChange)="actions.set($event)"
            name="actions"
            required
            rows="4"
            placeholder="Describe actions taken to address the challenge..."
            data-testid="actions-textarea"
          ></textarea>
        </label>
      }

      <!-- Sets precedent -->
      <label class="checkbox-field">
        <input
          type="checkbox"
          [ngModel]="setsPrecedent()"
          (ngModelChange)="setsPrecedent.set($event)"
          name="setsPrecedent"
          data-testid="sets-precedent-checkbox" />
        <span>This response sets a precedent for future challenges</span>
      </label>

      @if (errorMessage()) {
        <div class="form-error">{{ errorMessage() }}</div>
      }

      <button
        type="submit"
        class="submit-btn"
        [disabled]="!canSubmit() || submitting()"
        data-testid="submit-response-btn">
        @if (submitting()) {
          Submitting...
        } @else {
          Submit Response
        }
      </button>
    </form>
  `,
  styles: `
    :host {
      display: block;
    }

    .respond-form {
      display: flex;
      flex-direction: column;
      gap: 1rem;
      padding: 1rem;
    }

    .form-title {
      margin: 0 0 0.5rem;
      font-size: 1.125rem;
      font-weight: 600;
      color: var(--text-primary, #333);
    }

    .field {
      display: flex;
      flex-direction: column;
      gap: 0.25rem;
    }

    fieldset.field {
      border: none;
      padding: 0;
      margin: 0;
    }

    .field-label,
    legend.field-label {
      font-size: 0.8125rem;
      font-weight: 500;
      color: var(--text-secondary, #666);
      padding: 0;
    }

    .field-hint {
      font-size: 0.75rem;
      color: var(--text-tertiary, #999);
    }

    .field-hint--warning {
      color: var(--warning, #e6a23c);
    }

    .radio-group {
      display: flex;
      gap: 1rem;
      flex-wrap: wrap;
    }

    .radio-option {
      display: flex;
      align-items: center;
      gap: 0.375rem;
      font-size: 0.875rem;
      color: var(--text-primary, #333);
      cursor: pointer;
    }

    .checkbox-field {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      font-size: 0.875rem;
      color: var(--text-primary, #333);
      cursor: pointer;
    }

    textarea {
      padding: 0.5rem;
      border: 1px solid var(--border-color, #ddd);
      border-radius: 4px;
      font-size: 0.875rem;
      font-family: inherit;
      background: var(--surface, #fff);
      color: var(--text-primary, #333);
      resize: vertical;
      min-height: 4rem;
    }

    .form-error {
      padding: 0.5rem 0.75rem;
      border-radius: 4px;
      background: var(--error-bg, #fef0f0);
      color: var(--error, #f56c6c);
      font-size: 0.8125rem;
    }

    .submit-btn {
      align-self: flex-start;
      padding: 0.5rem 1.25rem;
      border: none;
      border-radius: 4px;
      background: var(--primary, #409eff);
      color: #fff;
      font-size: 0.875rem;
      font-weight: 500;
      cursor: pointer;
      transition: opacity 0.2s;
    }

    .submit-btn:hover:not(:disabled) {
      opacity: 0.85;
    }

    .submit-btn:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
  `,
})
export class RespondToChallengeComponent {
  /** The challenge being responded to. */
  challengeId = input.required<string>();

  /** Emitted when the response is successfully submitted. */
  responded = output<ChallengeView>();

  private readonly governanceApi = inject(GovernanceApiService);

  // --- Form state ---
  readonly outcome = signal('');
  readonly reasoning = signal('');
  readonly actions = signal('');
  readonly setsPrecedent = signal(false);
  readonly submitting = signal(false);
  readonly errorMessage = signal('');

  readonly outcomeOptions = OUTCOME_OPTIONS;

  /** Actions textarea is required when outcome is upheld or partially-upheld. */
  readonly actionsRequired = computed(
    () => this.outcome() === 'upheld' || this.outcome() === 'partially-upheld'
  );

  /** Form is valid when all required fields are filled. */
  readonly canSubmit = computed(() => {
    if (!this.outcome() || this.reasoning().length < 200) return false;
    if (this.actionsRequired() && !this.actions().trim()) return false;
    return !this.submitting();
  });

  async onSubmit(): Promise<void> {
    if (!this.canSubmit()) return;

    this.submitting.set(true);
    this.errorMessage.set('');

    const payload: RespondToChallengeInputView = {
      outcome: this.outcome(),
      reasoning: this.reasoning(),
      actions: this.actionsRequired() ? this.actions() : null,
      setsPrecedent: this.setsPrecedent(),
    };

    try {
      const result = await this.governanceApi.respondToChallenge(this.challengeId(), payload);
      this.responded.emit(result);
      this.resetForm();
    } catch {
      this.errorMessage.set('Failed to submit response. Please try again.');
    } finally {
      this.submitting.set(false);
    }
  }

  private resetForm(): void {
    this.outcome.set('');
    this.reasoning.set('');
    this.actions.set('');
    this.setsPrecedent.set(false);
  }
}
