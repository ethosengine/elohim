import { Injectable, Optional, inject } from '@angular/core';

// @coverage: 42.6% (2026-02-05)

import { map, shareReplay, take, catchError } from 'rxjs/operators';

import { BehaviorSubject, Observable, of, combineLatest } from 'rxjs';

import { SessionHumanService } from '@app/imagodei/services/session-human.service';
import {
  EmotionalReaction,
  EmotionalReactionType,
  MediationLog,
  REACTION_CATEGORIES,
} from '@app/lamad/models/feedback-profile.model';

import { LoggerService } from './logger.service';

// =============================================================================
// LRU Cache Implementation
// =============================================================================

/**
 * Simple LRU (Least Recently Used) cache with configurable max size.
 * When capacity is reached, evicts the least recently accessed entries.
 */
class LruCache<K, V> {
  private readonly cache = new Map<K, V>();
  private accessOrder: K[] = [];

  constructor(private readonly maxSize: number) {}

  has(key: K): boolean {
    return this.cache.has(key);
  }

  get(key: K): V | undefined {
    const value = this.cache.get(key);
    if (value !== undefined) {
      this.touch(key);
    }
    return value;
  }

  set(key: K, value: V): void {
    if (this.cache.has(key)) {
      this.cache.set(key, value);
      this.touch(key);
      return;
    }

    // Evict LRU entries if at capacity
    while (this.cache.size >= this.maxSize && this.accessOrder.length > 0) {
      const lruKey = this.accessOrder.shift()!;
      this.cache.delete(lruKey);
    }

    this.cache.set(key, value);
    this.accessOrder.push(key);
  }

  delete(key: K): boolean {
    const deleted = this.cache.delete(key);
    if (deleted) {
      const idx = this.accessOrder.indexOf(key);
      if (idx !== -1) {
        this.accessOrder.splice(idx, 1);
      }
    }
    return deleted;
  }

  clear(): void {
    this.cache.clear();
    this.accessOrder = [];
  }

  get size(): number {
    return this.cache.size;
  }

  private touch(key: K): void {
    const idx = this.accessOrder.indexOf(key);
    if (idx !== -1) {
      this.accessOrder.splice(idx, 1);
    }
    this.accessOrder.push(key);
  }
}

// =============================================================================
// Constants
// =============================================================================

/** Maximum number of cached content signals */
const MAX_CONTENT_CACHE_SIZE = 100;

/** Maximum number of cached path signals */
const MAX_PATH_CACHE_SIZE = 50;

/**
 * GovernanceSignalService - Central Signal Hub for Governance Feedback
 *
 * "Every system needs positive and particularly negative feedback,
 * or it goes chaotic." - Destin Sandlin, Smarter Every Day
 *
 * This service is the nervous system of the governance layer:
 * 1. Collects signals from diverse sources (reactions, feedback, learning, completions)
 * 2. Aggregates signals for content-level governance visualization
 * 3. Computes opinion clusters (Polis-style) for consensus detection
 * 4. Triggers governance actions (attestations, reviews) based on patterns
 *
 * Signal Sources:
 * - ReactionBarComponent: Emotional reactions (low friction)
 * - GraduatedFeedbackComponent: Scaled feedback (medium friction)
 * - PathNavigator: Learning completion signals
 * - ContentViewer: Interactive completion signals
 * - AssessmentService: Assessment result signals
 *
 * MVP Implementation: localStorage with Observable patterns
 * Holochain Migration: DHT entries with links for aggregation
 */
const SIGNAL_GRADUATED_FEEDBACK = 'graduated-feedback';

@Injectable({ providedIn: 'root' })
export class GovernanceSignalService {
  private readonly logger = inject(LoggerService).createChild('GovernanceSignal');
  private readonly STORAGE_PREFIX = 'elohim-governance-signal-';

  // Cached signal aggregates with LRU eviction
  private readonly contentSignalsCache = new LruCache<string, Observable<AggregatedSignals>>(
    MAX_CONTENT_CACHE_SIZE
  );
  private readonly pathSignalsCache = new LruCache<string, Observable<PathSignalAggregate>>(
    MAX_PATH_CACHE_SIZE
  );

  // Signal change stream for UI reactivity
  private readonly signalChangeSubject = new BehaviorSubject<SignalChangeEvent | null>(null);
  public readonly signalChanges$ = this.signalChangeSubject.asObservable();

  constructor(@Optional() private readonly sessionHumanService: SessionHumanService | null) {}

  // ===========================================================================
  // Signal Collection - Low Friction (Reactions)
  // ===========================================================================

