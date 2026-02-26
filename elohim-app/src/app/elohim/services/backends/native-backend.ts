/**
 * Native Backend — Doorway proxy to the Rust elohim-agent crate.
 *
 * Production path: Angular → doorway /api/v1/elohim/invoke → Rust ElohimAgentService.
 * Stub until the doorway HTTP route is implemented.
 */

import { Observable, from } from 'rxjs';

import type { ElohimAgent, ElohimRequest, ElohimResponse } from '../../models/elohim-agent.model';
import type { ElohimBackend, ElohimBackendType } from '../elohim-backend';

const DOORWAY_ELOHIM_ENDPOINT = '/api/v1/elohim/invoke';
const DECLINED_PRINCIPLE = 'Capability boundaries';

/**
 * Native backend — calls the Rust elohim-agent via doorway HTTP route.
 *
 * This is the production path. The Rust crate handles constitutional
 * reasoning natively with its own LlmBackend (vLLM/Ollama/Anthropic BYOK).
 *
 * Currently a stub — isAvailable() probes the endpoint and returns
 * false until the doorway route exists.
 */
export class NativeBackend implements ElohimBackend {
  readonly id = 'native';
  readonly name = 'Native (Rust Agent)';
  readonly type: ElohimBackendType = 'native';

  /** BYOK API key — forwarded to the Rust backend, never used for direct API calls. */
  byokApiKey: string | null = null;

  async isAvailable(): Promise<boolean> {
    try {
      const response = await fetch(`${DOORWAY_ELOHIM_ENDPOINT}/health`, {
        method: 'GET',
        signal: AbortSignal.timeout(2000),
      });
      return response.ok;
    } catch {
      return false;
    }
  }

  invoke(request: ElohimRequest, elohim: ElohimAgent): Observable<ElohimResponse> {
    return from(this.callDoorway(request, elohim));
  }

  private async callDoorway(request: ElohimRequest, elohim: ElohimAgent): Promise<ElohimResponse> {
    const startTime = Date.now();

    try {
      const response = await fetch(DOORWAY_ELOHIM_ENDPOINT, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          requestId: request.requestId,
          elohimId: elohim.id,
          capability: request.capability,
          params: request.params,
          requesterId: request.requesterId,
          priority: request.priority,
          credentials: this.byokApiKey
            ? { apiKey: this.byokApiKey, backendId: 'anthropic' }
            : undefined,
        }),
      });

      if (!response.ok) {
        const errorBody = await response.text();
        return this.declinedResponse(
          request,
          elohim.id,
          `Doorway error: ${response.status}`,
          `Doorway error ${response.status}: ${errorBody.substring(0, 200)}`
        );
      }

      const data = (await response.json()) as ElohimResponse;
      const timeMs = Date.now() - startTime;

      // The Rust crate returns ElohimResponse directly (camelCase via serde)
      return { ...data, cost: { ...data.cost, timeMs } } as ElohimResponse;
    } catch (err) {
      return this.declinedResponse(
        request,
        elohim.id,
        'Native backend unavailable',
        `Connection error: ${err instanceof Error ? err.message : String(err)}`
      );
    }
  }

  private declinedResponse(
    request: ElohimRequest,
    elohimId: string,
    interpretation: string,
    reason: string
  ): ElohimResponse {
    return {
      requestId: request.requestId,
      elohimId,
      status: 'declined',
      constitutionalReasoning: {
        primaryPrinciple: DECLINED_PRINCIPLE,
        interpretation,
        valuesWeighed: [],
        confidence: 1,
      },
      declineReason: reason,
      respondedAt: new Date().toISOString(),
    };
  }
}
