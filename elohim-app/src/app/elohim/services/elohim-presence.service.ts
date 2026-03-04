/**
 * ElohimPresenceService — The "when and where" orchestrator.
 *
 * Observes learner moments across pillars and decides when to invoke the elohim.
 * Implements BannerNoticeProvider so presence moments appear as banner notices.
 *
 * This is the nudge/play/resolve orchestrator from the manifesto:
 * - nudge: gentle suggestions after learning moments
 * - play: celebrations of achievement
 * - resolve: care interventions when spiraling is detected
 */

import { Injectable, OnDestroy, inject } from '@angular/core';
import { Router } from '@angular/router';

import { map, switchMap } from 'rxjs/operators';

import { BehaviorSubject, Observable, Subscription, of } from 'rxjs';

import { LearnerContextService } from '@app/lamad/services/learner-context.service';
import {
  PathRecommendationService,
  type PathRecommendation,
} from '@app/lamad/services/path-recommendation.service';

import { BannerService } from './banner.service';
import { ElohimAgentService } from './elohim-agent.service';

import type { BannerNotice, BannerNoticeProvider } from '../models/banner-notice.model';
import type {
  ElohimCapability,
  ElohimResponse,
  ElohimComputationCost,
} from '../models/elohim-agent.model';
import type {
  ElohimPresenceMoment,
  ElohimPresenceType,
  PresenceAction,
} from '../models/elohim-presence.model';

const PROVIDER_ID = 'elohim-presence';
const CAPABILITY_KNOWLEDGE_MAP: ElohimCapability = 'knowledge-map-synthesis';
const CAPABILITY_SPIRAL: ElohimCapability = 'spiral-detection';

@Injectable({ providedIn: 'root' })
export class ElohimPresenceService implements BannerNoticeProvider, OnDestroy {
  readonly providerId = PROVIDER_ID;

  private readonly router = inject(Router);
  private readonly moments$ = new BehaviorSubject<ElohimPresenceMoment[]>([]);
  private readonly sessionCost$ = new BehaviorSubject<ElohimComputationCost>({
    tokensProcessed: 0,
    timeMs: 0,
    constitutionalChecks: 0,
    precedentLookups: 0,
  });
  private readonly dismissed = new Set<string>();
  private readonly subscriptions = new Subscription();

  /** All active presence moments. */
  readonly presence$: Observable<ElohimPresenceMoment[]> = this.moments$.asObservable();

  /** Running session cost for transparency. */
  readonly cost$: Observable<ElohimComputationCost> = this.sessionCost$.asObservable();

  /** BannerNoticeProvider — maps presence moments to banner notices. */
  readonly notices$: Observable<BannerNotice[]> = this.moments$.pipe(
    map(moments => moments.filter(m => !this.dismissed.has(m.id)).map(m => this.toBannerNotice(m)))
  );

  private readonly agentService = inject(ElohimAgentService);
  private readonly bannerService = inject(BannerService);
  private readonly pathRecommendation = inject(PathRecommendationService);
  private readonly learnerContext = inject(LearnerContextService);

  constructor() {
    this.bannerService.registerProvider(this);
  }

  ngOnDestroy(): void {
    this.bannerService.unregisterProvider(PROVIDER_ID);
    this.subscriptions.unsubscribe();
  }

  // =========================================================================
  // Learner Moment Hooks
  // =========================================================================

