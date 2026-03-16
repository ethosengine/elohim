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
import {
  MechanismSelectionService,
  type MechanismSelection,
} from '../../services/mechanism-selection.service';

@Component({
  selector: 'qahal-feedback-mechanism-gateway',
  standalone: true,
  imports: [
    ContextMenuOnlyComponent,
    ReactionBarComponent,
    GraduatedFeedbackComponent,
    PsephosBallotWrapperComponent,
    FileChallengeComponent,
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

    @media (prefers-color-scheme: dark) {
      .challenge-panel {
        background: var(--surface, #1a1a1a);
        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
      }

      .challenge-close:hover {
        background: var(--surface-elevated, #2a2a2a);
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

  /** Whether the inline challenge form is currently shown. */
  readonly showChallengeForm = signal(false);

  /** Loaded governance state for this entity. */
  private readonly governanceState = signal<GovernanceStateView | null>(null);

  /** Active proposal for this entity (if any). */
  private readonly activeProposal = signal<ProposalView | undefined>(undefined);

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

    const [state, proposals] = await Promise.all([
      this.governanceApi.getGovernanceState(entityType, entityId),
      this.governanceApi.queryProposals(entityId, 'active'),
    ]);

    this.governanceState.set(state);
    this.activeProposal.set(proposals.length > 0 ? proposals[0] : undefined);
    this.loaded.set(true);
  }
}
