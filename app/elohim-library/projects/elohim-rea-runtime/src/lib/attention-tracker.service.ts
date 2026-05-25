/**
 * AttentionTrackerService — Records content attention as economic events.
 *
 * Migrated to @elohim/rea-runtime from @app/shefa/services/attention-tracker.service
 * as part of Wave 2 Slice 2.4 of the cross-pillar import cleanup sprint.
 *
 * Orchestrates dwell-time qualification, per-session deduplication,
 * and delegates to EventService for the actual REA event creation.
 * This replaces Google Analytics with protocol-native attention tracking.
 *
 * TODO(slice-2.1): AgentService will migrate from @app/elohim/services to
 * @elohim/service when Slice 2.1 completes. Update the provider registration
 * in elohim-app's DI root at that time (the AGENT_CONTEXT token stays here).
 */

import { Injectable, InjectionToken, OnDestroy, inject } from '@angular/core';

import { Subscription } from 'rxjs';

import { EventService } from './event.service';

/** Minimum milliseconds on content before recording a view event. */
export const DWELL_THRESHOLD_MS = 3000;

// =============================================================================
// IAgentContext — minimal interface for agent identity lookup
// =============================================================================

/**
 * IAgentContext — the minimal agent API that AttentionTrackerService needs.
 *
 * AgentService in elohim-app implements this interface. Libraries inject
 * this token; elohim-app provides the concrete implementation.
 */
export interface IAgentContext {
  getCurrentAgentId(): string;
}

/**
 * DI token for the agent context.
 *
 * elohim-app must provide AgentService under this token:
 *   { provide: AGENT_CONTEXT, useExisting: AgentService }
 *
 * Tests provide a mock:
 *   { provide: AGENT_CONTEXT, useValue: { getCurrentAgentId: () => 'agent-id' } }
 */
export const AGENT_CONTEXT = new InjectionToken<IAgentContext>('AgentContext', {
  factory: () => {
    throw new Error(
      '[rea-runtime] AGENT_CONTEXT not provided. ' +
        'Add { provide: AGENT_CONTEXT, useExisting: AgentService } to your app providers.'
    );
  },
});

// =============================================================================
// AttentionTrackerService
// =============================================================================

@Injectable({ providedIn: 'root' })
export class AttentionTrackerService implements OnDestroy {
  private readonly eventService = inject(EventService);
  private readonly agentContext = inject(AGENT_CONTEXT);

  /** Content IDs that have had a qualified view in this session. */
  private readonly sessionViewed = new Set<string>();

  /** Pending dwell timers keyed by content ID. */
  private readonly pendingTimers = new Map<string, ReturnType<typeof setTimeout>>();

  /** Active subscriptions for cleanup. */
  private readonly subscriptions: Subscription[] = [];

  /**
   * Start tracking a content view. After DWELL_THRESHOLD_MS, records
   * a content-view economic event (unless already viewed this session).
   */
  trackContentView(contentId: string): void {
    // Already viewed this session — skip
    if (this.sessionViewed.has(contentId)) return;

    // Cancel any existing timer for this content
    this.cancelTimer(contentId);

    // Start dwell timer
    const timer = setTimeout(() => {
      this.recordQualifiedView(contentId);
      this.pendingTimers.delete(contentId);
    }, DWELL_THRESHOLD_MS);

    this.pendingTimers.set(contentId, timer);
  }

  /**
   * Stop tracking a content view. Cancels the dwell timer if the
   * threshold hasn't been met yet.
   */
  trackContentLeave(contentId: string): void {
    this.cancelTimer(contentId);
  }

  /**
   * Returns the set of content IDs viewed this session (qualified views only).
   */
  getSessionViewedIds(): ReadonlySet<string> {
    return this.sessionViewed;
  }

  ngOnDestroy(): void {
    // Clear all pending timers
    for (const timer of this.pendingTimers.values()) {
      clearTimeout(timer);
    }
    this.pendingTimers.clear();

    // Clean up subscriptions
    for (const sub of this.subscriptions) {
      sub.unsubscribe();
    }
  }

  private recordQualifiedView(contentId: string): void {
    this.sessionViewed.add(contentId);

    const agentId = this.agentContext.getCurrentAgentId();
    const sub = this.eventService
      .recordContentInteraction(agentId, contentId, 'content-view')
      .subscribe();
    this.subscriptions.push(sub);
  }

  private cancelTimer(contentId: string): void {
    const existing = this.pendingTimers.get(contentId);
    if (existing) {
      clearTimeout(existing);
      this.pendingTimers.delete(contentId);
    }
  }
}
