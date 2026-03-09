/**
 * Request/Response types for the Elohim Agent SDK sidecar.
 *
 * These interfaces match the wire format between the Angular frontend
 * (via doorway proxy) and the sidecar service.
 */

// ============================================================================
// Request Types
// ============================================================================

/**
 * InvokeRequest - The POST body doorway forwards to the sidecar.
 *
 * Sent by NativeBackend in elohim-app when invoking an Elohim capability.
 */
export interface InvokeRequest {
  /** Unique ID for correlating request/response */
  requestId: string;

  /** Which Elohim agent to invoke */
  elohimId: string;

  /** Capability to execute (kebab-case ElohimCapability) */
  capability: string;

  /** Capability-specific parameters */
  params: Record<string, unknown>;

  /** Agent ID of the requester */
  requesterId: string;

  /** Request priority level */
  priority: string;

  /** Optional BYOK credentials for direct API access */
  credentials?: {
    apiKey: string;
    backendId: string;
  };
}

// ============================================================================
// Response Types
// ============================================================================

/**
 * ElohimResponse - Response from an Elohim to a request.
 *
 * Matches the shape expected by elohim-app's ElohimResponse interface.
 */
export interface ElohimResponse {
  /** The request this responds to */
  requestId: string;

  /** Which Elohim responded */
  elohimId: string;

  /** Status of the response */
  status: 'fulfilled' | 'declined' | 'deferred' | 'escalated';

  /** Constitutional reasoning for the response */
  constitutionalReasoning: ConstitutionalReasoning;

  /** The actual response payload (capability-specific) */
  payload?: unknown;

  /** If declined: why */
  declineReason?: string;

  /** If escalated: to which Elohim/layer */
  escalatedTo?: string;

  /** When response was generated (ISO 8601) */
  respondedAt: string;

  /** Computational cost of the response */
  cost?: ElohimComputationCost;
}

/**
 * ConstitutionalReasoning - How the Elohim justified its response.
 *
 * Every Elohim decision must trace back to constitutional principles.
 * This is the audit trail.
 */
export interface ConstitutionalReasoning {
  /** Primary constitutional principle applied */
  primaryPrinciple: string;

  /** How the principle was interpreted */
  interpretation: string;

  /** Values that were weighed */
  valuesWeighed: {
    value: string;
    weight: number;
    direction: 'for' | 'against';
  }[];

  /** Confidence in the decision (0.0 - 1.0) */
  confidence: number;

  /** Precedents referenced */
  precedents?: string[];

  /** If this creates new precedent */
  newPrecedent?: boolean;
}

/**
 * ElohimComputationCost - Resource usage for transparency.
 */
export interface ElohimComputationCost {
  /** Total tokens processed (input + output) */
  tokensProcessed: number;

  /** Wall-clock time in milliseconds */
  timeMs: number;

  /** Number of constitutional checks performed */
  constitutionalChecks: number;

  /** Number of precedent lookups performed */
  precedentLookups: number;
}

// ============================================================================
// Health Check
// ============================================================================

/**
 * HealthResponse - Response from the /health endpoint.
 */
export interface HealthResponse {
  /** Service status */
  status: 'ok';

  /** Remaining budget in the current period */
  budgetRemaining: number;

  /** LLM model in use */
  model: string;
}
