/**
 * AttentionTrackerService — Records content attention as native AttentionTending
 * entries on the agent's private source chain.
 *
 * ## M-REA-2: substrate-correct attention tracking
 *
 * Sends `POST /api/v1/attention/tending` when the agent leaves a content node.
 * The route:
 *   1. Compares elapsedMs against the tending-policy Manifest's
 *      dwell_qualification_ms (substrate policy, not client constant).
 *   2. If qualified, calls `create_attention_tending` on the content_store zome.
 *   3. Returns {accepted: boolean, skipReason?: string}.
 *
 * The substrate deduplicates re-tends via `refresh_tending_ttl` (tended_at
 * vec append). The client's `sessionViewed` Set is no longer needed.
 *
 * ## What was removed (F-POLICY-4 + F-REA-2 retirement)
 *
 * - `DWELL_THRESHOLD_MS = 3000` constant → now substrate policy via
 *   Manifest{kind: "tending-policy", payload_json: {dwell_qualification_ms: N}}.
 * - `sessionViewed: Set<string>` → substrate deduplicates via re-tend.
 * - `pendingTimers` / `setTimeout` orchestration → substrate gates dwell at route.
 * - `EventService.recordContentInteraction` dependency → content-view tracking
 *   IS AttentionTending, not a separate EconomicEvent.
 *
 * TODO(slice-2.1): AgentService will migrate from @app/elohim/services to
 * @elohim/service when Slice 2.1 completes. Update the provider registration
 * in elohim-app's DI root at that time (the AGENT_CONTEXT token stays here).
 */

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
