/**
 * PsephosBallotWrapperComponent - Psephos Integration Seam
 *
 * Bridges the FeedbackMechanismGateway (qahal) to the PsephosBallotComponent
 * (psephos-plugin). Loads proposal options, builds ballot props, mounts the
 * Psephos custom element, handles BallotRecognition callbacks, and submits
 * ranked votes to the backend.
 *
 * Governance levels 3-7 route here via the gateway's renderTarget === 'psephos'.
 */

import {
  Component,
  CUSTOM_ELEMENTS_SCHEMA,
  computed,
  effect,
  inject,
  input,
  output,
  signal,
} from '@angular/core';
import { DecimalPipe, TitleCasePipe } from '@angular/common';

import type {
  ProposalView,
  ProposalOptionView,
  CastRankedVoteInputView,
  TallyResult,
  BallotEntry,
} from '@elohim/storage-client/generated';

import { GovernanceApiService } from '@app/elohim/services/governance-api.service';

/** Shape of the ballot object passed to PsephosBallotComponent. */
interface PsephosBallot {
  mechanism: string;
  options: {
    id: string;
    label: string;
    description: string;
    position: number;
    source: string | null;
    sourceJustification: string | null;
  }[];
  config: {
    scoreMin: number | null;
    scoreMax: number | null;
    dotsPerVoter: number | null;
  };
  context: {
    proposalId: string;
    title: string;
    description: string;
  };
  hygiene: {
    randomizeOrder: boolean;
    equalVisualWeight: boolean;
    requireReasoning: boolean;
    showResultsAfterVote: boolean;
    confirmBeforeSubmit: boolean;
    hideVoterCount: boolean;
  };
}

/** Recognition event emitted by PsephosBallotComponent on vote submission. */
interface BallotRecognition {
  ballots: BallotEntry[];
  reasoning?: string;
  humanId?: string;
}

@Component({
  selector: 'qahal-psephos-ballot-wrapper',
  standalone: true,
  imports: [DecimalPipe, TitleCasePipe],
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
  template: `
    @if (loading()) {
      <div class="ballot-loading">Loading ballot options...</div>
    } @else if (tallyResult()) {
      <div class="ballot-results">
        <h4>Results: {{ tallyResult()!.mechanism }}</h4>
        <div class="results-summary">
          {{ tallyResult()!.recommendation | titlecase }} &mdash;
          {{ tallyResult()!.totalVoters }} voters
        </div>
        @for (result of tallyResult()!.optionResults; track result.optionId) {
          <div class="option-result">
            <span class="label">{{ result.label }}</span>
            <span class="percentage">{{ result.percentage | number: '1.0-1' }}%</span>
            <div class="bar" [style.width.%]="result.percentage"></div>
          </div>
        }
      </div>
    } @else {
      <div class="ballot-container">
        <h4>{{ proposal().title }}</h4>
        <p class="mechanism-badge">{{ mechanism() }}</p>
        <app-psephos-ballot
          [ballot]="ballot()"
          (recognized)="onRecognized($any($event))" />
      </div>
    }
  `,
  styles: `
    :host {
      display: block;
    }

    .ballot-loading {
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 1rem;
      font-size: 0.875rem;
      color: var(--text-secondary, #999);
    }

    .ballot-container h4 {
      margin: 0 0 0.5rem;
      font-size: 1rem;
      font-weight: 600;
    }

    .mechanism-badge {
      display: inline-block;
      margin: 0 0 1rem;
      padding: 0.125rem 0.5rem;
      font-size: 0.75rem;
      font-weight: 500;
      text-transform: capitalize;
      color: var(--text-on-primary, #fff);
      background: var(--primary, #6366f1);
      border-radius: 9999px;
    }

    .ballot-results h4 {
      margin: 0 0 0.5rem;
      font-size: 1rem;
      font-weight: 600;
    }

    .results-summary {
      margin-bottom: 0.75rem;
      font-size: 0.875rem;
      color: var(--text-secondary, #666);
    }

    .option-result {
      position: relative;
      display: flex;
      align-items: center;
      gap: 0.5rem;
      padding: 0.5rem 0;
    }

    .option-result .label {
      flex: 1;
      font-size: 0.875rem;
    }

    .option-result .percentage {
      flex-shrink: 0;
      font-size: 0.8125rem;
      font-weight: 600;
      color: var(--text-secondary, #666);
    }

    .option-result .bar {
      position: absolute;
      bottom: 0;
      left: 0;
      height: 3px;
      background: var(--primary, #6366f1);
      border-radius: 2px;
      transition: width 0.3s ease;
    }
  `,
})
export class PsephosBallotWrapperComponent {
  /** The active proposal to vote on. */
  proposal = input.required<ProposalView>();

