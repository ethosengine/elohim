/**
 * Elohim Stub Service
 *
 * Development/testing stub for AI agent calls. Logs all prompts and decisions
 * that would be made by Elohim agents, returning domain-appropriate mock responses.
 *
 * Purpose:
 * - Enable development without API keys or network calls
 * - Log prompts for review and debugging
 * - Return realistic stub responses based on domain models
 * - Make AI integration points explicit and auditable
 *
 * Usage:
 * Replace actual AI calls with ElohimStubService methods during development.
 * When ready for production, swap in the real implementation.
 */

import { Injectable } from '@angular/core';

// @coverage: 97.9% (2026-02-05)

import { Observable, of, delay } from 'rxjs';

import {
  StagedTransaction,
  CategorySuggestion,
  CategorizationResult,
} from '../models/transaction-import.model';

/**
 * Logged Elohim call for debugging and prompt review
 */
export interface ElohimCallLog {
  id: string;
  timestamp: string;
  agentType: 'categorizer' | 'adjuster' | 'reviewer' | 'synthesizer';
  prompt: string;
  context: Record<string, unknown>;
  response: unknown;
  latencyMs: number;
}

/**
 * Categorization request for the stub
 */
export interface CategorizationRequest {
  transactions: StagedTransaction[];
  categories: string[];
  stewardId: string;
  historicalExamples?: { description: string; category: string }[];
}

/**
 * Adjudication request for insurance claims
 */
export interface AdjudicationRequest {
  claimId: string;
  claimType: string;
  amount: number;
  evidence: string[];
  memberHistory: {
    claimsCount: number;
    riskScore: number;
    memberSince: string;
  };
}

/**
 * Adjudication response from the stub
 */
export interface AdjudicationResponse {
  decision: 'approve' | 'deny' | 'review';
  reasoning: string;
  confidence: number;
  suggestedPayout?: number;
  flagsForReview?: string[];
}

@Injectable({
  providedIn: 'root',
})
export class ElohimStubService {
  private callLogs: ElohimCallLog[] = [];
  private readonly SIMULATED_LATENCY_MS = 150; // Simulate AI response time

  // Default keyword → category mappings for stub categorization
  private readonly DEFAULT_PATTERNS: Record<string, string> = {
    amazon: 'Shopping',
    'whole foods': 'Groceries',
    safeway: 'Groceries',
    kroger: 'Groceries',
    target: 'Shopping',
    walmart: 'Shopping',
    starbucks: 'Dining',
    coffee: 'Dining',
    restaurant: 'Dining',
    uber: 'Transportation',
    lyft: 'Transportation',
    gas: 'Transportation',
    shell: 'Transportation',
    chevron: 'Transportation',
    netflix: 'Entertainment',
    spotify: 'Entertainment',
    hulu: 'Entertainment',
    gym: 'Health & Fitness',
    pharmacy: 'Health',
    cvs: 'Health',
    walgreens: 'Health',
    doctor: 'Health',
    rent: 'Housing',
    mortgage: 'Housing',
    electric: 'Utilities',
    water: 'Utilities',
    internet: 'Utilities',
    comcast: 'Utilities',
    'at&t': 'Utilities',
    verizon: 'Utilities',
  };

  constructor() {
    // Stub service initialized - AI calls will be logged and stubbed
  }

  // ============================================================================
  // TRANSACTION CATEGORIZATION
  // ============================================================================

  /**
   * Stub for AI-powered transaction categorization.
   * Logs the prompt and returns keyword-based categorization.
   */
  categorizeTransactions(request: CategorizationRequest): Observable<CategorizationResult[]> {
    // Build the prompt that would be sent to the AI
    const prompt = this.buildCategorizationPrompt(request);

    // Log the call
    const results = request.transactions.map(txn =>
      this.stubCategorizeTransaction(txn, request.categories)
    );

    const callLog: ElohimCallLog = {
      id: `elohim-${Date.now()}`,
      timestamp: new Date().toISOString(),
      agentType: 'categorizer',
      prompt,
      context: {
        transactionCount: request.transactions.length,
        categories: request.categories,
        stewardId: request.stewardId,
        hasHistoricalExamples: (request.historicalExamples?.length ?? 0) > 0,
      },
      response: results,
      latencyMs: this.SIMULATED_LATENCY_MS,
    };

    this.logCall(callLog);

    // Return with simulated latency
    return of(results).pipe(delay(this.SIMULATED_LATENCY_MS));
  }