  /**
   * After discovery assessment completion → path recommendation.
   *
   * This is the primary integration point: the learner just completed
   * a self-discovery assessment and the elohim offers an insight about
   * what they learned about themselves and where to go next.
   */
  onDiscoveryCompleted(
    contentNodeId: string,
    assessmentTitle: string,
    requesterId: string
  ): Observable<ElohimResponse> {
    const response$ = this.agentService.invoke({
      requestId: this.generateId(),
      targetElohimId: 'auto',
      capability: 'path-recommendation',
      params: {
        type: 'path-analysis',
        pathId: contentNodeId,
        analysisType: 'learning-objectives',
      },
      requesterId,
      constitutionalContext: `Learner completed discovery assessment: ${assessmentTitle}`,
      priority: 'normal',
      requestedAt: new Date().toISOString(),
    });

    this.subscriptions.add(
      response$
        .pipe(
          switchMap(response => {
            if (response.status !== 'fulfilled') return of(null);

            const profile = this.learnerContext.getDiscoveryProfile();
            if (!profile) {
              this.addDiscoveryMoment(response, assessmentTitle, null, contentNodeId);
              return of(null);
            }

            return this.pathRecommendation.getTopRecommendation(profile).pipe(
              map(rec => {
                this.addDiscoveryMoment(response, assessmentTitle, rec, contentNodeId);
                return null;
              })
            );
          })
        )
        .subscribe()
    );

    return response$;
  }

  /**
   * After content step completion → knowledge map synthesis.
   */
  onContentCompleted(
    contentId: string,
    masteryLevel: string,
    requesterId: string
  ): Observable<ElohimResponse> {
    const response$ = this.agentService.invoke({
      requestId: this.generateId(),
      targetElohimId: 'auto',
      capability: CAPABILITY_KNOWLEDGE_MAP,
      params: {
        type: 'knowledge-map-synthesis',
        subjectType: 'self',
        subjectId: requesterId,
      },
      requesterId,
      constitutionalContext: `Learner completed content with mastery: ${masteryLevel}`,
      priority: 'low',
      requestedAt: new Date().toISOString(),
    });

    this.subscriptions.add(
      response$.subscribe(response => {
        if (response.status === 'fulfilled') {
          this.addMoment({
            type: masteryLevel === 'mastered' ? 'celebration' : 'nudge',
            capability: CAPABILITY_KNOWLEDGE_MAP,
            response,
            message:
              masteryLevel === 'mastered'
                ? 'Your knowledge map is growing. Well done.'
                : 'Your understanding is building. Keep going.',
          });
        }
      })
    );

    return response$;
  }

  /**
   * Periodic check → spiral detection (care).
   */
  checkLearnerWellbeing(agentId: string): Observable<ElohimResponse> {
    const response$ = this.agentService.invoke({
      requestId: this.generateId(),
      targetElohimId: 'auto',
      capability: CAPABILITY_SPIRAL,
      params: {
        type: 'spiral-detection',
        agentId,
      },
      requesterId: agentId,
      priority: 'normal',
      requestedAt: new Date().toISOString(),
    });

    this.subscriptions.add(
      response$.subscribe(response => {
        if (response.status === 'fulfilled' && response.payload) {
          const payload = response.payload as { spiralDetected?: boolean };
          if (payload.spiralDetected) {
            this.addMoment({
              type: 'care',
              capability: CAPABILITY_SPIRAL,
              response,
              message: 'I notice you might benefit from a pause. Your wellbeing matters.',
              actions: [
                { id: 'take-break', label: 'Take a Break', primary: true },
                { id: 'continue', label: "I'm OK" },
              ],
            });
          }
        }
      })
    );

    return response$;
  }

  /**
   * Before attestation grant → constitutional verification.
   */
  verifyAttestation(
    contentId: string,
    attestationType: string,
    requesterId: string
  ): Observable<ElohimResponse> {
    return this.agentService.requestAttestationRecommendation(
      contentId,
      attestationType,
      requesterId
    );
  }

  // =========================================================================
  // BannerNoticeProvider
  // =========================================================================

  dismissNotice(noticeId: string): void {
    this.dismissed.add(noticeId);
    this.moments$.next(this.moments$.value);
  }

  handleAction(noticeId: string, actionId: string): void {
    if (actionId === 'dismiss') {
      this.dismissNotice(noticeId);
      return;
    }

    if (actionId === 'view-path') {
      this.handleViewRecommendedPath(noticeId);
    }

    // Other actions handled by consumers via presence$ stream
  }

