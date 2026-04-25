import { Injectable, inject } from '@angular/core';

import { AgentService } from '@app/elohim/services/agent.service';
import { LAMAD_COUPLING_MAP, type ContentTypeCoupling } from '@app/lamad/generated/coupling-map';
import {
  EconomicEventsApiService,
  SignalEmitService,
  type CreateEconomicEventInput,
  type SignalIntent,
} from '@app/shefa';

import type { RendererCompletionEvent } from '../renderers/renderer-registry.service';
import type { REAAction, LamadEventType } from '@app/elohim/models';

/**
 * Feature flag for the EPR write-through path (Task C.3).
 *
 * When `true`, the harness first POSTs a SignalIntent to
 * `/api/v1/signal/emit`. If write-through is ON for shefa/EconomicEvent the
 * intent is composed, signed by the conductor, ingested as an EPR, and
 * projected — the result is the same row in `economic_events` plus a
 * notarized envelope. If write-through is OFF (HTTP 503), the harness
 * falls back to the legacy POST `/api/v1/economic-events` path.
 *
 * When `false`, the harness skips the new endpoint entirely and uses the
 * legacy path. Defaults to `true` so the migration is opt-out, but the
 * hardcoded boolean keeps the kill-switch close at hand while shefa is the
 * only pillar exercising the path. Future pillars will read this from a
 * runtime flag service.
 */
const USE_SIGNAL_EMIT_ENDPOINT = true;

/**
 * SignalHarnessService — bridge between renderer output and protocol input.
 *
 * Reads manifest coupling declarations (via generated LAMAD_COUPLING_MAP)
 * and translates RendererCompletionEvent → CreateEconomicEventInput.
 * This is the only path from renderer to protocol — apps can't skip
 * economic events because the harness IS the render-to-protocol bridge.
 */
@Injectable({ providedIn: 'root' })
export class SignalHarnessService {
  private readonly economicEventsApi = inject(EconomicEventsApiService);
  private readonly signalEmit = inject(SignalEmitService);
  private readonly agentService = inject(AgentService);

  /** Minimal shape required from a content node for signal translation. */
  async onRendererComplete(
    node: { id: string; contentType: string; contentFormat: string },
    event: RendererCompletionEvent
  ): Promise<void> {
    const agentId = this.agentService.getCurrentAgentId();
    const coupling = this.getCoupling(node.contentType);
    if (!coupling?.value) return;

    // Determine which lifecycle event fired
    const lifecycle = event.passed ? 'onComplete' : 'onConsume';
    const valueFlow = coupling.value[lifecycle];
    if (!valueFlow) return;

    const economicEvent: CreateEconomicEventInput = {
      action: valueFlow.action as REAAction,
      providerId: agentId,
      receiverId: node.id,
      resourceConformsTo: valueFlow.resourceConformsTo,
      lamadEventType: this.inferLamadEventType(event),
      note: `Signal harness: ${node.contentType}/${event.type}`,
    };

    // Try the EPR write-through path first (Task C.3). On 503 (write-through
    // OFF for shefa/EconomicEvent) fall back to the legacy economic-events
    // endpoint so consumers still see the row even before the operator opts
    // shefa in. Other errors propagate.
    if (USE_SIGNAL_EMIT_ENDPOINT) {
      const intent: SignalIntent = {
        pillar: 'shefa',
        signalType: 'EconomicEvent',
        agentCid: agentId,
        payload: economicEvent as unknown as Record<string, unknown>,
        couplingRefs: {
          // Lamad node id is the knowledge-leg referent; future Phase 4
          // resolves these to actual EPR CIDs via the manifest registry.
          knowledge: node.id,
        },
      };
      const result = await this.signalEmit.tryEmit(intent);
      if (result.status === 'emitted') {
        return;
      }
      if (result.status === 'error') {
        throw new Error(`Signal emit failed (HTTP ${result.status_code}): ${result.message}`);
      }
      // status === 'fallback' → drop through to legacy path.
    }

    await this.economicEventsApi.createEconomicEvent(economicEvent);
  }

  private getCoupling(contentType: string): ContentTypeCoupling | undefined {
    return LAMAD_COUPLING_MAP[contentType];
  }

  private getSignalType(coupling: ContentTypeCoupling, lifecycle: string): string | undefined {
    const signalTypes = coupling.governance?.signalTypes ?? [];
    if (lifecycle === 'onComplete') {
      return signalTypes.find(s => s.includes('mastery') || s.includes('completed'));
    }
    return signalTypes.find(s => s.includes('learning') || s.includes('engagement'));
  }

  private inferLamadEventType(event: RendererCompletionEvent): LamadEventType {
    if (event.type === 'quiz' && event.passed) return 'assessment-complete';
    if (event.type === 'quiz') return 'assessment-start';
    if (event.type === 'simulation') return 'content-complete';
    if (event.type === 'view') return 'content-view';
    return 'content-complete';
  }
}
