/**
 * FileAppealComponent - Form for filing an appeal against a challenge response.
 *
 * Standalone, signal-based component with inline template.
 * Collects appeal grounds and optional additional evidence,
 * then calls GovernanceApiService.fileAppeal().
 */

import { Component, computed, inject, input, output, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';

import type { AppealView, FileAppealInputView } from '@elohim/storage-client/generated';

import { GovernanceApiService } from '@app/elohim/services/governance-api.service';

/** Appeal grounds options matching the protocol's governance model. */
const APPEAL_GROUNDS_OPTIONS = [
  { value: 'new-evidence', label: 'New Evidence' },
  { value: 'procedural-error', label: 'Procedural Error' },
  { value: 'disproportionate-response', label: 'Disproportionate Response' },
] as const;

@Component({
  selector: 'qahal-file-appeal',
  standalone: true,
  imports: [FormsModule],
  template: `
    <form class="file-appeal-form" (ngSubmit)="onSubmit()">
      <h3 class="form-title">File an Appeal</h3>

      <label class="field">
        <span class="field-label">Grounds for Appeal</span>
        <select
          [ngModel]="grounds()"
          (ngModelChange)="grounds.set($event)"
          name="grounds"
          required
          data-testid="appeal-grounds-select">
          <option value="" disabled>Select grounds for appeal</option>
          @for (opt of groundsOptions; track opt.value) {
            <option [value]="opt.value">{{ opt.label }}</option>
          }
        </select>
      </label>

      <label class="field">
        <span class="field-label">Additional Evidence (optional)</span>
        <textarea
          [ngModel]="additionalEvidence()"
          (ngModelChange)="additionalEvidence.set($event)"
          name="additionalEvidence"
          rows="4"
          placeholder="Provide any additional evidence supporting your appeal..."
          data-testid="appeal-evidence-textarea"
        ></textarea>
      </label>

      @if (errorMessage()) {
        <div class="form-error">{{ errorMessage() }}</div>
      }

      <button
        type="submit"
        class="submit-btn"
        [disabled]="!canSubmit() || submitting()"
        data-testid="submit-appeal-btn">
        @if (submitting()) {
          Filing...
        } @else {
          File Appeal
        }
      </button>
    </form>
  `,
  styles: `
    :host {
      display: block;
    }

    .file-appeal-form {
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
      background: var(--warning, #e6a23c);
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
export class FileAppealComponent {
  /** The challenge being appealed. */
  challengeId = input.required<string>();

  /** Emitted when an appeal is successfully filed. */
  appealFiled = output<AppealView>();

  private readonly governanceApi = inject(GovernanceApiService);

  // --- Form state ---
  readonly grounds = signal('');
  readonly additionalEvidence = signal('');
  readonly submitting = signal(false);
  readonly errorMessage = signal('');

  readonly groundsOptions = APPEAL_GROUNDS_OPTIONS;

  /** Form is valid when grounds are selected. */
  readonly canSubmit = computed(() => this.grounds().length > 0 && !this.submitting());

  async onSubmit(): Promise<void> {
    if (!this.canSubmit()) return;

    this.submitting.set(true);
    this.errorMessage.set('');

    const payload: FileAppealInputView = {
      grounds: this.grounds(),
      additionalEvidence: this.additionalEvidence().trim() || null,
    };

    try {
      const result = await this.governanceApi.fileAppeal(this.challengeId(), payload);
      this.appealFiled.emit(result);
      this.resetForm();
    } catch {
      this.errorMessage.set('Failed to file appeal. Please try again.');
    } finally {
      this.submitting.set(false);
    }
  }

  private resetForm(): void {
    this.grounds.set('');
    this.additionalEvidence.set('');
  }
}