  /**
   * Record an emotional reaction to content.
   * Low-friction signal that respects content's FeedbackProfile constraints.
   */
  recordReaction(contentId: string, reaction: EmotionalReaction): Observable<boolean> {
    const agentId = this.getAgentId();
    const randomBytes = crypto.getRandomValues(new Uint8Array(6));
    const randomStr = Array.from(randomBytes)
      .map(b => b.toString(36))
      .join('')
      .substring(0, 7);
    const reactionRecord: ReactionRecord = {
      id: `reaction-${Date.now()}-${randomStr}`,
      contentId,
      reactorId: agentId,
      reactionType: reaction.type,
      category: REACTION_CATEGORIES[reaction.type],
      context: reaction.context,
      private: reaction.private,
      createdAt: new Date().toISOString(),
    };

    const saved = this.saveSignal('reactions', contentId, reactionRecord);

    if (saved) {
      this.invalidateCache(contentId);
      this.emitSignalChange({
        type: 'reaction',
        contentId,
        signalId: reactionRecord.id,
        agentId,
        timestamp: reactionRecord.createdAt,
      });
    }

    return of(saved);
  }

  /**
   * Record when a user proceeds through mediation (strong behavioral signal).
   * Indicates ignoring constitutional reasoning about harmful reactions.
   */
  recordMediationProceed(log: MediationLog): Observable<boolean> {
    // Add an id to the log for storage
    const randomBytes = crypto.getRandomValues(new Uint8Array(6));
    const randomStr = Array.from(randomBytes)
      .map(b => b.toString(36))
      .join('')
      .substring(0, 7);
    const logWithId = {
      ...log,
      id: `mediation-${Date.now()}-${randomStr}`,
    };
    const saved = this.saveSignal('mediation-logs', log.contentId, logWithId);

    if (saved) {
      this.emitSignalChange({
        type: 'mediation-proceed',
        contentId: log.contentId,
        signalId: logWithId.id,
        agentId: log.userId,
        timestamp: log.loggedAt,
      });
    }

    return of(saved);
  }

  /**
   * Get reactions for content (public reactions only by default).
   */
  getReactions(contentId: string, includePrivate = false): Observable<ReactionRecord[]> {
    return of(this.loadSignals<ReactionRecord>('reactions', contentId)).pipe(
      map(reactions => {
        if (includePrivate) return reactions;
        return reactions.filter(r => !r.private);
      })
    );
  }

  /**
   * Get aggregated reaction counts for content.
   */
  getReactionCounts(contentId: string): Observable<ReactionCounts> {
    return this.getReactions(contentId).pipe(
      map(reactions => {
        const counts: ReactionCounts = {
          total: reactions.length,
          byType: {} as Record<EmotionalReactionType, number>,
          byCategory: { supportive: 0, critical: 0 },
        };

        for (const r of reactions) {
          counts.byType[r.reactionType] = (counts.byType[r.reactionType] ?? 0) + 1;
          counts.byCategory[r.category]++;
        }

        return counts;
      })
    );
  }

  // ===========================================================================
  // Signal Collection - Medium Friction (Graduated Feedback)
  // ===========================================================================

  /**
   * Record graduated feedback (Loomio-style scaled responses).
   * Medium-friction signal with optional reasoning.
   */
  recordGraduatedFeedback(
    contentId: string,
    feedback: GraduatedFeedbackInput
  ): Observable<boolean> {
    const agentId = this.getAgentId();
    const randomBytes = crypto.getRandomValues(new Uint8Array(6));
    const randomStr = Array.from(randomBytes)
      .map(b => b.toString(36))
      .join('')
      .substring(0, 7);
    const feedbackRecord: GraduatedFeedbackRecord = {
      id: `feedback-${Date.now()}-${randomStr}`,
      contentId,
      respondentId: agentId,
      context: feedback.context,
      position: feedback.position,
      positionIndex: feedback.positionIndex,
      intensity: feedback.intensity,
      reasoning: feedback.reasoning,
      createdAt: new Date().toISOString(),
    };

    const saved = this.saveSignal(SIGNAL_GRADUATED_FEEDBACK, contentId, feedbackRecord);

    if (saved) {
      this.invalidateCache(contentId);
      this.emitSignalChange({
        type: SIGNAL_GRADUATED_FEEDBACK,
        contentId,
        signalId: feedbackRecord.id,
        agentId,
        timestamp: feedbackRecord.createdAt,
      });
    }

    return of(saved);
  }