  /** The governance mechanism (e.g. 'ranked-choice', 'consent', 'score', 'dot', 'approval'). */
  mechanism = input.required<string>();

  /** Emitted after a ballot is successfully submitted and tallied. */
  ballotSubmitted = output<TallyResult>();

  private readonly governanceApi = inject(GovernanceApiService);

  /** Whether we are loading proposal options from the backend. */
  readonly loading = signal(true);

  /** Loaded proposal options. */
  private readonly options = signal<ProposalOptionView[]>([]);

  /** Error message if loading or submission fails. */
  readonly error = signal<string | null>(null);

  /** Whether a vote submission is in progress. */
  readonly submitting = signal(false);

  /** Tally results displayed after successful vote. */
  readonly tallyResult = signal<TallyResult | null>(null);

  /** Ballot object computed from proposal + options, consumed by <app-psephos-ballot>. */
  readonly ballot = computed<PsephosBallot | null>(() => {
    const opts = this.options();
    if (opts.length === 0) return null;

    const prop = this.proposal();
    return {
      mechanism: this.mechanism(),
      options: opts.map((o) => ({
        id: o.id,
        label: o.label,
        description: o.description,
        position: o.position,
        source: o.source,
        sourceJustification: o.sourceJustification,
      })),
      config: {
        scoreMin: prop.scoreMin,
        scoreMax: prop.scoreMax,
        dotsPerVoter: prop.dotsPerVoter,
      },
      context: {
        proposalId: prop.id,
        title: prop.title,
        description: prop.body,
      },
      hygiene: {
        randomizeOrder: true,
        equalVisualWeight: true,
        requireReasoning: this.mechanism() === 'consent',
        showResultsAfterVote: true,
        confirmBeforeSubmit: true,
        hideVoterCount: true,
      },
    };
  });

  constructor() {
    // Load options whenever the proposal input changes
    effect(() => {
      const prop = this.proposal();
      this.loadOptions(prop.id);
    });
  }

  /**
   * Handle the (recognized) event from PsephosBallotComponent.
   * Converts the BallotRecognition into a CastRankedVoteInputView and submits.
   */
  async onRecognized(event: BallotRecognition): Promise<void> {
    const prop = this.proposal();
    this.submitting.set(true);
    this.error.set(null);

    try {
      const input: CastRankedVoteInputView = {
        humanId: event.humanId ?? '',
        ballots: event.ballots,
        reasoning: event.reasoning ?? null,
        proxyElohimId: null,
        proxyJustification: null,
      };

      await this.governanceApi.castRankedVotes(prop.id, input);

      // Load and display tally results
      const tally = await this.governanceApi.getTally(prop.id);
      this.tallyResult.set(tally);
      this.ballotSubmitted.emit(tally);
    } catch (err) {
      this.error.set('Failed to submit ballot. Please try again.');
      console.error('[PsephosBallotWrapper] Vote submission failed:', err);
    } finally {
      this.submitting.set(false);
    }
  }

  private async loadOptions(proposalId: string): Promise<void> {
    this.loading.set(true);
    this.error.set(null);

    try {
      const opts = await this.governanceApi.getProposalOptions(proposalId);
      this.options.set(opts);
    } catch (err) {
      this.error.set('Failed to load ballot options.');
      console.error('[PsephosBallotWrapper] Failed to load options:', err);
    } finally {
      this.loading.set(false);
    }
  }
}
