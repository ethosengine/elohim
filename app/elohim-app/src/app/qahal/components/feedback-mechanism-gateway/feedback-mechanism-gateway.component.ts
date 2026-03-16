/**
 * FeedbackMechanismGatewayComponent - Core Governance Gateway Shell
 *
 * Orchestrating component that loads governance state, calls
 * MechanismSelectionService, and renders the appropriate sub-component
 * based on renderTarget (angular for levels 0-2, psephos for levels 3-7).
 *
 * This is the single entry point for all governance feedback UI on any entity.
 * Host components only need to provide entityType and entityId — the gateway
 * handles mechanism selection, loading state, and routing to the correct renderer.
 */

import { Component, computed, effect, inject, input, signal } from '@angular/core';

import type { ChallengeView, GovernanceStateView, ProposalView } from '@elohim/storage-client';

import { GovernanceApiService } from '@app/elohim/services/governance-api.service';

import {
  ContextMenuOnlyComponent,
  type ContextMenuAction,
} from '../context-menu-only/context-menu-only.component';
import { ReactionBarComponent } from '../reaction-bar/reaction-bar.component';
import { GraduatedFeedbackComponent } from '../graduated-feedback/graduated-feedback.component';
import { PsephosBallotWrapperComponent } from '../psephos-ballot-wrapper/psephos-ballot-wrapper.component';
import { FileChallengeComponent } from '../file-challenge/file-challenge.component';
import { FeedbackAggregateComponent } from '../feedback-aggregate/feedback-aggregate.component';
import {
  MechanismSelectionService,
  type MechanismSelection,
} from '../../services/mechanism-selection.service';
import {
  SignalAccumulationService,
  type AccumulationStatus,
} from '../../services/signal-accumulation.service';

@Component({
  selector: 'qahal-feedback-mechanism-gateway',
  standalone: true,
  imports: [
    ContextMenuOnlyComponent,
    ReactionBarComponent,
    GraduatedFeedbackComponent,
    PsephosBallotWrapperComponent,
    FileChallengeComponent,
    FeedbackAggregateComponent,
  ],
  template: `
    @if (showChallengeForm()) {
      <div class="challenge-overlay">
        <div class="challenge-panel">
          <button
            class="challenge-close"
            type="button"
            aria-label="Close challenge form"
            (click)="closeChallengeForm()">
            &times;
          </button>
          <qahal-file-challenge
            [entityType]="entityType()"
            [entityId]="entityId()"
            (challengeFiled)="onChallengeFiled($event)" />
        </div>
      </div>
    }

    @if (selection(); as sel) {
      @if (sel.renderTarget === 'angular') {
        @switch (sel.level) {
          @case (0) {
            <qahal-context-menu-only
              [entityType]="entityType()"
              [entityId]="entityId()"
              (challenge)="onChallengeAction($event)"
              (flag)="onFlagAction($event)" />
          }
          @case (1) {
            <app-reaction-bar
              [entityType]="entityType()"
              [entityId]="entityId()" />
          }
          @case (2) {
            <app-graduated-feedback
              [entityType]="entityType()"
              [entityId]="entityId()" />
            <app-reaction-bar
              [entityType]="entityType()"
              [entityId]="entityId()" />
          }
        }
      }

      @if (sel.renderTarget === 'psephos' && sel.activeProposal) {
        <qahal-psephos-ballot-wrapper
          [proposal]="sel.activeProposal"
          [mechanism]="sel.mechanism"
          (ballotSubmitted)="onBallotSubmitted($event)" />
      }

      <!-- Signal aggregate - shows what others thought -->
      <app-feedback-aggregate
        [entityType]="entityType()"
        [entityId]="entityId()" />

      <!-- Accumulation status badges -->
      @if (accumulationStatus(); as status) {
        @if (status.readyForSensemaking) {
          <div class="accumulation-badge sensemaking">
            Diverse perspectives on this content — sensemaking available
          </div>
        }
        @if (status.controversyDetected) {
          <div class="accumulation-badge controversy">Active discussion</div>
        }
        @if (status.settled) {
          <div class="accumulation-badge settled">Community consensus</div>
        }
      }
    } @else {
      <div class="gateway-loading">Loading governance...</div>
    }
  `,
  styles: `
    :host {
      display: block;
      position: relative;
    }

    .gateway-loading {
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 0.75rem;
      font-size: 0.8125rem;
      color: var(--text-secondary, #999);
    }

    .challenge-overlay {
      position: fixed;
      inset: 0;
      z-index: 1000;
      display: flex;
      align-items: center;
      justify-content: center;
      background: rgba(0, 0, 0, 0.5);
    }

    .challenge-panel {
      position: relative;
      width: 90%;
      max-width: 560px;
      max-height: 90vh;
      overflow-y: auto;
      background: var(--surface, #fff);
      border-radius: 12px;
      box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
    }

    .challenge-close {
      position: absolute;
      top: 0.5rem;
      right: 0.5rem;
      width: 32px;
      height: 32px;
      display: flex;
      align-items: center;
      justify-content: center;
      border: none;
      border-radius: 6px;
      background: transparent;
      font-size: 1.25rem;
      color: var(--text-secondary, #666);
      cursor: pointer;
      z-index: 1;
    }

    .challenge-close:hover {
      background: var(--surface-elevated, #f5f5f5);
      color: var(--text-primary, #333);
    }

    .accumulation-badge {
      display: inline-block;
      margin-top: 0.5rem;
      padding: 0.25rem 0.625rem;
      font-size: 0.75rem;
      font-weight: 500;
      border-radius: 999px;
      line-height: 1.4;
    }

    .accumulation-badge.sensemaking {
      background: var(--sensemaking-bg, #e8f0fe);
      color: var(--sensemaking-fg, #1a56db);
    }

    .accumulation-badge.controversy {
      background: var(--controversy-bg, #fef3c7);
      color: var(--controversy-fg, #92400e);
    }

    .accumulation-badge.settled {
      background: var(--settled-bg, #d1fae5);
      color: var(--settled-fg, #065f46);
    }

    @media (prefers-color-scheme: dark) {
      .challenge-panel {
        background: var(--surface, #1a1a1a);
        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
      }

      .challenge-close:hover {
        background: var(--surface-elevated, #2a2a2a);
      }

      .accumulation-badge.sensemaking {
        background: var(--sensemaking-bg, #1e3a5f);
        color: var(--sensemaking-fg, #93bbfc);
      }

      .accumulation-badge.controversy {
        background: var(--controversy-bg, #4a3520);
        color: var(--controversy-fg, #fbbf24);
      }

      .accumulation-badge.settled {
        background: var(--settled-bg, #14432a);
        color: var(--settled-fg, #6ee7b7);
      }
    }
  `,
})
export class FeedbackMechanismGatewayComponent {
  /** The type of entity (e.g. 'content', 'path', 'proposal'). */
  entityType = input.required<string>();