  /**
   * Get graduated feedback for content.
   */
  getGraduatedFeedback(contentId: string): Observable<GraduatedFeedbackRecord[]> {
    return of(this.loadSignals<GraduatedFeedbackRecord>('graduated-feedback', contentId));
  }

  /**
   * Get aggregated feedback statistics.
   */
  getFeedbackStats(contentId: string): Observable<FeedbackStats> {
    return this.getGraduatedFeedback(contentId).pipe(
      map(feedback => {
        if (feedback.length === 0) {
          return {
            totalResponses: 0,
            averagePosition: 0,
            averageIntensity: 0,
            distribution: {},
            contexts: [],
          };
        }

        const byContext = new Map<string, GraduatedFeedbackRecord[]>();
        let totalPosition = 0;
        let totalIntensity = 0;
        const distribution: Record<string, number> = {};

        for (const f of feedback) {
          // Group by context
          if (!byContext.has(f.context)) {
            byContext.set(f.context, []);
          }
          byContext.get(f.context)!.push(f);

          // Aggregate
          totalPosition += f.positionIndex;
          totalIntensity += f.intensity;

          // Distribution
          distribution[f.position] = (distribution[f.position] || 0) + 1;
        }

        return {
          totalResponses: feedback.length,
          averagePosition: totalPosition / feedback.length,
          averageIntensity: totalIntensity / feedback.length,
          distribution,
          contexts: Array.from(byContext.keys()),
        };
      })
    );
  }

  // ===========================================================================
  // Signal Collection - Learning Signals
  // ===========================================================================

  /**
   * Record a learning signal when user completes path steps.
   * These signals aggregate to content quality assessment.
   */
  recordLearningSignal(signal: LearningSignalInput): Observable<boolean> {
    const agentId = this.getAgentId();
    const randomBytes = crypto.getRandomValues(new Uint8Array(6));
    const randomStr = Array.from(randomBytes)
      .map(b => b.toString(36))
      .join('')
      .substring(0, 7);
    const signalRecord: LearningSignalRecord = {
      id: `learning-${Date.now()}-${randomStr}`,
      contentId: signal.contentId,
      signalType: signal.signalType,
      payload: signal.payload,
      learnerId: agentId,
      createdAt: new Date().toISOString(),
    };

    // Extract pathId from payload if present for path indexing
    const pathId = signal.payload?.['pathId'] as string | undefined;

    // Save to content index
    const savedToContent = this.saveSignal('learning-signals', signal.contentId, signalRecord);

    // Also save to path index if pathId is provided
    const savedToPath = pathId
      ? this.saveSignal('path-learning-signals', pathId, signalRecord)
      : true;

    if (savedToContent) {
      this.invalidateCache(signal.contentId);
      this.emitSignalChange({
        type: 'learning-signal',
        contentId: signal.contentId,
        signalId: signalRecord.id,
        agentId,
        timestamp: signalRecord.createdAt,
      });
    }

    return of(savedToContent && savedToPath);
  }

  /**
   * Get learning signals for content.
   */
  getLearningSignals(contentId: string): Observable<LearningSignalRecord[]> {
    return of(this.loadSignals<LearningSignalRecord>('learning-signals', contentId));
  }

  /**
   * Get learning signals for a path.
   */
  getPathLearningSignals(pathId: string): Observable<LearningSignalRecord[]> {
    return of(this.loadSignals<LearningSignalRecord>('path-learning-signals', pathId));
  }

  // ===========================================================================
  // Signal Collection - Interactive Completion
  // ===========================================================================

  /**
   * Record interactive content completion (renderer success/failure).
   * These signals indicate content effectiveness.
   */
  recordInteractiveCompletion(signal: CompletionSignalInput): Observable<boolean> {
    const agentId = this.getAgentId();
    const randomBytes = crypto.getRandomValues(new Uint8Array(6));
    const randomStr = Array.from(randomBytes)
      .map(b => b.toString(36))
      .join('')
      .substring(0, 7);
    const completionRecord: CompletionSignalRecord = {
      id: `completion-${Date.now()}-${randomStr}`,
      contentId: signal.contentId,
      interactionType: signal.interactionType,
      passed: signal.passed,
      score: signal.score,
      details: signal.details,
      completedBy: agentId,
      completedAt: new Date().toISOString(),
    };

    const saved = this.saveSignal('completions', signal.contentId, completionRecord);

    if (saved) {
      this.invalidateCache(signal.contentId);
      this.emitSignalChange({
        type: 'completion',
        contentId: signal.contentId,
        signalId: completionRecord.id,
        agentId,
        timestamp: completionRecord.completedAt,
      });
    }

    return of(saved);
  }

