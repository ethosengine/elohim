// Retired: DWELL_THRESHOLD_MS, sessionViewed Set, pendingTimers, EventService dependency (M-REA-2).
// Substrate policy (tending-policy Manifest) qualifies dwell; deduplication via re-tend.
import { Injectable, OnDestroy, inject } from '@angular/core';

import { Subscription } from 'rxjs';

import { AttentionTendingApiService } from '@elohim/rea-runtime';

@Injectable({ providedIn: 'root' })
export class AttentionTrackerService implements OnDestroy {
  private readonly attentionApi = inject(AttentionTendingApiService);

  /** Mount timestamps keyed by content ID (set on trackContentView). */
  private readonly mountedAt = new Map<string, number>();

  /** Active subscriptions for cleanup. */
  private readonly subscriptions: Subscription[] = [];

  /** Record that the agent mounted a content node. */
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
        contextJson: JSON.stringify({ pillar: 'shefa' }),
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