  /**
   * Builds the prompt that would be sent to the AI for categorization
   */
  private buildCategorizationPrompt(request: CategorizationRequest): string {
    const categoryList = request.categories.join(', ');
    const transactionSummary = request.transactions
      .slice(0, 5)
      .map(t => `"${t.description}" ($${t.amount.value})`)
      .join('; ');

    return `[ELOHIM CATEGORIZER PROMPT]
Task: Categorize ${request.transactions.length} bank transactions

Available Categories: ${categoryList}

Sample Transactions: ${transactionSummary}${request.transactions.length > 5 ? ` ... and ${request.transactions.length - 5} more` : ''}

${request.historicalExamples?.length ? `Historical Examples (${request.historicalExamples.length}): User previously categorized similar transactions` : 'No historical examples available'}

Instructions: For each transaction, determine the most appropriate category based on merchant name, description, and amount. Provide confidence score (0-100) and brief reasoning.`;
  }

  /**
   * Stub categorization for a single transaction using keyword matching
   */
  private stubCategorizeTransaction(
    txn: StagedTransaction,
    availableCategories: string[]
  ): CategorizationResult {
    const searchText = `${txn.description} ${txn.merchantName ?? ''}`.toLowerCase();

    // Try to match against patterns
    for (const [pattern, category] of Object.entries(this.DEFAULT_PATTERNS)) {
      if (searchText.includes(pattern)) {
        // Only use the category if it's in the available list
        const matchedCategory =
          availableCategories.find(c => c.toLowerCase() === category.toLowerCase()) ?? category;

        return {
          transactionId: txn.id,
          category: matchedCategory,
          confidence:
            75 + Math.floor((crypto.getRandomValues(new Uint32Array(1))[0] / 2 ** 32) * 20), // 75-95
          reasoning: `[STUB] Matched pattern "${pattern}" in transaction`,
          alternatives: this.generateAlternatives(availableCategories, matchedCategory),
        };
      }
    }

    // No pattern match - return uncategorized with low confidence
    return {
      transactionId: txn.id,
      category: availableCategories.includes('Uncategorized')
        ? 'Uncategorized'
        : availableCategories[0],
      confidence: 25 + Math.floor((crypto.getRandomValues(new Uint32Array(1))[0] / 2 ** 32) * 25), // 25-50
      reasoning: '[STUB] No pattern match - manual review recommended',
      alternatives: this.generateAlternatives(availableCategories, ''),
    };
  }

  /**
   * Generate alternative category suggestions
   */
  private generateAlternatives(
    categories: string[],
    primaryCategory: string
  ): CategorySuggestion[] {
    return categories
      .filter(c => c !== primaryCategory)
      .slice(0, 2)
      .map(category => ({
        category,
        confidence: 10 + Math.floor((crypto.getRandomValues(new Uint32Array(1))[0] / 2 ** 32) * 30),
        reasoning: '[STUB] Alternative suggestion',
        source: 'ai' as const,
      }));
  }

  // ============================================================================
  // INSURANCE CLAIM ADJUDICATION
  // ============================================================================