  /**
   * Get completion signals for content.
   */
  getCompletions(contentId: string): Observable<CompletionSignalRecord[]> {
    return of(this.loadSignals<CompletionSignalRecord>('completions', contentId));
  }

  // ===========================================================================
  // Aggregation Queries
  // ===========================================================================

  /**
   * Get all aggregated signals for content.
   * This is the primary query for governance visualization.
   */
  getContentSignals(contentId: string): Observable<AggregatedSignals> {
    if (!this.contentSignalsCache.has(contentId)) {
      const signals$ = combineLatest([
        this.getReactionCounts(contentId),
        this.getFeedbackStats(contentId),
        this.getLearningSignals(contentId),
        this.getCompletions(contentId),
      ]).pipe(
        map(([reactionCounts, feedbackStats, learningSignals, completions]) => {
          // Compute overall sentiment from reactions
          const sentimentScore = this.computeSentimentScore(reactionCounts);

          // Compute pedagogical effectiveness from learning signals
          const effectivenessScore = this.computeEffectivenessScore(learningSignals, completions);

          // Compute consensus state from feedback
          const consensusState = this.computeConsensusState(feedbackStats);

          return {
            contentId,
            reactionCounts,
            feedbackStats,
            learningSignalCount: learningSignals.length,
            completionCount: completions.length,
            sentimentScore,
            effectivenessScore,
            consensusState,
            lastUpdated: new Date().toISOString(),
          };
        }),
        shareReplay(1)
      );

      this.contentSignalsCache.set(contentId, signals$);
    }

    return this.contentSignalsCache.get(contentId)!;
  }

  /**
   * Get aggregated signals for a learning path.
   */
  getPathSignals(pathId: string): Observable<PathSignalAggregate> {
    if (!this.pathSignalsCache.has(pathId)) {
      const signals$ = this.getPathLearningSignals(pathId).pipe(
        map(signals => {
          if (signals.length === 0) {
            return {
              pathId,
              totalLearners: 0,
              completionsByStep: {},
              averageMasteryByStep: {},
              averageTimeByStep: {},
              scaffoldingScoreByStep: {},
              overallEffectiveness: 0,
              lastUpdated: new Date().toISOString(),
            };
          }

          // Helper to extract payload values
          const getPayloadNumber = (
            s: LearningSignalRecord,
            key: string,
            defaultVal: number
          ): number => {
            const val = s.payload?.[key];
            return typeof val === 'number' ? val : defaultVal;
          };
          const getPayloadString = (
            s: LearningSignalRecord,
            key: string,
            defaultVal: string
          ): string => {
            const val = s.payload?.[key];
            return typeof val === 'string' ? val : defaultVal;
          };

          // Group by step
          const byStep = new Map<number, LearningSignalRecord[]>();
          const uniqueLearners = new Set<string>();

          for (const s of signals) {
            uniqueLearners.add(s.learnerId);
            const stepIndex = getPayloadNumber(s, 'stepIndex', 0);
            if (!byStep.has(stepIndex)) {
              byStep.set(stepIndex, []);
            }
            byStep.get(stepIndex)!.push(s);
          }

          // Compute per-step metrics
          const completionsByStep: Record<number, number> = {};
          const averageMasteryByStep: Record<number, number> = {};
          const averageTimeByStep: Record<number, number> = {};
          const scaffoldingScoreByStep: Record<number, number> = {};

          for (const [step, stepSignals] of byStep.entries()) {
            completionsByStep[step] = stepSignals.length;
            averageMasteryByStep[step] =
              stepSignals.reduce(
                (sum, s) => sum + this.masteryToNumber(getPayloadString(s, 'masteryLevel', 'none')),
                0
              ) / stepSignals.length;
            averageTimeByStep[step] =
              stepSignals.reduce((sum, s) => sum + getPayloadNumber(s, 'timeSpentSeconds', 0), 0) /
              stepSignals.length;
            scaffoldingScoreByStep[step] =
              stepSignals.reduce(
                (sum, s) => sum + getPayloadNumber(s, 'scaffoldingEffective', 0.5),
                0
              ) / stepSignals.length;
          }

          // Overall effectiveness
          const allMasteries = signals.map(s =>
            this.masteryToNumber(getPayloadString(s, 'masteryLevel', 'none'))
          );
          const overallEffectiveness =
            allMasteries.reduce((sum, m) => sum + m, 0) / allMasteries.length;

          return {
            pathId,
            totalLearners: uniqueLearners.size,
            completionsByStep,
            averageMasteryByStep,
            averageTimeByStep,
            scaffoldingScoreByStep,
            overallEffectiveness,
            lastUpdated: new Date().toISOString(),
          };
        }),
        shareReplay(1)
      );

      this.pathSignalsCache.set(pathId, signals$);
    }

    return this.pathSignalsCache.get(pathId)!;
  }

