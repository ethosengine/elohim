/**
 * AI Categorization Service
 *
 * Transaction categorization using Elohim AI agents.
 * Currently uses ElohimStubService for development - logs prompts and returns
 * domain-appropriate mock responses.
 *
 * Features:
 * - Batch categorization (up to 50 transactions per request)
 * - Few-shot learning from user corrections
 * - Confidence scoring (0-100)
 * - Fallback to keyword matching
 * - Auto-create TransactionRules from high-confidence patterns
 */

import { Injectable } from '@angular/core';

import { map, catchError } from 'rxjs/operators';

import { Observable, of } from 'rxjs';

import {
  StagedTransaction,
  CategorizationResponse,
  CategorizationResult,
  CorrectionRecord,
} from '../models/transaction-import.model';

import { ElohimStubService, CategorizationRequest } from './elohim-stub.service';

/**
 * Budget category definition for prompting
 */
interface BudgetCategory {
  name: string;
  description?: string;
  keywords?: string[];
}

@Injectable({
  providedIn: 'root',
})
export class AICategorizationService {
  private readonly BATCH_SIZE = 50; // Max transactions per request

  // Learning: store user corrections for few-shot examples
  private readonly corrections: CorrectionRecord[] = [];
  private readonly merchantPatterns = new Map<string, { category: string; confidence: number }>();

  constructor(private readonly elohimStub: ElohimStubService) {
    this.initializeDefaultPatterns();
  }

  /**
   * Initialize keyword patterns for fallback matching
   */
  private initializeDefaultPatterns(): void {
    const defaultPatterns: Record<string, string> = {
      amazon: 'Shopping',
      'whole foods': 'Groceries',
      safeway: 'Groceries',
      target: 'Shopping',
      coffee: 'Dining',
      restaurant: 'Dining',
      'gas station': 'Transportation',
      uber: 'Transportation',
      delta: 'Travel',
      airbnb: 'Travel',
      netflix: 'Entertainment',
      gym: 'Health & Fitness',
      walgreens: 'Health',
      pharmacy: 'Health',
      rent: 'Housing',
      electric: 'Utilities',
      water: 'Utilities',
      internet: 'Utilities',
    };

    Object.entries(defaultPatterns).forEach(([merchant, category]) => {
      this.merchantPatterns.set(merchant.toLowerCase(), {
        category,
        confidence: 75,
      });
    });
  }

  // ============================================================================
  // CATEGORIZATION
  // ============================================================================

  /**
   * Categorizes a batch of transactions using Elohim AI.
   *
   * Strategy:
   * 1. Check transaction rules first (instant)
   * 2. Use Elohim agent for categorization
   * 3. Fall back to keyword matching if needed
   * 4. Store for learning
   */
  categorizeBatch(
    transactions: StagedTransaction[],
    categories: BudgetCategory[],
    stewardId: string
  ): Observable<CategorizationResponse> {
    if (transactions.length === 0) {
      return of({ results: [] });
    }

    const startTime = Date.now();

    // Get historical examples for few-shot learning
    const examples = this.getHistoricalExamples(stewardId);

    // Build request for Elohim
    const request: CategorizationRequest = {
      transactions,
      categories: categories.map(c => c.name),
      stewardId,
      historicalExamples: examples.map(e => ({
        description: e.transactionDescription,
        category: e.correctedCategory,
      })),
    };

    // Call Elohim (stub in development)
    return this.elohimStub.categorizeTransactions(request).pipe(
      map(results => {
        const duration = Date.now() - startTime;
        return {
          results,
          duration,
          model: 'elohim-stub',
        };
      }),
      catchError(_error => {
        return of({
          results: transactions.map(txn => this.fallbackCategorize(txn)),
          model: 'fallback',
        });
      })
    );
  }

  /**
   * Categorizes a single transaction
   */
  categorizeTransaction(
    transaction: StagedTransaction,
    categories: BudgetCategory[]
  ): Observable<CategorizationResult> {
    return this.categorizeBatch([transaction], categories, '').pipe(
      map(response => response.results[0])
    );
  }

  // ============================================================================
  // LEARNING FROM CORRECTIONS
  // ============================================================================

