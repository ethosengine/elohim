/**
 * Fastify HTTP server for the Elohim Agent SDK sidecar.
 *
 * Entry point that exposes:
 *   GET  /health  — liveness check with budget + model info
 *   POST /invoke  — constitutional AI request pipeline
 *
 * Configuration via environment variables:
 *   ANTHROPIC_API_KEY   (required)
 *   ELOHIM_AGENT_PORT   (default: 8095)
 *   ELOHIM_BUDGET_LIMIT (default: 10)
 */

import Fastify from 'fastify';
import Anthropic from '@anthropic-ai/sdk';
import { BudgetEnforcer, handleInvoke } from './invoke.js';
import type { InvokeRequest } from './types.js';

// ============================================================================
// Configuration
// ============================================================================

const PORT = parseInt(process.env.ELOHIM_AGENT_PORT || '8095', 10);
const API_KEY = process.env.ANTHROPIC_API_KEY;
const BUDGET_LIMIT = parseInt(process.env.ELOHIM_BUDGET_LIMIT || '10', 10);
const MODEL = 'claude-haiku-4-5-20251001';

if (!API_KEY) {
  console.error('ANTHROPIC_API_KEY environment variable is required');
  process.exit(1);
}

// ============================================================================
// Server Setup
// ============================================================================

const app = Fastify({ logger: true });
const sdk = new Anthropic({ apiKey: API_KEY });
const budget = new BudgetEnforcer(BUDGET_LIMIT);

// ============================================================================
// Routes
// ============================================================================

app.get('/health', async () => ({
  status: 'ok' as const,
  budgetRemaining: budget.remaining(),
  model: MODEL,
}));

app.post<{ Body: InvokeRequest }>('/invoke', async (request, reply) => {
  try {
    const result = await handleInvoke(request.body, sdk, budget, MODEL);
    reply.header('X-Elohim-Budget-Remaining', budget.remaining().toString());
    if (result.status === 'declined' && result.declineReason?.includes('Budget exhausted')) {
      reply.status(429);
    }
    return result;
  } catch (err) {
    request.log.error({ err: err instanceof Error ? err.message : String(err) }, 'Invoke handler error');
    reply.header('X-Elohim-Budget-Remaining', budget.remaining().toString());
    reply.status(500);
    return {
      requestId: request.body.requestId || 'unknown',
      elohimId: 'elohim-sdk',
      status: 'declined' as const,
      constitutionalReasoning: {
        primaryPrinciple: 'Service continuity',
        interpretation: 'Internal error during processing',
        valuesWeighed: [] as { value: string; weight: number; direction: 'for' | 'against' }[],
        confidence: 1,
      },
      declineReason: 'Internal processing error',
      respondedAt: new Date().toISOString(),
    };
  }
});

// ============================================================================
// Start
// ============================================================================

app.listen({ port: PORT, host: '0.0.0.0' }).then(() => {
  console.log(`Elohim Agent SDK sidecar listening on port ${PORT}`);
  console.log(`Model: ${MODEL}`);
  console.log(`Budget limit: ${BUDGET_LIMIT} calls`);
});
