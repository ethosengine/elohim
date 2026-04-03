import { Injectable, OnDestroy, inject } from '@angular/core';

import { Subscription } from 'rxjs';

import { AgentService } from '@app/elohim/services/agent.service';
import { EventService } from './event.service';

/** Minimum milliseconds on content before recording a view event. */
const DWELL_THRESHOLD_MS = 3000;

/**
 * AttentionTrackerService — Records content attention as economic events.
 *
 * Orchestrates dwell-time qualification, per-session deduplication,
 * and delegates to EventService for the actual REA event creation.
 * This replaces Google Analytics with protocol-native attention tracking.
 */
@Injectable({ providedIn: 'root' })
export class AttentionTrackerService implements OnDestroy {
  private readonly eventService = inject(EventService);
  private readonly agentService = inject(AgentService);

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

    const agentId = this.agentService.getCurrentAgentId();
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