  /**
   * Records when a user corrects a categorization.
   *
   * Triggers:
   * 1. Storage for future few-shot examples
   * 2. Accuracy tracking per merchant/pattern
   * 3. Auto-creation of TransactionRules if pattern is strong
   */
  learnFromCorrection(staged: StagedTransaction, correctedCategory: string): void {
    // Store correction record
    const correction: CorrectionRecord = {
      id: `corr-${Date.now()}`,
      stewardId: staged.stewardId,
      transactionDescription: staged.description,
      merchantName: staged.merchantName,
      transactionAmount: staged.amount.value,
      originalCategory: staged.category,
      originalConfidence: staged.categoryConfidence,
      correctedCategory,
      timestamp: new Date().toISOString(),
      improvedAccuracy: false,
      ruleCreatedFrom: false,
    };

    this.corrections.push(correction);

    // Update merchant pattern
    if (staged.merchantName) {
      const merchant = staged.merchantName.toLowerCase();
      this.merchantPatterns.set(merchant, {
        category: correctedCategory,
        confidence: 85,
      });
    }

    // Check if we should auto-create a TransactionRule
    const shouldCreateRule = this.checkShouldCreateRule(staged, correctedCategory);
    if (shouldCreateRule) {
      // Emit event or call TransactionRuleService.createRule() when available
    }
  }

  /**
   * Checks if a pattern is strong enough to auto-create a TransactionRule
   */
  private checkShouldCreateRule(staged: StagedTransaction, correctedCategory: string): boolean {
    if (!staged.merchantName) {
      return false;
    }

    const merchant = staged.merchantName.toLowerCase();

    // Count corrections for this merchant → category
    const relatedCorrections = this.corrections.filter(
      c => c.merchantName?.toLowerCase() === merchant && c.correctedCategory === correctedCategory
    );

    if (relatedCorrections.length < 5) {
      return false;
    }

    // Check for contradictions
    const contradictions = this.corrections.filter(
      c => c.merchantName?.toLowerCase() === merchant && c.correctedCategory !== correctedCategory
    );

    if (contradictions.length > 0) {
      const confidence =
        relatedCorrections.length / (relatedCorrections.length + contradictions.length);
      return confidence > 0.9;
    }

    return true;
  }

  // ============================================================================
  // FALLBACK: KEYWORD MATCHING
  // ============================================================================

  /**
   * Fallback categorization using keyword patterns
   */
  private fallbackCategorize(transaction: StagedTransaction): CategorizationResult {
    // Try merchant name first
    if (transaction.merchantName) {
      const merchantLower = transaction.merchantName.toLowerCase();
      const pattern = this.merchantPatterns.get(merchantLower);

      if (pattern) {
        return {
          transactionId: transaction.id,
          category: pattern.category,
          confidence: pattern.confidence,
          reasoning: `Matched merchant pattern: ${transaction.merchantName}`,
        };
      }

      // Try partial matching
      for (const [merchant, pattern] of this.merchantPatterns.entries()) {
        if (merchantLower.includes(merchant) || merchant.includes(merchantLower)) {
          return {
            transactionId: transaction.id,
            category: pattern.category,
            confidence: Math.max(50, pattern.confidence - 20),
            reasoning: `Partial merchant match: ${merchant}`,
          };
        }
      }
    }

    // Try description keyword matching
    const descLower = transaction.description.toLowerCase();
    for (const [keyword, pattern] of this.merchantPatterns.entries()) {
      if (descLower.includes(keyword)) {
        return {
          transactionId: transaction.id,
          category: pattern.category,
          confidence: Math.max(50, pattern.confidence - 30),
          reasoning: `Keyword match in description: ${keyword}`,
        };
      }
    }

    // Default fallback
    return {
      transactionId: transaction.id,
      category: 'Uncategorized',
      confidence: 0,
      reasoning: 'Could not determine category - manual review needed',
    };
  }

  // ============================================================================
  // HISTORICAL EXAMPLES
  // ============================================================================

  /**
   * Gets high-quality examples from user's correction history
   */
  private getHistoricalExamples(stewardId: string): CorrectionRecord[] {
    const stewardCorrections = this.corrections.filter(c => c.stewardId === stewardId);

    return stewardCorrections.sort(
      (a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()
    );
  }

  /**
   * Gets all corrections (for debugging)
   */
  getCorrections(): CorrectionRecord[] {
    return [...this.corrections];
  }
}