  private handleViewRecommendedPath(noticeId: string): void {
    const moment = this.moments$.value.find(m => m.id === noticeId);
    if (!moment) return;

    const pathId = moment.metadata?.['recommendedPathId'] as string | undefined;
    if (pathId) {
      void this.router.navigate(['/lamad/path', pathId]);
    } else {
      // Fallback: navigate to path catalog
      void this.router.navigate(['/lamad']);
    }

    this.dismissNotice(noticeId);
  }

  // =========================================================================
  // Internals
  // =========================================================================

  private addDiscoveryMoment(
    response: ElohimResponse,
    assessmentTitle: string,
    recommendation: PathRecommendation | null,
    fallbackContentNodeId: string
  ): void {
    const message = recommendation
      ? `Based on your ${assessmentTitle} results, "${recommendation.pathTitle}" looks like a great next step.`
      : this.formatInsightMessage(response, assessmentTitle);

    this.addMoment({
      type: 'insight',
      capability: 'path-recommendation',
      response,
      message,
      actions: [
        { id: 'view-path', label: 'View Recommended Path', primary: true },
        { id: 'dismiss', label: 'Maybe Later' },
      ],
      metadata: { recommendedPathId: recommendation?.pathId ?? fallbackContentNodeId },
    });
  }

  private addMoment(opts: {
    type: ElohimPresenceType;
    capability: ElohimCapability;
    response: ElohimResponse;
    message: string;
    actions?: PresenceAction[];
    metadata?: Record<string, unknown>;
  }): void {
    const moment: ElohimPresenceMoment = {
      id: this.generateId(),
      type: opts.type,
      capability: opts.capability,
      message: opts.message,
      reasoning: opts.response.constitutionalReasoning,
      actions: opts.actions,
      cost: opts.response.cost,
      dismissible: true,
      createdAt: new Date().toISOString(),
      metadata: opts.metadata,
    };

    // Accumulate session cost
    if (opts.response.cost) {
      const current = this.sessionCost$.value;
      this.sessionCost$.next({
        tokensProcessed: current.tokensProcessed + opts.response.cost.tokensProcessed,
        timeMs: current.timeMs + opts.response.cost.timeMs,
        constitutionalChecks:
          current.constitutionalChecks + opts.response.cost.constitutionalChecks,
        precedentLookups: current.precedentLookups + opts.response.cost.precedentLookups,
      });
    }

    this.moments$.next([...this.moments$.value, moment]);
  }

  private toBannerNotice(moment: ElohimPresenceMoment): BannerNotice {
    return {
      id: moment.id,
      providerId: PROVIDER_ID,
      severity: moment.type === 'care' ? 'warning' : 'info',
      priority: 'agent',
      contexts: ['global', 'lamad'],
      title: this.getPresenceTitle(moment.type),
      message: moment.message,
      actions: moment.actions?.map(a => ({ id: a.id, label: a.label })),
      dismissible: moment.dismissible,
      createdAt: new Date(moment.createdAt),
    };
  }

  private getPresenceTitle(type: ElohimPresenceType): string {
    const titles: Record<ElohimPresenceType, string> = {
      nudge: 'Elohim Suggestion',
      insight: 'Elohim Insight',
      recommendation: 'Elohim Recommendation',
      care: 'Elohim Care',
      celebration: 'Elohim Celebration',
    };
    return titles[type];
  }

  private formatInsightMessage(response: ElohimResponse, assessmentTitle: string): string {
    const principle = response.constitutionalReasoning.primaryPrinciple;
    return (
      `Based on your ${assessmentTitle} results, I have a learning path recommendation. ` +
      `(Guided by: ${principle})`
    );
  }

  private generateId(): string {
    const randomBytes = crypto.getRandomValues(new Uint8Array(8));
    const randomStr = Array.from(randomBytes)
      .map(b => b.toString(36))
      .join('')
      .substring(0, 9);
    return `presence-${Date.now()}-${randomStr}`;
  }
}
