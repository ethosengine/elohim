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

import type { GovernanceStateView, ProposalView } from '@elohim/storage-client';

import { GovernanceApiService } from '@app/elohim/services/governance-api.service';

import { ContextMenuOnlyComponent } from '../context-menu-only/context-menu-only.component';
import { ReactionBarComponent } from '../reaction-bar/reaction-bar.component';
import { GraduatedFeedbackComponent } from '../graduated-feedback/graduated-feedback.component';
import {
  MechanismSelectionService,
  type MechanismSelection,
} from '../../services/mechanism-selection.service';

@Component({
  selector: 'qahal-feedback-mechanism-gateway',
  standalone: true,
  imports: [ContextMenuOnlyComponent, ReactionBarComponent, GraduatedFeedbackComponent],
  template: `
    @if (selection(); as sel) {
      @if (sel.renderTarget === 'angular') {
        @switch (sel.level) {
          @case (0) {
            <qahal-context-menu-only
              [entityType]="entityType()"
              [entityId]="entityId()" />
          }
          @case (1) {
            <app-reaction-bar [contentId]="entityId()" />
          }
          @case (2) {
            <app-graduated-feedback [contentId]="entityId()" />
            <app-reaction-bar [contentId]="entityId()" />
          }
        }
      }

      @if (sel.renderTarget === 'psephos') {
        <!-- PsephosBallotWrapper (Task 6) — not yet created -->
        <div class="psephos-placeholder"
          data-proposal-id="{{ sel.activeProposal?.id }}"
          data-mechanism="{{ sel.mechanism }}">
          Formal governance ballot loading...
        </div>
      }
    } @else {
      <div class="gateway-loading">Loading governance...</div>
    }
  `,
  styles: `
    :host {
      display: block;
    }

    .gateway-loading {
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 0.75rem;
      font-size: 0.8125rem;
      color: var(--text-secondary, #999);
    }

    .psephos-placeholder {
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 1rem;
      font-size: 0.875rem;
      color: var(--text-secondary, #666);
      border: 1px dashed var(--border, #e5e5e5);
      border-radius: 8px;
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
