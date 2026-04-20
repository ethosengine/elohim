import { CommonModule } from '@angular/common';
import { Component, OnDestroy, OnInit, inject, input } from '@angular/core';

import { Subject, takeUntil } from 'rxjs';

import { GovernanceApiService } from '@app/elohim/services/governance-api.service';
import {
  GovernanceSignalService,
  ReactionCounts,
} from '@app/elohim/services/governance-signal.service';
import {
  EmotionalReactionType,
  EmotionalReactionConstraints,
  EmotionalReaction,
  EMOTIONAL_REACTION_DESCRIPTIONS,
  REACTION_CATEGORIES,
  MediatedReaction,
  DEFAULT_REACTION_CONSTRAINTS,
} from '@app/lamad/models/feedback-profile.model';
import { GovernanceRecognitionService } from '@app/qahal/services/governance-recognition.service';

import type { RecordSignalInputView, GovernanceSignalView } from '@elohim/storage-client/generated';

/**
 * ReactionBarComponent - Low-Friction Emotional Feedback
 *
 * NOT Facebook-style likes. These are contextual emotional responses that:
 * - Respect content's FeedbackProfile constraints
 * - Distinguish supportive vs critical reactions
 * - Mediate potentially harmful reactions (constitutional teaching)
 * - Track patterns for social health monitoring
 * - Record signals to the governance API backend (mechanism level 1)
 *
 * "The tyranny of the laughing emoji" - reactions can be weaponized.
 * This component protects against that through Elohim mediation.
 */
