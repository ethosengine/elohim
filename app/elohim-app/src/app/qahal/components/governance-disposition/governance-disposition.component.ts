/**
 * GovernanceDispositionComponent — Displays and edits the governance profile.
 *
 * Shows three disposition axes (risk tolerance, change openness, consensus
 * preference), priority values, participation stats, and voting pattern
 * summary. Toggle into edit mode to manually adjust the sliders, or
 * recompute the profile from voting history.
 */

import { Component, inject, input, OnInit, signal, computed } from '@angular/core';

import { GovernanceApiService } from '@app/elohim/services/governance-api.service';

import type {
  GovernanceDispositionView,
  UpdateDispositionInputView,
  JsonValue,
} from '@elohim/storage-client/generated';

@Component({
  selector: 'qahal-governance-disposition',
  standalone: true,
  template: `
    @if (disposition()) {
      <div class="disposition-card">
        <h2>Your Governance Profile</h2>

        <!-- Risk Tolerance -->
        <div class="axis">
          <label id="risk-tolerance-label">Risk Tolerance</label>
          <div class="range-row">
            <span class="range-label-left">Conservative</span>
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              aria-labelledby="risk-tolerance-label"
              data-testid="risk-tolerance-slider"
              [value]="editRiskTolerance()"
              [disabled]="!editing()"
              (input)="onRiskToleranceChange($event)"
            />
            <span class="range-label-right">Progressive</span>
          </div>
          <span class="range-value">{{ (editRiskTolerance() * 100).toFixed(0) }}%</span>
        </div>

        <!-- Change Openness -->
        <div class="axis">
          <label id="change-openness-label">Change Openness</label>
          <div class="range-row">
            <span class="range-label-left">Cautious</span>
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              aria-labelledby="change-openness-label"
              data-testid="change-openness-slider"
              [value]="editChangeOpenness()"
              [disabled]="!editing()"
              (input)="onChangeOpennessChange($event)"
            />
            <span class="range-label-right">Embrace change</span>
          </div>
          <span class="range-value">{{ (editChangeOpenness() * 100).toFixed(0) }}%</span>
        </div>

        <!-- Consensus Preference -->
        <div class="axis">
          <label id="consensus-preference-label">Consensus Preference</label>
          <div class="range-row">
            <span class="range-label-left">Independent</span>
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              aria-labelledby="consensus-preference-label"
              data-testid="consensus-preference-slider"
              [value]="editConsensusPreference()"
              [disabled]="!editing()"
              (input)="onConsensusPreferenceChange($event)"
            />
            <span class="range-label-right">Consensus-seeking</span>
          </div>
          <span class="range-value">{{ (editConsensusPreference() * 100).toFixed(0) }}%</span>
        </div>

        <!-- Priority Values -->
        @if (priorityValueTags().length > 0) {
          <div class="priority-values">
            <label>Priority Values</label>
            <div class="tags">
              @for (tag of priorityValueTags(); track tag) {
                <span class="tag">{{ tag }}</span>
              }
            </div>
          </div>
        }

        <!-- Stats -->
        <div class="stats">
          {{ disposition()!.totalVotesCast }} votes cast &middot;
          {{ disposition()!.totalChallengesFiled }} challenges filed &middot;
          {{ disposition()!.totalSignalsRecorded }} signals recorded
        </div>

        <!-- Voting Pattern -->
        @if (votingPatternText()) {
          <div class="voting-pattern">
            <label>Voting Pattern</label>
            <p>{{ votingPatternText() }}</p>
          </div>
        }

        <!-- Actions -->
        <div class="actions">
          @if (!editing()) {
            <button
              class="btn btn-secondary"
              data-testid="adjust-manually-btn"
              (click)="startEditing()"
            >
              Adjust manually
            </button>
            <button
              class="btn btn-secondary"
              data-testid="recompute-btn"
              (click)="recompute()"
              [disabled]="recomputing()"
            >
              {{ recomputing() ? 'Recomputing...' : 'Recompute from history' }}
            </button>
          } @else {
            <button
              class="btn btn-primary"
              data-testid="save-disposition-btn"
              (click)="save()"
              [disabled]="saving()"
            >
              {{ saving() ? 'Saving...' : 'Save' }}
            </button>
            <button
              class="btn btn-secondary"
              data-testid="cancel-edit-btn"
              (click)="cancelEditing()"
            >
              Cancel
            </button>
          }
        </div>
      </div>
    } @else if (loading()) {
      <div class="loading">Loading governance profile...</div>
    } @else {
      <div class="empty">No governance profile found.</div>
    }
  `,
  styles: `
    .disposition-card {
      max-width: 600px;
      margin: 0 auto;
      padding: 1.5rem;
    }

    h2 {
      margin: 0 0 1.5rem;
      font-size: 1.375rem;
      font-weight: 600;
    }

    .axis {
      margin-bottom: 1.25rem;
    }

    .axis > label {
      display: block;
      margin-bottom: 0.25rem;
      font-size: 0.875rem;
      font-weight: 600;
    }

    .range-row {
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .range-label-left,
    .range-label-right {
      font-size: 0.75rem;
      color: var(--text-secondary, #666);
      white-space: nowrap;
      min-width: 6rem;
    }

    .range-label-left {
      text-align: right;
    }

    .range-label-right {
      text-align: left;
    }

    input[type='range'] {
      flex: 1;
      height: 0.375rem;
      appearance: none;
      background: var(--surface-variant, #e0e0e0);
      border-radius: 0.1875rem;
      outline: none;
    }

    input[type='range']::-webkit-slider-thumb {
      appearance: none;
      width: 1rem;
      height: 1rem;
      border-radius: 50%;
      background: var(--primary-color, #2196f3);
      cursor: pointer;
    }

    input[type='range']:disabled::-webkit-slider-thumb {
      background: var(--text-secondary, #999);
      cursor: default;
    }

    .range-value {
      display: block;
      text-align: center;
      font-size: 0.75rem;
      color: var(--text-secondary, #666);
      margin-top: 0.125rem;
    }

    .priority-values {
      margin-bottom: 1rem;
    }

    .priority-values > label {
      display: block;
      margin-bottom: 0.375rem;
      font-size: 0.875rem;
      font-weight: 600;
    }

    .tags {
      display: flex;
      flex-wrap: wrap;
      gap: 0.375rem;
    }

    .tag {
      display: inline-block;
      padding: 0.125rem 0.625rem;
      font-size: 0.75rem;
      border-radius: 999px;
      background: var(--surface-variant, #e8e8e8);
      color: var(--text-primary, #333);
    }

    .stats {
      margin-bottom: 0.75rem;
      font-size: 0.8125rem;
      color: var(--text-secondary, #666);
    }

    .voting-pattern {
      margin-bottom: 1rem;
    }

    .voting-pattern > label {
      display: block;
      margin-bottom: 0.25rem;
      font-size: 0.875rem;
      font-weight: 600;
    }

    .voting-pattern p {
      margin: 0;
      font-size: 0.8125rem;
      color: var(--text-secondary, #666);
    }

    .actions {
      display: flex;
      gap: 0.5rem;
      margin-top: 1rem;
    }

    .btn {
      padding: 0.5rem 1rem;
      border: none;
      border-radius: 0.375rem;
      font-size: 0.8125rem;
      font-weight: 500;
      cursor: pointer;
      transition: opacity 0.15s ease;
    }

    .btn:hover:not(:disabled) {
      opacity: 0.9;
    }

    .btn:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }

    .btn-primary {
      background: var(--primary-color, #2196f3);
      color: #fff;
    }

    .btn-secondary {
      background: var(--surface-variant, #e0e0e0);
      color: var(--text-primary, #333);
    }

    .loading,
    .empty {
      text-align: center;
      padding: 3rem 1rem;
      color: var(--text-secondary, #666);
      font-size: 0.875rem;
    }

    @media (prefers-color-scheme: dark) {
      .tag {
        background: var(--surface-variant, #333);
        color: var(--text-primary, #e0e0e0);
      }

      .btn-secondary {
        background: var(--surface-variant, #333);
        color: var(--text-primary, #e0e0e0);
      }
    }
  `,
})
export class GovernanceDispositionComponent implements OnInit {
  readonly humanId = input.required<string>();

