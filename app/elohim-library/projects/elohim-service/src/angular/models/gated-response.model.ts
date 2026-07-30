import type { GateEvaluationView } from '@elohim/storage-client';

/** Response envelope for gated mutations. */
export interface GatedResponse<T> {
  data: T;
  gate: GateEvaluationView;
}

/** Type guard: does this response include gate evaluation? */
export function isGatedResponse(response: unknown): response is GatedResponse<unknown> {
  return (
    response !== null &&
    response !== undefined &&
    typeof response === 'object' &&
    'gate' in response &&
    (response as Record<string, unknown>)['gate'] !== null &&
    (response as Record<string, unknown>)['gate'] !== undefined
  );
}

/**
 * Extract gate evaluation from a response, or null if not present.
 * Intentional `T | null` API; sonarjs/function-return-type misfires on any nullable
 * union in this toolchain (verified: even a non-divergent `T | null` return triggers it).
 */
// eslint-disable-next-line sonarjs/function-return-type -- see JSDoc above
export function extractGateFromResponse(response: unknown): GateEvaluationView | null {
  if (isGatedResponse(response)) {
    return response.gate;
  }
  return null;
}