  /**
   * Get community consensus state for content.
   */
  getCommunityConsensus(contentId: string): Observable<ConsensusState> {
    return this.getContentSignals(contentId).pipe(map(signals => signals.consensusState));
  }

  // ===========================================================================
  // Governance Triggers
  // ===========================================================================

  /**
   * Suggest an attestation based on evidence.
   * Called when signals indicate content deserves recognition.
   */
  suggestAttestation(
    contentId: string,
    attestationType: string,
    evidence: AttestationEvidence
  ): Observable<AttestationSuggestion> {
    const suggestion: AttestationSuggestion = {
      id: `attestation-suggestion-${Date.now()}`,
      contentId,
      attestationType,
      evidence,
      suggestedAt: new Date().toISOString(),
      suggestedBy: 'governance-signal-service',
      status: 'pending',
    };

    const suggestions =
      this.loadFromStorage<AttestationSuggestion[]>(
        `${this.STORAGE_PREFIX}attestation-suggestions`
      ) ?? [];
    suggestions.push(suggestion);
    this.saveToStorage(`${this.STORAGE_PREFIX}attestation-suggestions`, suggestions);

    return of(suggestion);
  }

  /**
   * Flag content for review based on concerns.
   * Called when signals indicate potential issues.
   */
  flagForReview(
    contentId: string,
    concern: string,
    evidence: ReviewFlagEvidence
  ): Observable<ReviewFlag> {
    const flag: ReviewFlag = {
      id: `review-flag-${Date.now()}`,
      contentId,
      concern,
      evidence,
      flaggedAt: new Date().toISOString(),
      flaggedBy: this.getAgentId(),
      status: 'pending',
    };

    const flags = this.loadFromStorage<ReviewFlag[]>(`${this.STORAGE_PREFIX}review-flags`) ?? [];
    flags.push(flag);
    this.saveToStorage(`${this.STORAGE_PREFIX}review-flags`, flags);

    this.emitSignalChange({
      type: 'review-flag',
      contentId,
      signalId: flag.id,
      agentId: flag.flaggedBy,
      timestamp: flag.flaggedAt,
    });

    return of(flag);
  }

  /**
   * Get pending attestation suggestions.
   */
  getPendingAttestations(): Observable<AttestationSuggestion[]> {
    const suggestions =
      this.loadFromStorage<AttestationSuggestion[]>(
        `${this.STORAGE_PREFIX}attestation-suggestions`
      ) ?? [];
    return of(suggestions.filter(s => s.status === 'pending'));
  }

  /**
   * Get pending review flags.
   */
  getPendingReviewFlags(): Observable<ReviewFlag[]> {
    const flags = this.loadFromStorage<ReviewFlag[]>(`${this.STORAGE_PREFIX}review-flags`) ?? [];
    return of(flags.filter(f => f.status === 'pending'));
  }

  // ===========================================================================
  // Polis-Style Opinion Clustering
  // ===========================================================================

  /**
   * Compute opinion clusters for content (Polis pattern).
   * Groups respondents by similarity in their feedback patterns.
   */
  computeOpinionClusters(contentId: string): Observable<OpinionCluster[]> {
    return combineLatest([this.getGraduatedFeedback(contentId), this.getReactions(contentId)]).pipe(
      map(([feedback, reactions]) => {
        if (feedback.length === 0 && reactions.length === 0) {
          return [];
        }

        // Build respondent vectors
        const respondentVectors = this.buildRespondentVectors(feedback, reactions);

        // Simple clustering (MVP: position-based grouping)
        // In production, use proper PCA + k-means like Polis
        return this.clusterRespondents(respondentVectors);
      })
    );
  }

