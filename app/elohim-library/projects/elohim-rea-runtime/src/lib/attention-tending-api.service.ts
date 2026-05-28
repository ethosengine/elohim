/**
 * AttentionTendingApiService — Thin HTTP wrapper for POST /api/v1/attention/tending.
 *
 * M-REA-2: the substrate-correct attention tracking route. Sends an
 * AttentionTendingIntent to the storage API, which dwell-qualifies and proxies
 * to the create_attention_tending coordinator on the content_store zome.
 *
 * This service is injected by AttentionTrackerService. It is separate from
 * EventService because AttentionTending is a private source-chain entry
 * (Visibility::Private), not an EconomicEvent — they are categorically
 * different substrate types.
 */

import { Injectable, inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';

import { Observable } from 'rxjs';

// ---------------------------------------------------------------------------
// Wire types (aligned with intents/attention-tending-intent.schema.json)
// ---------------------------------------------------------------------------

/** Wire shape for the POST /api/v1/attention/tending request body. */
export interface AttentionTendingIntent {
  /** JSON-encoded FilterSubject identifying the content node or path. */
  filterSubjectJson: string;
  /** Attention classification: values-forward | fatigue | scope-mismatch | safety. */
  classification: 'values-forward' | 'fatigue' | 'scope-mismatch' | 'safety';
  /** Optional human-readable reason (never shared cross-agent). */
  reason?: string;
  /** TTL in seconds; minimum 3600 (integrity zome floor). */
  ttlSeconds: number;
  /** JSON-encoded ContextScope (pillar, surface, session phase). */
  contextJson: string;
  /**
   * Elapsed milliseconds on the content node. The route compares this against
   * the tending-policy Manifest's dwell_qualification_ms (default 3000).
   * Replaces the retired client-side DWELL_THRESHOLD_MS = 3000 constant.
   */
  elapsedMs: number;
}

/** Response from POST /api/v1/attention/tending. */
export interface AttentionTendingAck {
  /** Whether the tending was forwarded to the coordinator. */
  accepted: boolean;
  /** Hex-encoded action hash when accepted. */
  actionHash?: string;
  /** Reason when not accepted (e.g. 'below_dwell_threshold' | 'no_conductor'). */
  skipReason?: string;
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

@Injectable({ providedIn: 'root' })
export class AttentionTendingApiService {
  private readonly http = inject(HttpClient);

  /**
   * POST /api/v1/attention/tending
   *
   * Sends an attention tending intent. The route dwell-qualifies against the
   * substrate tending-policy Manifest before calling the coordinator. The
   * client does not need to track the dwell threshold — the substrate does.
   */
  postTending(intent: AttentionTendingIntent): Observable<AttentionTendingAck> {
    return this.http.post<AttentionTendingAck>('/api/v1/attention/tending', intent);
  }
}
