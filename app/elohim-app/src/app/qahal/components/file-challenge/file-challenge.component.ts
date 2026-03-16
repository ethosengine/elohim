/**
 * FileChallengeComponent - Form for filing a governance challenge against an entity.
 *
 * Standalone, signal-based component with inline template.
 * Collects grounds, evidence, standing basis, and optional requested outcome,
 * then calls GovernanceApiService.fileChallenge().
 */

import { Component, computed, inject, input, output, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';

import type { ChallengeView, FileChallengeInputView } from '@elohim/storage-client/generated';

import { GovernanceApiService } from '@app/elohim/services/governance-api.service';

/** Challenge grounds options matching the protocol's governance model. */
const GROUNDS_OPTIONS = [
  { value: 'factual-error', label: 'Factual Error' },
  { value: 'misapplication', label: 'Misapplication' },
  { value: 'bias', label: 'Bias' },
  { value: 'inconsistency', label: 'Inconsistency' },
  { value: 'outdated', label: 'Outdated' },
  { value: 'harmful', label: 'Harmful' },
  { value: 'procedural', label: 'Procedural' },
] as const;

/** Standing basis options — how the challenger has standing to file. */
const STANDING_OPTIONS = [
  { value: 'steward', label: 'Steward of this content' },
  { value: 'learner', label: 'Active learner' },
  { value: 'community-member', label: 'Community member' },
  { value: 'subject-expert', label: 'Subject matter expert' },
] as const;

/** Requested outcome options. */
const OUTCOME_OPTIONS = [
  { value: '', label: 'No specific outcome requested' },
  { value: 'correction', label: 'Correction' },
  { value: 'retraction', label: 'Retraction' },
  { value: 'review', label: 'Review by stewards' },
  { value: 'amendment', label: 'Amendment' },
] as const;

@Component({
  selector: 'qahal-file-challenge',
  standalone: true,
  imports: [FormsModule],
  template: `
    <form class="file-challenge-form" (ngSubmit)="onSubmit()">
      <h3 class="form-title">File a Challenge</h3>

      <label class="field">
        <span class="field-label">Grounds</span>
        <select
          [ngModel]="grounds()"
          (ngModelChange)="grounds.set($event)"
          name="grounds"
          required>
          <option value="" disabled>Select grounds for challenge</option>
          @for (opt of groundsOptions; track opt.value) {
            <option [value]="opt.value">{{ opt.label }}</option>
          }
        </select>
      </label>

      <label class="field">
        <span class="field-label">Evidence</span>
        <textarea
          [ngModel]="evidence()"
          (ngModelChange)="evidence.set($event)"
          name="evidence"
          required
          [minlength]="100"
          rows="6"
          placeholder="Provide detailed evidence supporting your challenge (minimum 100 characters)..."
        ></textarea>
        @if (evidence().length > 0 && evidence().length < 100) {
          <span class="field-hint field-hint--warning">
            {{ 100 - evidence().length }} more characters required
          </span>
        }
      </label>

      <label class="field">
        <span class="field-label">Standing Basis</span>
        <select
          [ngModel]="standingBasis()"
          (ngModelChange)="standingBasis.set($event)"
          name="standingBasis"
          required>
          <option value="" disabled>How do you have standing to file?</option>
          @for (opt of standingOptions; track opt.value) {
            <option [value]="opt.value">{{ opt.label }}</option>
          }
        </select>
      </label>

      <label class="field">
        <span class="field-label">Requested Outcome (optional)</span>
        <select
          [ngModel]="requestedOutcome()"
          (ngModelChange)="requestedOutcome.set($event)"
          name="requestedOutcome">
          @for (opt of outcomeOptions; track opt.value) {
            <option [value]="opt.value">{{ opt.label }}</option>
          }
        </select>
      </label>

      @if (errorMessage()) {
        <div class="form-error">{{ errorMessage() }}</div>
      }

      <button
        type="submit"
        class="submit-btn"
        [disabled]="!canSubmit() || submitting()">
        @if (submitting()) {
          Filing...
        } @else {
          File Challenge
        }
      </button>
    </form>
  `,
  styles: `
    :host {
      display: block;
    }

    .file-challenge-form {
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

    .field-label {
      font-size: 0.8125rem;
      font-weight: 500;
      color: var(--text-secondary, #666);
    }

    .field-hint {
      font-size: 0.75rem;
      color: var(--text-tertiary, #999);
    }

    .field-hint--warning {
      color: var(--warning, #e6a23c);
    }

    select,
    textarea {
      padding: 0.5rem;
      border: 1px solid var(--border-color, #ddd);
      border-radius: 4px;
      font-size: 0.875rem;
      font-family: inherit;
      background: var(--surface, #fff);
      color: var(--text-primary, #333);
    }

    textarea {
      resize: vertical;
      min-height: 6rem;
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
export class FileChallengeComponent {
  /** The type of entity being challenged (e.g. 'content', 'path'). */
  entityType = input.required<string>();

  /** The unique identifier of the entity being challenged. */
  entityId = input.required<string>();

  /** Emitted when a challenge is successfully filed. */
  challengeFiled = output<ChallengeView>();

  private readonly governanceApi = inject(GovernanceApiService);

  // --- Form state ---
  readonly grounds = signal('');
  readonly evidence = signal('');
  readonly standingBasis = signal('');
  readonly requestedOutcome = signal('');
  readonly submitting = signal(false);
  readonly errorMessage = signal('');

  // --- Options for template ---
  readonly groundsOptions = GROUNDS_OPTIONS;
  readonly standingOptions = STANDING_OPTIONS;
  readonly outcomeOptions = OUTCOME_OPTIONS;

  /** Form is valid when required fields are filled and evidence meets minimum length. */
  readonly canSubmit = computed(
    () =>
      this.grounds().length > 0 &&
      this.evidence().length >= 100 &&
      this.standingBasis().length > 0 &&
      !this.submitting()
  );

  private static readonly CURRENT_USER_ID = 'current-user';

  async onSubmit(): Promise<void> {
    if (!this.canSubmit()) return;

    this.submitting.set(true);
    this.errorMessage.set('');

    const input: FileChallengeInputView = {
      entityType: this.entityType(),
      entityId: this.entityId(),
      challengerId: FileChallengeComponent.CURRENT_USER_ID,
      standingBasis: this.standingBasis(),
      groundsPrimary: this.grounds(),
      groundsSecondary: null,
      evidence: this.evidence(),
      requestedOutcome: this.requestedOutcome() || null,
    };

    try {
      const challenge = await this.governanceApi.fileChallenge(input);
      this.challengeFiled.emit(challenge);
      this.resetForm();
    } catch {
      this.errorMessage.set('Failed to file challenge. Please try again.');
    } finally {
      this.submitting.set(false);
    }
  }

  private resetForm(): void {
    this.grounds.set('');
    this.evidence.set('');
    this.standingBasis.set('');
    this.requestedOutcome.set('');
  }
}