  /**
   * Get consensus statements (what bridges opinion groups).
   */
  getConsensusStatements(contentId: string): Observable<ConsensusStatement[]> {
    return this.getGraduatedFeedback(contentId).pipe(
      map(feedback => {
        // Group by context (statement type)
        const byContext = new Map<string, GraduatedFeedbackRecord[]>();
        for (const f of feedback) {
          if (!byContext.has(f.context)) {
            byContext.set(f.context, []);
          }
          byContext.get(f.context)!.push(f);
        }

        const statements: ConsensusStatement[] = [];

        for (const [context, contextFeedback] of byContext.entries()) {
          if (contextFeedback.length < 2) continue;

          const positions = contextFeedback.map(f => f.positionIndex);
          const mean = positions.reduce((a, b) => a + b, 0) / positions.length;
          const variance =
            positions.reduce((sum, p) => sum + Math.pow(p - mean, 2), 0) / positions.length;
          const stdDev = Math.sqrt(variance);

          // Low variance = consensus, high variance = divisive
          const isConsensus = stdDev < 0.5;
          const isDivisive = stdDev > 1.5;

          if (isConsensus || isDivisive) {
            statements.push({
              context,
              statementType: isConsensus ? 'consensus' : 'divisive',
              averagePosition: mean,
              standardDeviation: stdDev,
              responseCount: contextFeedback.length,
              agreePercentage: positions.filter(p => p >= 0.6).length / positions.length,
            });
          }
        }

        return statements;
      })
    );
  }

  // ===========================================================================
  // Assessment Integration Triggers
  // ===========================================================================

  /**
   * Check if content should receive attestation based on assessment results.
   * Call this after assessment completion.
   */
  checkAttestationTrigger(
    contentId: string,
    assessmentScore: number,
    attempts: number
  ): Observable<boolean> {
    // High success rate suggests pedagogical validity
    if (assessmentScore >= 0.8) {
      this.suggestAttestation(contentId, 'pedagogically-verified', {
        evidenceType: 'assessment-success',
        score: assessmentScore,
        attempts,
      })
        .pipe(
          take(1),
          catchError(err => {
            this.logger.error(
              'Failed to suggest attestation',
              err instanceof Error ? err : new Error(String(err))
            );
            return of(null);
          })
        )
        .subscribe();
      return of(true);
    }

    // Low success with multiple attempts suggests need for clarification
    if (assessmentScore <= 0.4 && attempts >= 3) {
      this.flagForReview(contentId, 'needs-clarification', {
        evidenceType: 'assessment-failure',
        attempts,
        avgScore: assessmentScore,
      })
        .pipe(
          take(1),
          catchError(err => {
            this.logger.error(
              'Failed to flag for review',
              err instanceof Error ? err : new Error(String(err))
            );
            return of(null);
          })
        )
        .subscribe();
      return of(true);
    }

    return of(false);
  }

  // ===========================================================================
  // Private Helpers
  // ===========================================================================

  private getAgentId(): string {
    return this.sessionHumanService?.getSessionId() ?? 'anonymous';
  }

  private saveSignal<T extends { id: string }>(
    signalType: string,
    entityId: string,
    signal: T
  ): boolean {
    const key = `${this.STORAGE_PREFIX}${signalType}-${entityId}`;
    try {
      const existing = this.loadFromStorage<T[]>(key) ?? [];
      existing.push(signal);
      this.saveToStorage(key, existing);
      return true;
    } catch (err) {
      this.logger.error(
        `Failed to save ${signalType}`,
        err instanceof Error ? err : new Error(String(err))
      );
      return false;
    }
  }

  private loadSignals<T>(signalType: string, entityId: string): T[] {
    const key = `${this.STORAGE_PREFIX}${signalType}-${entityId}`;
    return this.loadFromStorage<T[]>(key) ?? [];
  }

  private loadFromStorage<T>(key: string): T | null {
    try {
      const data = localStorage.getItem(key);
      return data ? JSON.parse(data) : null;
    } catch {
      return null;
    }
  }

  private saveToStorage<T>(key: string, data: T): void {
    try {
      localStorage.setItem(key, JSON.stringify(data));
    } catch (err) {
      this.logger.error('Storage error', err instanceof Error ? err : new Error(String(err)));
    }
  }

  private invalidateCache(contentId: string): void {
    this.contentSignalsCache.delete(contentId);
  }

  private emitSignalChange(event: SignalChangeEvent): void {
    this.signalChangeSubject.next(event);
  }

  private computeSentimentScore(counts: ReactionCounts): number {
    if (counts.total === 0) return 0.5;

    const supportiveWeight = counts.byCategory.supportive / counts.total;
    const criticalWeight = counts.byCategory.critical / counts.total;

    // Scale from 0 (all critical) to 1 (all supportive)
    return 0.5 + (supportiveWeight - criticalWeight) * 0.5;
  }