  /**
   * Stub for AI-powered claim adjudication.
   * Logs the prompt and returns rule-based decision.
   */
  adjudicateClaim(request: AdjudicationRequest): Observable<AdjudicationResponse> {
    // Build the prompt that would be sent to the AI
    const prompt = this.buildAdjudicationPrompt(request);

    // Make stub decision based on simple rules
    const response = this.stubAdjudicateClaim(request);

    const callLog: ElohimCallLog = {
      id: `elohim-${Date.now()}`,
      timestamp: new Date().toISOString(),
      agentType: 'adjuster',
      prompt,
      context: {
        claimId: request.claimId,
        claimType: request.claimType,
        amount: request.amount,
        evidenceCount: request.evidence.length,
        memberRiskScore: request.memberHistory.riskScore,
      },
      response,
      latencyMs: this.SIMULATED_LATENCY_MS,
    };

    this.logCall(callLog);

    return of(response).pipe(delay(this.SIMULATED_LATENCY_MS));
  }

  /**
   * Builds the prompt that would be sent to the AI for adjudication
   */
  private buildAdjudicationPrompt(request: AdjudicationRequest): string {
    return `[ELOHIM ADJUSTER PROMPT]
Task: Adjudicate insurance claim

Claim ID: ${request.claimId}
Claim Type: ${request.claimType}
Amount Requested: $${request.amount}

Member History:
- Claims Count: ${request.memberHistory.claimsCount}
- Risk Score: ${request.memberHistory.riskScore}/100
- Member Since: ${request.memberHistory.memberSince}

Evidence Provided: ${request.evidence.length} items
${request.evidence.map((e, i) => `${i + 1}. ${e}`).join('\n')}

Instructions:
1. Evaluate the claim based on evidence and member history
2. Determine if claim should be approved, denied, or flagged for review
3. Provide clear reasoning citing constitutional basis
4. Apply generosity principle for ambiguous cases
5. Flag any concerns for governance review`;
  }

  /**
   * Stub adjudication using simple rules
   */
  private stubAdjudicateClaim(request: AdjudicationRequest): AdjudicationResponse {
    // Simple rule-based decision for stub
    const riskScore = request.memberHistory.riskScore;
    const hasEvidence = request.evidence.length > 0;
    const isLowAmount = request.amount < 1000;
    const isNewMember = request.memberHistory.claimsCount === 0;

    // High risk or no evidence → review
    if (riskScore > 80 || !hasEvidence) {
      return {
        decision: 'review',
        reasoning:
          '[STUB] Flagged for review due to ' +
          (riskScore > 80 ? 'high risk score' : 'insufficient evidence'),
        confidence: 60,
        flagsForReview: riskScore > 80 ? ['High risk member'] : ['Insufficient evidence'],
      };
    }

    // Low amount with evidence → approve
    if (isLowAmount && hasEvidence) {
      return {
        decision: 'approve',
        reasoning:
          '[STUB] Low amount claim with supporting evidence - approved per generosity principle',
        confidence: 85,
        suggestedPayout: request.amount,
      };
    }

    // New member with moderate claim → review
    if (isNewMember && request.amount > 5000) {
      return {
        decision: 'review',
        reasoning: '[STUB] New member with significant claim - manual review recommended',
        confidence: 70,
        flagsForReview: ['New member', 'Significant amount'],
      };
    }

    // Default: approve with standard confidence
    return {
      decision: 'approve',
      reasoning: '[STUB] Claim meets standard criteria - approved',
      confidence: 75,
      suggestedPayout: request.amount,
    };
  }

  // ============================================================================
  // LOGGING & DEBUGGING
  // ============================================================================

  /**
   * Logs an Elohim call for debugging
   */
  private logCall(callLog: ElohimCallLog): void {
    this.callLogs.push(callLog);
  }

  /**
   * Gets all logged Elohim calls for debugging
   */
  getCallLogs(): ElohimCallLog[] {
    return [...this.callLogs];
  }

  /**
   * Gets calls filtered by agent type
   */
  getCallsByAgent(agentType: ElohimCallLog['agentType']): ElohimCallLog[] {
    return this.callLogs.filter(log => log.agentType === agentType);
  }

  /**
   * Clears all logged calls
   */
  clearLogs(): void {
    this.callLogs = [];
  }

  /**
   * Exports logs as JSON for analysis
   */
  exportLogs(): string {
    return JSON.stringify(this.callLogs, null, 2);
  }
}
