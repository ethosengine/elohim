import type { GateEvaluationView } from '@elohim/storage-client';

/** Response envelope for gated mutations. */
export interface GatedResponse<T> {
  data: T;
  gate: GateEvaluationView;
}

/** Type guard: does this response include gate evaluation? */
export function isGatedResponse(response: unknown): response is GatedResponse<unknown> {
  return (
    response != null &&
    typeof response === 'object' &&
    'gate' in response &&
    (response as Record<string, unknown>).gate != null
  );
}

/** Extract gate evaluation from a response, or null if not present. */
export function extractGateFromResponse(response: unknown): GateEvaluationView | null {
  if (isGatedResponse(response)) {
    return response.gate;
  }
  return null;
}