@Component({
  selector: 'app-reaction-bar',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="reaction-bar" [class.compact]="compact()" role="group" aria-label="Reactions">
      @for (reaction of availableReactions; track reaction.type) {
        <button
          class="reaction-btn"
          [class.active]="isUserReaction(reaction.type)"
          [class.mediated]="reaction.mediated"
          [attr.aria-label]="reaction.label"
          [attr.aria-pressed]="isUserReaction(reaction.type)"
          [attr.data-testid]="'reaction-' + reaction.type"
          [title]="reaction.description"
          (click)="onReactionClick(reaction)"
        >
          <span class="reaction-icon">{{ reaction.icon }}</span>
          @if (!compact()) {
            <span class="reaction-label">{{ reaction.label }}</span>
          }
          @if (showCounts() && getReactionCount(reaction.type) > 0) {
            <span class="reaction-count">{{ getReactionCount(reaction.type) }}</span>
          }
          @if (reaction.mediated) {
            <span class="mediation-indicator" aria-hidden="true">*</span>
          }
        </button>
      }
    </div>

    @if (showMediationDialog && mediationContext) {
      <div
        class="mediation-dialog-overlay"
        role="button"
        aria-label="Close mediation dialog"
        data-testid="mediation-overlay"
        tabindex="-1"
        (click)="closeMediationDialog()"
        (keydown.escape)="closeMediationDialog()"
      >
        <div
          class="mediation-dialog"
          role="dialog"
          aria-label="Mediation reflection"
          data-testid="mediation-dialog"
          (click)="$event.stopPropagation()"
        >
          <h3>A moment of reflection</h3>
          <p>{{ mediationContext.mediationConfig.constitutionalReasoning }}</p>
          <div class="mediation-actions">
            @if (mediationContext.mediationConfig.suggestedAlternatives?.length) {
              <p>Consider expressing this differently:</p>
              @for (alt of mediationContext.mediationConfig.suggestedAlternatives; track alt) {
                <button
                  class="mediation-alt-btn"
                  [attr.data-testid]="'mediation-alt-' + alt"
                  (click)="onMediationResponse(false, alt)"
                >
                  {{ getReactionDisplay(alt)?.icon }} {{ getReactionDisplay(alt)?.label }}
                </button>
              }
            }
            <button
              class="mediation-proceed-btn"
              data-testid="mediation-proceed-btn"
              (click)="onMediationResponse(true)"
            >
              Proceed anyway
            </button>
            <button
              class="mediation-cancel-btn"
              data-testid="mediation-cancel-btn"
              (click)="closeMediationDialog()"
            >
              Cancel
            </button>
          </div>
        </div>
      </div>
    }
  `,
  styles: [
    `
      .reaction-bar {
        display: flex;
        flex-wrap: wrap;
        gap: 0.5rem;
        padding: 0.5rem 0;
      }
      .reaction-bar.compact {
        gap: 0.25rem;
      }
      .reaction-btn {
        display: inline-flex;
        align-items: center;
        gap: 0.25rem;
        padding: 0.25rem 0.5rem;
        border: 1px solid var(--border-color, #ddd);
        border-radius: 1rem;
        background: var(--surface-color, #fff);
        cursor: pointer;
        font-size: 0.875rem;
        transition: all 0.2s ease;
      }
      .reaction-btn:hover {
        background: var(--hover-color, #f0f0f0);
      }
      .reaction-btn.active {
        background: var(--active-color, #e3f2fd);
        border-color: var(--active-border, #2196f3);
      }
      .reaction-btn.mediated {
        border-style: dashed;
      }
      .reaction-icon {
        font-size: 1.125rem;
      }
      .reaction-count {
        font-size: 0.75rem;
        color: var(--text-secondary, #666);
        min-width: 1rem;
        text-align: center;
      }
      .mediation-indicator {
        color: var(--warning-color, #f39c12);
        font-weight: bold;
      }
      .mediation-dialog-overlay {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.5);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 1000;
      }
      .mediation-dialog {
        background: var(--surface-color, #fff);
        border-radius: 0.5rem;
        padding: 1.5rem;
        max-width: 24rem;
        width: 90%;
      }
      .mediation-dialog h3 {
        margin: 0 0 0.75rem;
      }
      .mediation-actions {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        margin-top: 1rem;
      }
      .mediation-alt-btn {
        padding: 0.5rem;
        border: 1px solid var(--border-color, #ddd);
        border-radius: 0.25rem;
        background: var(--surface-color, #fff);
        cursor: pointer;
      }
      .mediation-proceed-btn {
        padding: 0.5rem;
        border: 1px solid var(--warning-color, #f39c12);
        border-radius: 0.25rem;
        background: transparent;
        color: var(--warning-color, #f39c12);
        cursor: pointer;
      }
      .mediation-cancel-btn {
        padding: 0.5rem;
        border: 1px solid var(--border-color, #ddd);
        border-radius: 0.25rem;
        background: transparent;
        cursor: pointer;
      }
    `,
  ],
})
export class ReactionBarComponent implements OnInit, OnDestroy {
  /** Entity type for governance signals (e.g. 'content', 'proposal') */
  readonly entityType = input<string>('content');

  /** Entity ID for governance signals */
  readonly entityId = input<string>('');

  /**
   * Legacy content ID input — used as fallback for entityId when entityId is not provided.
   * Prefer entityId + entityType for new usage.
   */
  readonly contentId = input<string>('');

  /** Allowed reaction types (defaults to constraints-based) */
  readonly allowedReactions = input<EmotionalReactionType[]>([]);

  /** Reaction constraints from content's FeedbackProfile */
  readonly constraints = input<EmotionalReactionConstraints | undefined>(undefined);

  /** Content type for constraint lookup */
  readonly contentType = input<string>('learning-content');

  /** Show reaction counts */
  readonly showCounts = input<boolean>(true);

  /** Compact mode (icons only) */
  readonly compact = input<boolean>(false);

  /** Resolved entity ID: prefers entityId input, falls back to contentId */
  private get resolvedEntityId(): string {
    return this.entityId() || this.contentId();
  }

  // Available reactions based on constraints
  availableReactions: ReactionDisplay[] = [];

  // Current reaction counts (from local + API aggregate)
  reactionCounts: ReactionCounts | null = null;

  // Aggregate counts from governance API signals
  apiReactionCounts: Record<string, number> = {};

  // User's current reaction (if any)
  userReaction: EmotionalReactionType | null = null;

  // Set of reactions the current user has submitted (for toggle behavior)
  userSubmittedReactions = new Set<EmotionalReactionType>();

  // Mediation state
  showMediationDialog = false;
  mediationContext: MediationDialogContext | null = null;

  private readonly destroy$ = new Subject<void>();

  // Icons for reaction types
  private readonly reactionIcons: Record<EmotionalReactionType, string> = {
    moved: '\uD83D\uDCAB',
    grateful: '\uD83D\uDE4F',
    inspired: '\u2728',
    hopeful: '\uD83C\uDF31',
    grieving: '\uD83D\uDD4A\uFE0F',
    challenged: '\uD83E\uDD14',
    concerned: '\u26A0\uFE0F',
    uncomfortable: '\uD83D\uDE1F',
  };

  private readonly signalService = inject(GovernanceSignalService);
  private readonly governanceApi = inject(GovernanceApiService);
  private readonly governanceRecognition = inject(GovernanceRecognitionService);

  private static readonly CURRENT_USER_ID = 'current-user';

  ngOnInit(): void {
    this.buildAvailableReactions();
    this.loadReactionCounts();
    this.loadApiSignals();
    this.subscribeToChanges();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  /**
   * Handle reaction click.
   * May trigger mediation dialog for sensitive reactions.
   */
  onReactionClick(reaction: ReactionDisplay): void {
    // Toggle behavior: if user already submitted this reaction, un-select it
    if (this.userSubmittedReactions.has(reaction.type)) {
      this.userSubmittedReactions.delete(reaction.type);
      this.userReaction =
        this.userSubmittedReactions.size > 0 ? [...this.userSubmittedReactions].pop()! : null;
      return;
    }

    // Check if this is a mediated reaction
    if (reaction.mediated && reaction.mediationConfig) {
      this.showMediationPrompt(reaction);
      return;
    }

    // Direct reaction (no mediation needed)
    this.submitReaction(reaction.type);
  }

  /**
   * Submit the reaction to both local signal service and governance API.
   */
  private submitReaction(type: EmotionalReactionType, context?: string): void {
    const reaction: EmotionalReaction = {
      type,
      context,
      private: false,
      respondedAt: new Date().toISOString(),
      responderId: '', // Will be filled by service
    };

    // Record locally via existing signal service
    this.signalService
      .recordReaction(this.resolvedEntityId, reaction)
      .pipe(takeUntil(this.destroy$))
      .subscribe(success => {
        if (success) {
          this.userReaction = type;
          this.userSubmittedReactions.add(type);
          this.loadReactionCounts(); // Refresh counts
        }
      });

    // Record to governance API backend
    const signal: RecordSignalInputView = {
      entityType: this.entityType(),
      entityId: this.resolvedEntityId,
      humanId: ReactionBarComponent.CURRENT_USER_ID,
      signalType: 'reaction',
      signalValue: type,
      mechanismLevel: 1,
      proxyElohimId: null,
    };

    this.governanceApi.recordSignal(signal).catch(() => {
      // API failure is non-blocking; local signal is already recorded
    });

    // Generate REA economic event for governance participation
    this.governanceRecognition
      .recordParticipation({
        entityType: this.entityType(),
        entityId: this.resolvedEntityId,
        humanId: ReactionBarComponent.CURRENT_USER_ID,
        mechanismLevel: 1,
        participationType: 'reaction',
      })
      .catch(() => {
        // Recognition failure is non-blocking
      });
  }

  /**
   * Show mediation dialog for potentially harmful reactions.
   */
  private showMediationPrompt(reaction: ReactionDisplay): void {
    if (!reaction.mediationConfig) return;

    this.mediationContext = {
      reaction,
      mediationConfig: reaction.mediationConfig,
    };
    this.showMediationDialog = true;
  }

  /**
   * Handle mediation dialog response.
   */
  onMediationResponse(proceed: boolean, alternativeChosen?: EmotionalReactionType): void {
    if (!this.mediationContext) {
      this.closeMediationDialog();
      return;
    }

    const { reaction, mediationConfig } = this.mediationContext;

    if (proceed) {
      // User chose to proceed despite mediation
      this.signalService
        .recordMediationProceed({
          userId: '',
          contentId: this.resolvedEntityId,
          contentType: this.contentType(),
          reactionType: reaction.type,
          reasoningShown: mediationConfig.constitutionalReasoning,
          proceededAnyway: true,
          loggedAt: new Date().toISOString(),
        })
        .subscribe();

      if (mediationConfig.proceedBehavior.visibleToOthers) {
        this.submitReaction(reaction.type);
      } else {
        this.userReaction = reaction.type;
        this.userSubmittedReactions.add(reaction.type);
      }
    } else if (alternativeChosen) {
      this.signalService
        .recordMediationProceed({
          userId: '',
          contentId: this.resolvedEntityId,
          contentType: this.contentType(),
          reactionType: reaction.type,
          reasoningShown: mediationConfig.constitutionalReasoning,
          proceededAnyway: false,
          alternativeChosen,
          loggedAt: new Date().toISOString(),
        })
        .subscribe();

      this.submitReaction(alternativeChosen);
    }

    this.closeMediationDialog();
  }

  /**
   * Close mediation dialog without action.
   */
  closeMediationDialog(): void {
    this.showMediationDialog = false;
    this.mediationContext = null;
  }

  /**
   * Get display data for a reaction type.
   */
  getReactionDisplay(type: EmotionalReactionType): ReactionDisplay | undefined {
    return this.availableReactions.find(r => r.type === type);
  }

  /**
   * Get count for a specific reaction type (merging local + API counts).
   */
  getReactionCount(type: EmotionalReactionType): number {
    const localCount = this.reactionCounts?.byType[type] ?? 0;
    const apiCount = this.apiReactionCounts[type] ?? 0;
    // Use whichever is higher to avoid double-counting while showing best data
    return Math.max(localCount, apiCount);
  }

  /**
   * Check if user has reacted with this type.
   */
  isUserReaction(type: EmotionalReactionType): boolean {
    return this.userSubmittedReactions.has(type);
  }

  /**
   * Build the list of available reactions based on constraints.
   */
  private buildAvailableReactions(): void {
    const effectiveConstraints =
      this.constraints() ??
      DEFAULT_REACTION_CONSTRAINTS[this.contentType()] ??
      DEFAULT_REACTION_CONSTRAINTS['learning-content'];

    const permittedTypes =
      this.allowedReactions().length > 0
        ? this.allowedReactions()
        : effectiveConstraints.permittedTypes;

    const mediatedTypes = effectiveConstraints.mediatedTypes ?? [];
    const mediatedMap = new Map(mediatedTypes.map(m => [m.type, m]));

    this.availableReactions = permittedTypes.map(type => {
      const mediation = mediatedMap.get(type);
      return {
        type,
        icon: this.reactionIcons[type],
        label: this.getReactionLabel(type),
        description: EMOTIONAL_REACTION_DESCRIPTIONS[type],
        category: REACTION_CATEGORIES[type],
        mediated: !!mediation,
        mediationConfig: mediation,
      };
    });

    // Also add mediated types (shown with warning indicator)
    for (const mediated of mediatedTypes) {
      if (!permittedTypes.includes(mediated.type)) {
        this.availableReactions.push({
          type: mediated.type,
          icon: this.reactionIcons[mediated.type],
          label: this.getReactionLabel(mediated.type),
          description: EMOTIONAL_REACTION_DESCRIPTIONS[mediated.type],
          category: REACTION_CATEGORIES[mediated.type],
          mediated: true,
          mediationConfig: mediated,
        });
      }
    }
  }

  private getReactionLabel(type: EmotionalReactionType): string {
    const labels: Record<EmotionalReactionType, string> = {
      moved: 'Moved',
      grateful: 'Grateful',
      inspired: 'Inspired',
      hopeful: 'Hopeful',
      grieving: 'Grieving',
      challenged: 'Challenged',
      concerned: 'Concerned',
      uncomfortable: 'Uncomfortable',
    };
    return labels[type];
  }

  private loadReactionCounts(): void {
    this.signalService
      .getReactionCounts(this.resolvedEntityId)
      .pipe(takeUntil(this.destroy$))
      .subscribe(counts => {
        this.reactionCounts = counts;
      });
  }

  /**
   * Load existing signals from the governance API to show aggregate counts
   * and determine which reactions the current user has already submitted.
   */
  private loadApiSignals(): void {
    this.governanceApi
      .getSignals(this.entityType(), this.resolvedEntityId)
      .then((signals: GovernanceSignalView[]) => {
        const reactionSignals = signals.filter(s => s.signalType === 'reaction');

        // Build aggregate counts from API signals
        const counts: Record<string, number> = {};
        for (const sig of reactionSignals) {
          counts[sig.signalValue] = (counts[sig.signalValue] ?? 0) + 1;
        }
        this.apiReactionCounts = counts;

        // Track which reactions the current user has already submitted
        const userSignals = reactionSignals.filter(
          s => s.humanId === ReactionBarComponent.CURRENT_USER_ID
        );
        for (const sig of userSignals) {
          const reactionType = sig.signalValue as EmotionalReactionType;
          if (this.reactionIcons[reactionType]) {
            this.userSubmittedReactions.add(reactionType);
            this.userReaction = reactionType;
          }
        }
      })
      .catch(() => {
        // API unavailable — rely on local signals only
      });
  }

  private subscribeToChanges(): void {
    this.signalService.signalChanges$.pipe(takeUntil(this.destroy$)).subscribe(change => {
      if (change?.type === 'reaction' && change.contentId === this.resolvedEntityId) {
        this.loadReactionCounts();
      }
    });
  }
}

// ===========================================================================
// Component Types
// ===========================================================================

interface ReactionDisplay {
  type: EmotionalReactionType;
  icon: string;
  label: string;
  description: string;
  category: 'supportive' | 'critical';
  mediated: boolean;
  mediationConfig?: MediatedReaction;
}

interface MediationDialogContext {
  reaction: ReactionDisplay;
  mediationConfig: MediatedReaction;
}
