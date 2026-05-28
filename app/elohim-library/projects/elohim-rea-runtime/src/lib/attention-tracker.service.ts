// M-REA-2: DWELL_THRESHOLD_MS, sessionViewed, pendingTimers retired — substrate qualifies dwell via tending-policy Manifest.

import { Injectable, InjectionToken, OnDestroy, inject } from '@angular/core';

import { Subscription } from 'rxjs';

import { AttentionTendingApiService } from './attention-tending-api.service';

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
  private readonly attentionApi = inject(AttentionTendingApiService);
  private readonly agentContext = inject(AGENT_CONTEXT);

  /** Mount timestamps keyed by content ID (set on trackContentView). */
  private readonly mountedAt = new Map<string, number>();

  /** Active subscriptions for cleanup. */
  private readonly subscriptions: Subscription[] = [];

  /**
   * Record that the agent mounted a content node.
   * Captures the mount timestamp for elapsed-time calculation on leave.
   */
  trackContentView(contentId: string): void {
    if (!this.mountedAt.has(contentId)) {
      this.mountedAt.set(contentId, Date.now());
    }
  }

  /**
   * Record that the agent left a content node.
   * Sends elapsed time to POST /api/v1/attention/tending. The route evaluates
   * dwell qualification against the tending-policy Manifest (substrate policy).
   */
  trackContentLeave(contentId: string): void {
    const mountTime = this.mountedAt.get(contentId);
    if (mountTime == null) return;

    this.mountedAt.delete(contentId);

    const elapsedMs = Date.now() - mountTime;
    const sub = this.attentionApi
      .postTending({
        filterSubjectJson: JSON.stringify({ contentId }),
        classification: 'values-forward',
        ttlSeconds: 3600,
        contextJson: JSON.stringify({ pillar: 'lamad' }),
        elapsedMs,
      })
      .subscribe();
    this.subscriptions.push(sub);
  }

  ngOnDestroy(): void {
    this.mountedAt.clear();
    for (const sub of this.subscriptions) {
      sub.unsubscribe();
    }
  }
}