  private computeEffectivenessScore(
    learningSignals: LearningSignalRecord[],
    completions: CompletionSignalRecord[]
  ): number {
    if (learningSignals.length === 0 && completions.length === 0) {
      return 0.5; // Neutral when no data
    }

    let score = 0;
    let weight = 0;

    // Weight learning signals
    if (learningSignals.length > 0) {
      const avgMastery =
        learningSignals.reduce((sum, s) => {
          const masteryLevel = (s.payload?.['masteryLevel'] as string) ?? 'none';
          return sum + this.masteryToNumber(masteryLevel);
        }, 0) / learningSignals.length;
      score += avgMastery * 0.6;
      weight += 0.6;
    }

    // Weight completions
    if (completions.length > 0) {
      const successRate = completions.filter(c => c.passed).length / completions.length;
      score += successRate * 0.4;
      weight += 0.4;
    }

    return weight > 0 ? score / weight : 0.5;
  }

  private computeConsensusState(stats: FeedbackStats): ConsensusState {
    if (stats.totalResponses === 0) {
      return {
        level: 'unknown',
        confidence: 0,
        dominantPosition: null,
        polarization: 0,
      };
    }

    // Find dominant position
    let maxCount = 0;
    let dominantPosition: string | null = null;
    for (const [position, count] of Object.entries(stats.distribution)) {
      if (count > maxCount) {
        maxCount = count;
        dominantPosition = position;
      }
    }

    const dominantRatio = maxCount / stats.totalResponses;

    // Calculate polarization (how spread out are the positions)
    const positionCount = Object.keys(stats.distribution).length;
    const evenDistribution = 1 / positionCount;
    let polarization = 0;

    for (const count of Object.values(stats.distribution)) {
      const ratio = count / stats.totalResponses;
      polarization += Math.abs(ratio - evenDistribution);
    }
    polarization = polarization / 2; // Normalize to 0-1

    // Determine consensus level
    let level: 'strong' | 'moderate' | 'contested' | 'unknown';
    if (dominantRatio >= 0.7) {
      level = 'strong';
    } else if (dominantRatio >= 0.5) {
      level = 'moderate';
    } else {
      level = 'contested';
    }

    return {
      level,
      confidence: stats.totalResponses / 10, // Scale with sample size
      dominantPosition,
      polarization,
    };
  }

  private masteryToNumber(mastery: string): number {
    const levels: Record<string, number> = {
      none: 0,
      exposure: 0.2,
      familiarity: 0.4,
      competence: 0.6,
      proficiency: 0.8,
      mastery: 1,
    };
    return levels[mastery.toLowerCase()] ?? 0.5;
  }

  private buildRespondentVectors(
    feedback: GraduatedFeedbackRecord[],
    reactions: ReactionRecord[]
  ): Map<string, number[]> {
    const vectors = new Map<string, number[]>();

    // Build from feedback (position as primary dimension)
    for (const f of feedback) {
      if (!vectors.has(f.respondentId)) {
        vectors.set(f.respondentId, []);
      }
      vectors.get(f.respondentId)!.push(f.positionIndex);
    }

    // Augment with reaction sentiment
    for (const r of reactions) {
      if (!vectors.has(r.reactorId)) {
        vectors.set(r.reactorId, []);
      }
      const sentimentValue = r.category === 'supportive' ? 0.8 : 0.2;
      vectors.get(r.reactorId)!.push(sentimentValue);
    }

    return vectors;
  }

  private clusterRespondents(vectors: Map<string, number[]>): OpinionCluster[] {
    if (vectors.size < 2) {
      return [];
    }

    // Simple clustering: group by average position
    const respondentAverages: { id: string; avg: number }[] = [];

    for (const [id, values] of vectors.entries()) {
      const avg = values.reduce((a, b) => a + b, 0) / values.length;
      respondentAverages.push({ id, avg });
    }

    // Sort and partition into clusters
    respondentAverages.sort((a, b) => a.avg - b.avg);

    const clusters: OpinionCluster[] = [];
    const clusterSize = Math.ceil(respondentAverages.length / 3);

    // Create up to 3 clusters
    for (let i = 0; i < 3 && i * clusterSize < respondentAverages.length; i++) {
      const start = i * clusterSize;
      const end = Math.min(start + clusterSize, respondentAverages.length);
      const clusterMembers = respondentAverages.slice(start, end);

      if (clusterMembers.length > 0) {
        const avgPosition =
          clusterMembers.reduce((sum, m) => sum + m.avg, 0) / clusterMembers.length;

        clusters.push({
          id: `cluster-${i}`,
          label: this.getClusterLabel(avgPosition),
          memberCount: clusterMembers.length,
          centroid: [avgPosition, 0], // 2D: [position, 0] for now
          averagePosition: avgPosition,
          color: this.getClusterColor(avgPosition),
        });
      }
    }

    return clusters;
  }