  /** The unique identifier of the entity. */
  entityId = input.required<string>();

  /** Optional content type hint for mechanism selection (e.g. 'discussion', 'reflection'). */
  contentType = input<string>('learning-content');

  private readonly governanceApi = inject(GovernanceApiService);
  private readonly mechanismSelection = inject(MechanismSelectionService);
  private readonly signalAccumulation = inject(SignalAccumulationService);

  /** Whether the inline challenge form is currently shown. */
  readonly showChallengeForm = signal(false);

  /** Loaded governance state for this entity. */
  private readonly governanceState = signal<GovernanceStateView | null>(null);

  /** Active proposal for this entity (if any). */
  private readonly activeProposal = signal<ProposalView | undefined>(undefined);

  /** Accumulation status derived from signal aggregates. */
  readonly accumulationStatus = signal<AccumulationStatus | null>(null);

  /** Whether governance data has been loaded (distinguishes null-state from not-yet-loaded). */
  private readonly loaded = signal(false);

  /** The resolved mechanism selection, null while loading. */
  readonly selection = computed<MechanismSelection | null>(() => {
    if (!this.loaded()) return null;
    return this.mechanismSelection.selectMechanism(
      this.governanceState(),
      this.contentType(),
      this.activeProposal(),
    );
  });

  constructor() {
    // React to input changes and reload governance data
    effect(() => {
      const type = this.entityType();
      const id = this.entityId();
      this.loadGovernanceData(type, id);
    });
  }

  /**
   * Handle "Challenge" action from ContextMenuOnly.
   * Opens the inline challenge form overlay.
   */
  onChallengeAction(_event: ContextMenuAction): void {
    this.showChallengeForm.set(true);
  }

  /**
   * Handle "Flag" action from ContextMenuOnly.
   * Navigates to the community governance area (placeholder for future flag handling).
   */
  onFlagAction(_event: ContextMenuAction): void {
    // Flag handling will be wired in a future sprint
  }

  /** Close the inline challenge form. */
  closeChallengeForm(): void {
    this.showChallengeForm.set(false);
  }

  /** Handle successful challenge filing — close form and reload governance state. */
  onChallengeFiled(_challenge: ChallengeView): void {
    this.showChallengeForm.set(false);
    this.loadGovernanceData(this.entityType(), this.entityId());
  }

  /**
   * Handle ballot submission from PsephosBallotWrapper (Task 6).
   * Will be wired when PsephosBallotWrapper is created.
   */
  onBallotSubmitted(_event: unknown): void {
    // Reload governance data after a ballot is submitted
    this.loadGovernanceData(this.entityType(), this.entityId());
  }

  private async loadGovernanceData(entityType: string, entityId: string): Promise<void> {
    this.loaded.set(false);

    const [state, proposals, accumulation] = await Promise.all([
      this.governanceApi.getGovernanceState(entityType, entityId),
      this.governanceApi.queryProposals(entityId, 'active'),
      this.signalAccumulation.getAccumulationStatus(entityType, entityId),
    ]);

    this.governanceState.set(state);
    this.activeProposal.set(proposals.length > 0 ? proposals[0] : undefined);
    this.accumulationStatus.set(accumulation);
    this.loaded.set(true);
  }
}