  private readonly governanceApi = inject(GovernanceApiService);

  /** The loaded disposition. */
  readonly disposition = signal<GovernanceDispositionView | null>(null);
  readonly loading = signal(true);
  readonly editing = signal(false);
  readonly saving = signal(false);
  readonly recomputing = signal(false);

  /** Edit-mode slider values (separate from disposition to allow cancel). */
  readonly editRiskTolerance = signal(0);
  readonly editChangeOpenness = signal(0);
  readonly editConsensusPreference = signal(0);

  /** Parsed priority values as string tags. */
  readonly priorityValueTags = computed<string[]>(() => {
    const d = this.disposition();
    if (!d) return [];
    return parsePriorityValues(d.priorityValues);
  });

  /** Human-readable voting pattern summary. */
  readonly votingPatternText = computed<string>(() => {
    const d = this.disposition();
    if (!d) return '';
    return parseVotingPattern(d.votingPatternSummary);
  });

  async ngOnInit(): Promise<void> {
    this.loading.set(true);
    try {
      const result = await this.governanceApi.getDisposition(this.humanId());
      this.disposition.set(result);
      if (result) {
        this.syncEditValues(result);
      }
    } finally {
      this.loading.set(false);
    }
  }

  startEditing(): void {
    const d = this.disposition();
    if (d) {
      this.syncEditValues(d);
    }
    this.editing.set(true);
  }

