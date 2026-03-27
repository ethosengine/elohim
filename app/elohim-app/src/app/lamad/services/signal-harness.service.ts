import { Injectable, inject } from '@angular/core';

import { AgentService } from '@app/elohim/services/agent.service';
import type { REAAction, LamadEventType } from '@app/elohim/models';
import { EconomicEventsApiService, type CreateEconomicEventInput } from '@app/shefa';
import type { RendererCompletionEvent } from '../renderers/renderer-registry.service';
import {
  LAMAD_COUPLING_MAP,
  type ContentTypeCoupling,
} from '@app/lamad/generated/coupling-map';

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
  private readonly agentService = inject(AgentService);

  /** Minimal shape required from a content node for signal translation. */
  async onRendererComplete(
    node: { id: string; contentType: string; contentFormat: string },
    event: RendererCompletionEvent,
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

    await this.economicEventsApi.createEconomicEvent(economicEvent);
  }

  private getCoupling(contentType: string): ContentTypeCoupling | undefined {
    return LAMAD_COUPLING_MAP[contentType];
  }

  private getSignalType(
    coupling: ContentTypeCoupling,
    lifecycle: string,
  ): string | undefined {
    const signalTypes = coupling.governance?.signalTypes ?? [];
    if (lifecycle === 'onComplete') {
      return signalTypes.find(
        s => s.includes('mastery') || s.includes('completed'),
      );
    }
    return signalTypes.find(
      s => s.includes('learning') || s.includes('engagement'),
    );
  }

  private inferLamadEventType(event: RendererCompletionEvent): LamadEventType {
    if (event.type === 'quiz' && event.passed) return 'assessment-complete';
    if (event.type === 'quiz') return 'assessment-start';
    if (event.type === 'simulation') return 'content-complete';
    if (event.type === 'view') return 'content-view';
    return 'content-complete';
  }
}