  private getClusterLabel(position: number): string {
    if (position < 0.33) return 'Skeptical';
    if (position < 0.66) return 'Moderate';
    return 'Supportive';
  }

  private getClusterColor(position: number): string {
    if (position < 0.33) return '#e74c3c'; // Red
    if (position < 0.66) return '#f39c12'; // Orange
    return '#27ae60'; // Green
  }
}

// ===========================================================================
// Types
// ===========================================================================

export interface ReactionRecord {
  id: string;
  contentId: string;
  reactorId: string;
  reactionType: EmotionalReactionType;
  category: 'supportive' | 'critical';
  context?: string;
  private: boolean;
  createdAt: string;
}

export interface ReactionCounts {
  total: number;
  byType: Partial<Record<EmotionalReactionType, number>>;
  byCategory: { supportive: number; critical: number };
}

export interface GraduatedFeedbackInput {
  context: string; // 'accuracy' | 'usefulness' | 'proposal'
  position: string; // The selected position label
  positionIndex: number; // Normalized 0-1
  intensity: number; // 1-10 (ARCH pattern)
  reasoning?: string;
}

export interface GraduatedFeedbackRecord extends GraduatedFeedbackInput {
  id: string;
  contentId: string;
  respondentId: string;
  createdAt: string;
}

export interface FeedbackStats {
  totalResponses: number;
  averagePosition: number;
  averageIntensity: number;
  distribution: Record<string, number>;
  contexts: string[];
}

export interface LearningSignalInput {
  contentId: string;
  signalType:
    | 'content_viewed'
    | 'progress_update'
    | 'quiz_attempt'
    | 'mastery_achieved'
    | 'interactive_completion';
  payload: Record<string, unknown>;
}

export interface LearningSignalRecord extends LearningSignalInput {
  id: string;
  learnerId: string;
  createdAt: string;
}

export interface CompletionSignalInput {
  contentId: string;
  interactionType: string; // renderer type
  passed: boolean;
  score?: number;
  details?: Record<string, unknown>;
}

export interface CompletionSignalRecord extends CompletionSignalInput {
  id: string;
  completedBy: string;
  completedAt: string;
}

export interface AggregatedSignals {
  contentId: string;
  reactionCounts: ReactionCounts;
  feedbackStats: FeedbackStats;
  learningSignalCount: number;
  completionCount: number;
  sentimentScore: number; // 0-1
  effectivenessScore: number; // 0-1
  consensusState: ConsensusState;
  lastUpdated: string;
}

export interface PathSignalAggregate {
  pathId: string;
  totalLearners: number;
  completionsByStep: Record<number, number>;
  averageMasteryByStep: Record<number, number>;
  averageTimeByStep: Record<number, number>;
  scaffoldingScoreByStep: Record<number, number>;
  overallEffectiveness: number;
  lastUpdated: string;
}

export interface ConsensusState {
  level: 'strong' | 'moderate' | 'contested' | 'unknown';
  confidence: number;
  dominantPosition: string | null;
  polarization: number;
}

export interface AttestationEvidence {
  evidenceType: string;
  score?: number;
  attempts?: number;
  [key: string]: unknown;
}

export interface AttestationSuggestion {
  id: string;
  contentId: string;
  attestationType: string;
  evidence: AttestationEvidence;
  suggestedAt: string;
  suggestedBy: string;
  status: 'pending' | 'approved' | 'rejected';
}

export interface ReviewFlagEvidence {
  evidenceType: string;
  attempts?: number;
  avgScore?: number;
  [key: string]: unknown;
}

export interface ReviewFlag {
  id: string;
  contentId: string;
  concern: string;
  evidence: ReviewFlagEvidence;
  flaggedAt: string;
  flaggedBy: string;
  status: 'pending' | 'acknowledged' | 'resolved';
}

export interface SignalChangeEvent {
  type:
    | 'reaction'
    | 'graduated-feedback'
    | 'learning-signal'
    | 'completion'
    | 'review-flag'
    | 'mediation-proceed';
  contentId: string;
  signalId: string;
  agentId: string;
  timestamp: string;
}

export interface OpinionCluster {
  id: string;
  label: string;
  memberCount: number;
  centroid: [number, number]; // 2D position
  averagePosition: number;
  color: string;
}

export interface ConsensusStatement {
  context: string;
  statementType: 'consensus' | 'divisive';
  averagePosition: number;
  standardDeviation: number;
  responseCount: number;
  agreePercentage: number;
}