  cancelEditing(): void {
    const d = this.disposition();
    if (d) {
      this.syncEditValues(d);
    }
    this.editing.set(false);
  }

  async save(): Promise<void> {
    this.saving.set(true);
    try {
      const overrides: UpdateDispositionInputView = {
        riskTolerance: this.editRiskTolerance(),
        changeOpenness: this.editChangeOpenness(),
        consensusPreference: this.editConsensusPreference(),
        priorityValues: null,
      };
      const updated = await this.governanceApi.updateDisposition(this.humanId(), overrides);
      this.disposition.set(updated);
      this.editing.set(false);
    } finally {
      this.saving.set(false);
    }
  }

  async recompute(): Promise<void> {
    this.recomputing.set(true);
    try {
      const result = await this.governanceApi.computeDisposition(this.humanId());
      this.disposition.set(result);
      this.syncEditValues(result);
    } finally {
      this.recomputing.set(false);
    }
  }

  onRiskToleranceChange(event: Event): void {
    this.editRiskTolerance.set(Number((event.target as HTMLInputElement).value));
  }

  onChangeOpennessChange(event: Event): void {
    this.editChangeOpenness.set(Number((event.target as HTMLInputElement).value));
  }

  onConsensusPreferenceChange(event: Event): void {
    this.editConsensusPreference.set(Number((event.target as HTMLInputElement).value));
  }

  private syncEditValues(d: GovernanceDispositionView): void {
    this.editRiskTolerance.set(d.riskTolerance);
    this.editChangeOpenness.set(d.changeOpenness);
    this.editConsensusPreference.set(d.consensusPreference);
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Extract string tags from the priorityValues JSON field. */
function parsePriorityValues(values: JsonValue): string[] {
  if (Array.isArray(values)) {
    return values.filter((v): v is string => typeof v === 'string');
  }
  return [];
}

/**
 * Parse the votingPatternSummary JSON into a readable string.
 * Expected shape: { "consent": 60, "ranked-choice": 25, "approval": 15 }
 */
function parseVotingPattern(summary: JsonValue): string {
  if (!summary || typeof summary !== 'object' || Array.isArray(summary)) return '';
  const entries = Object.entries(summary as Record<string, JsonValue>)
    .filter(([, v]) => typeof v === 'number' && v > 0)
    .sort(([, a], [, b]) => (b as number) - (a as number));
  if (entries.length === 0) return '';
  return 'Mostly ' + entries.map(([mechanism, pct]) => `${mechanism} (${pct}%)`).join(', ');
}
