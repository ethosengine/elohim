/**
 * Invoke handler with budget enforcement for the Elohim Agent SDK sidecar.
 *
 * BudgetEnforcer provides an in-memory call counter that prevents runaway
 * API spend in demo/dev contexts. handleInvoke orchestrates the full
 * request lifecycle: budget check -> prompt assembly -> Claude API call ->
 * response parsing.
 */

import Anthropic from '@anthropic-ai/sdk';
import { buildSystemPrompt, buildUserPrompt } from './constitutional.js';
import type { InvokeRequest, ElohimResponse } from './types.js';

// ============================================================================
// BudgetEnforcer
// ============================================================================

/**
 * In-memory call counter with a configurable limit.
 *
 * Guards against runaway API spend by declining requests once the budget
 * is exhausted. Intended for demo/dev use — production would use a
 * persistent store with per-agent quotas.
 */
export class BudgetEnforcer {
  private count: number;

  constructor(private readonly limit: number = 10) {
    this.count = limit;
  }

  /**
   * Attempt to consume one unit of budget.
   * @returns true if budget was available and consumed, false if exhausted.
   */
  consume(): boolean {
    if (this.count <= 0) {
      return false;
    }
    this.count--;
    return true;
  }

  /**
   * @returns the number of remaining calls in this budget period.
   */
  remaining(): number {
    return this.count;
  }
}

// ============================================================================
// handleInvoke
// ============================================================================

/**
 * Process an InvokeRequest through the constitutional pipeline.
 *
 * 1. Check budget — decline immediately if exhausted
 * 2. Build system + user prompts from constitutional framework
 * 3. Call Claude Haiku via Anthropic SDK
 * 4. Parse structured JSON from Claude's response
 * 5. Return ElohimResponse with reasoning, payload, and cost
 */
export async function handleInvoke(
  request: InvokeRequest,
  sdk: Anthropic,
  budget: BudgetEnforcer,
  model: string = 'claude-haiku-4-5-20251001',
): Promise<ElohimResponse> {
  // 1. Check budget
  if (!budget.consume()) {
    return {
      requestId: request.requestId,
      elohimId: request.elohimId || 'elohim-sdk',
      status: 'declined',
      constitutionalReasoning: {
        primaryPrinciple: 'Resource stewardship',
        interpretation: 'Budget exhausted — demo limit reached',
        valuesWeighed: [],
        confidence: 1,
      },
      declineReason: 'Budget exhausted',
      respondedAt: new Date().toISOString(),
    };
  }

  const startTime = Date.now();

  // 2. Build prompts
  const systemPrompt = buildSystemPrompt();
  const userPrompt = buildUserPrompt(request.capability, request.params);

  // 3. Call Claude
  const completion = await sdk.messages.create({
    model,
    max_tokens: 2048,
    system: systemPrompt,
    messages: [{ role: 'user', content: userPrompt }],
  });

  const timeMs = Date.now() - startTime;

  // 4. Extract text from response
  const text = completion.content
    .filter((block): block is Anthropic.TextBlock => block.type === 'text')
    .map((block) => block.text)
    .join('');

  // 5. Try to parse Claude's JSON response
  let reasoning = {
    primaryPrinciple: 'Capability fulfillment',
    interpretation: `Processed ${request.capability} request following constitutional guidelines`,
    valuesWeighed: [] as { value: string; weight: number; direction: 'for' | 'against' }[],
    confidence: 0.85,
  };
  let payload: unknown = undefined;

  try {
    // Claude may wrap JSON in ```json blocks — strip them
    const jsonStr = text
      .replace(/```json\n?/g, '')
      .replace(/```\n?/g, '')
      .trim();
    const parsed = JSON.parse(jsonStr);
    if (parsed.constitutionalReasoning) {
      reasoning = { ...reasoning, ...parsed.constitutionalReasoning };
    }
    if (parsed.payload !== undefined) {
      payload = parsed.payload;
    }
  } catch {
    // If Claude didn't return parseable JSON, wrap raw text as payload
    payload = { rawResponse: text };
  }

  // 6. Build response
  return {
    requestId: request.requestId,
    elohimId: request.elohimId || 'elohim-sdk',
    status: 'fulfilled',
    constitutionalReasoning: reasoning,
    payload,
    respondedAt: new Date().toISOString(),
    cost: {
      tokensProcessed:
        (completion.usage?.input_tokens ?? 0) +
        (completion.usage?.output_tokens ?? 0),
      timeMs,
      constitutionalChecks: 1,
      precedentLookups: 0,
    },
  };
}
