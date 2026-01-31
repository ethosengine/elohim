import { CommonModule } from '@angular/common';
import { Component, Input, OnInit, OnDestroy } from '@angular/core';

// @coverage: 100.0% (2026-01-31)

import { Subject, takeUntil } from 'rxjs';

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

/**
 * ReactionBarComponent - Low-Friction Emotional Feedback
 *
 * NOT Facebook-style likes. These are contextual emotional responses that:
 * - Respect content's FeedbackProfile constraints
 * - Distinguish supportive vs critical reactions
 * - Mediate potentially harmful reactions (constitutional teaching)
 * - Track patterns for social health monitoring
 *
 * "The tyranny of the laughing emoji" - reactions can be weaponized.
 * This component protects against that through Elohim mediation.
 */
@Component({
  selector: 'app-reaction-bar',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './reaction-bar.component.html',
  styleUrls: ['./reaction-bar.component.css'],
})
export class ReactionBarComponent implements OnInit, OnDestroy {
  @Input() contentId!: string;
  @Input() allowedReactions: EmotionalReactionType[] = [];
  @Input() constraints?: EmotionalReactionConstraints;
  @Input() contentType = 'learning-content';
  @Input() showCounts = true;
  @Input() compact = false;

  // Available reactions based on constraints
  availableReactions: ReactionDisplay[] = [];

  // Current reaction counts
  reactionCounts: ReactionCounts | null = null;

  // User's current reaction (if any)
  userReaction: EmotionalReactionType | null = null;

  // Mediation state
  showMediationDialog = false;
  mediationContext: MediationDialogContext | null = null;

  private readonly destroy$ = new Subject<void>();

  // Icons for reaction types
  private readonly reactionIcons: Record<EmotionalReactionType, string> = {
    moved: '💫',
    grateful: '🙏',
    inspired: '✨',
    hopeful: '🌱',
    grieving: '🕊️',
    challenged: '🤔',
    concerned: '⚠️',
    uncomfortable: '😟',
  };

  constructor(private readonly signalService: GovernanceSignalService) {}

  ngOnInit(): void {
    this.buildAvailableReactions();
    this.loadReactionCounts();
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
    // Check if this is a mediated reaction
    if (reaction.mediated && reaction.mediationConfig) {
      this.showMediationPrompt(reaction);
      return;
    }

    // Direct reaction (no mediation needed)
    this.submitReaction(reaction.type);
  }

  /**
   * Submit the reaction to the governance signal service.
   */
  private submitReaction(type: EmotionalReactionType, context?: string): void {
    const reaction: EmotionalReaction = {
      type,
      context,
      private: false,
      respondedAt: new Date().toISOString(),
      responderId: '', // Will be filled by service
    };

    this.signalService
      .recordReaction(this.contentId, reaction)
      .pipe(takeUntil(this.destroy$))
      .subscribe(success => {
        if (success) {
          this.userReaction = type;
          this.loadReactionCounts(); // Refresh counts
        }
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
      // Log this for pattern monitoring
      this.signalService
        .recordMediationProceed({
          userId: '', // Filled by service
          contentId: this.contentId,
          contentType: this.contentType,
          reactionType: reaction.type,
          reasoningShown: mediationConfig.constitutionalReasoning,
          proceededAnyway: true,
          loggedAt: new Date().toISOString(),
        })
        .subscribe();

      // Submit reaction with mediated behavior
      if (mediationConfig.proceedBehavior.visibleToOthers) {
        this.submitReaction(reaction.type);
      } else {
        // Record but don't display
        this.userReaction = reaction.type;
      }
    } else if (alternativeChosen) {
      // User chose an alternative reaction
      this.signalService
        .recordMediationProceed({
          userId: '',
          contentId: this.contentId,
          contentType: this.contentType,
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
   * Get count for a specific reaction type.
   */
  getReactionCount(type: EmotionalReactionType): number {
    return this.reactionCounts?.byType[type] ?? 0;
  }

  /**
   * Check if user has reacted with this type.
   */
  isUserReaction(type: EmotionalReactionType): boolean {
    return this.userReaction === type;
  }

  /**
   * Build the list of available reactions based on constraints.
   */
  private buildAvailableReactions(): void {
    const effectiveConstraints =
      this.constraints ??
      DEFAULT_REACTION_CONSTRAINTS[this.contentType] ??
      DEFAULT_REACTION_CONSTRAINTS['learning-content'];

    const permittedTypes =
      this.allowedReactions.length > 0
        ? this.allowedReactions
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
      .getReactionCounts(this.contentId)
      .pipe(takeUntil(this.destroy$))
      .subscribe(counts => {
        this.reactionCounts = counts;
      });
  }

  private subscribeToChanges(): void {
    this.signalService.signalChanges$.pipe(takeUntil(this.destroy$)).subscribe(change => {
      if (change?.type === 'reaction' && change.contentId === this.contentId) {
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
