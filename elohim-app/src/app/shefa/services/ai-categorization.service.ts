/**
 * AI Categorization Service
 *
 * Anthropic Claude API integration for automated transaction categorization.
 *
 * Features:
 * - Batch categorization (up to 50 transactions per request)
 * - Few-shot learning from user corrections
 * - Confidence scoring (0-100)
 * - Fallback to keyword matching if API fails
 * - Auto-create TransactionRules from high-confidence patterns
 *
 * Prompt Engineering:
 * - Temperature 0.3 (lower for consistency over creativity)
 * - Few-shot examples from user's history
 * - Merchant + amount + description context
 */

import { Injectable } from '@angular/core';
import { HttpClient, HttpHeaders } from '@angular/common/http';
import { Observable, throwError } from 'rxjs';
import { catchError, timeout } from 'rxjs/operators';
import { environment } from '@elohim/environments/environment';

import {
  StagedTransaction,
  CategorySuggestion,
  CategorizationResponse,
  CategorizationResult,
  CorrectionRecord,
} from '@elohim/models/transaction-import.model';

/**
 * Anthropic API request structure
 */
interface AnthropicMessage {
  role: 'user' | 'assistant';
  content: string;
}

interface AnthropicRequest {
  model: string;
  max_tokens: number;
  temperature: number;
  messages: AnthropicMessage[];
}

interface AnthropicResponse {
  id: string;
  type: string;
  role: string;
  content: Array<{ type: string; text: string }>;
  model: string;
  stop_reason: string;
  usage: {
    input_tokens: number;
    output_tokens: number;
  };
}

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
  private readonly ANTHROPIC_API_BASE = 'https://api.anthropic.com/v1';
  private readonly MODEL = environment.ai?.model || 'claude-3-5-sonnet-20241022';
  private readonly BATCH_SIZE = 50; // Max transactions per request
  private readonly MAX_RETRIES = 3;

  // Learning: store user corrections for few-shot examples
  private corrections: CorrectionRecord[] = [];
  private merchantPatterns: Map<string, { category: string; confidence: number }> =
    new Map();

  constructor(private http: HttpClient) {
    this.validateEnvironmentConfig();
    this.initializeDefaultPatterns();
  }

  /**
   * Validate required environment configuration
   */
  private validateEnvironmentConfig(): void {
    if (!environment.ai?.apiKey) {
      console.error('[AICategories] Missing ANTHROPIC_API_KEY in environment');
    }
    if (!this.MODEL) {
      console.error('[AICategories] Missing AI model configuration');
    }
  }

  /**
   * Initialize keyword patterns for fallback matching
   */
  private initializeDefaultPatterns(): void {
    // Default merchant patterns (learned over time)
    const defaultPatterns: Record<string, string> = {
      'amazon': 'Shopping',
      'whole foods': 'Groceries',
      'safeway': 'Groceries',
      'target': 'Shopping',
      'coffee': 'Dining',
      'restaurant': 'Dining',
      'gas station': 'Transportation',
      'uber': 'Transportation',
      'delta': 'Travel',
      'airbnb': 'Travel',
      'netflix': 'Entertainment',
      'gym': 'Health & Fitness',
      'walgreens': 'Health',
      'pharmacy': 'Health',
      'rent': 'Housing',
      'electric': 'Utilities',
      'water': 'Utilities',
      'internet': 'Utilities',
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
   * Categorizes a batch of transactions using AI.
   *
   * Strategy:
   * 1. Check transaction rules first (instant)
   * 2. Use AI API for remaining transactions
   * 3. Fall back to keyword matching if API fails
   * 4. Store for learning
   */
  async categorizeBatch(
    transactions: StagedTransaction[],
    categories: BudgetCategory[],
    stewardId: string
  ): Promise<CategorizationResponse> {
    if (transactions.length === 0) {
      return { results: [] };
    }

    const startTime = Date.now();

    try {
      // Get historical examples for this steward
      const examples = this.getHistoricalExamples(stewardId);

      // Build prompt for AI
      const prompt = this.buildCategorizePrompt(
        transactions,
        categories,
        examples
      );

      // Call Claude API
      const response = await this.callAnthropicAPI({
        model: this.MODEL,
        max_tokens: 4096,
        temperature: 0.3, // Lower for consistent categorization
        messages: [
          {
            role: 'user',
            content: prompt,
          },
        ],
      }).toPromise();

      // Parse response
      const aiResults = this.parseCategorizationResponse(response, transactions);

      const duration = Date.now() - startTime;
      console.log(
        `[AICategories] Categorized ${transactions.length} transactions in ${duration}ms`
      );

      return {
        results: aiResults,
        duration,
        model: this.MODEL,
      };
    } catch (error) {
      console.error('[AICategories] API call failed, using fallback', error);
      return {
        results: transactions.map(txn => this.fallbackCategorize(txn)),
        model: 'fallback',
      };
    }
  }

  /**
   * Categorizes a single transaction
   */
  async categorizeTransaction(
    transaction: StagedTransaction,
    categories: BudgetCategory[]
  ): Promise<CategorizationResult> {
    const response = await this.categorizeBatch([transaction], categories, '');
    return response.results[0];
  }

  // ============================================================================
  // LEARNING FROM CORRECTIONS
  // ============================================================================

  /**
   * Records when a user corrects an AI categorization.
   *
   * Triggers:
   * 1. Storage for future few-shot examples
   * 2. Accuracy tracking per merchant/pattern
   * 3. Auto-creation of TransactionRules if pattern is strong
   */
  async learnFromCorrection(
    staged: StagedTransaction,
    correctedCategory: string
  ): Promise<void> {
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
      improvedAccuracy: false, // Will be determined over time
      ruleCreatedFrom: false,
    };

    this.corrections.push(correction);

    // Update merchant pattern
    if (staged.merchantName) {
      const merchant = staged.merchantName.toLowerCase();
      this.merchantPatterns.set(merchant, {
        category: correctedCategory,
        confidence: 85, // Increased from user correction
      });
    }

    // Check if we should auto-create a TransactionRule
    const shouldCreateRule = await this.checkShouldCreateRule(
      staged,
      correctedCategory
    );
    if (shouldCreateRule) {
      console.log(
        `[AICategories] Would create TransactionRule for ${staged.merchantName} → ${correctedCategory}`
      );
      // TODO: Emit event or call TransactionRuleService.createRule()
    }

    console.log(
      `[AICategories] Learned correction: ${staged.merchantName} → ${correctedCategory}`
    );
  }

  /**
   * Checks if a pattern is strong enough to auto-create a TransactionRule
   *
   * Conditions:
   * - 5+ corrections to same category for same merchant
   * - Confidence > 90%
   * - No contradictory corrections
   */
  private async checkShouldCreateRule(
    staged: StagedTransaction,
    correctedCategory: string
  ): Promise<boolean> {
    if (!staged.merchantName) {
      return false;
    }

    const merchant = staged.merchantName.toLowerCase();

    // Count corrections for this merchant → category
    const relatedCorrections = this.corrections.filter(
      c =>
        c.merchantName?.toLowerCase() === merchant &&
        c.correctedCategory === correctedCategory
    );

    if (relatedCorrections.length < 5) {
      return false;
    }

    // Check for contradictions (other corrections to different categories)
    const contradictions = this.corrections.filter(
      c =>
        c.merchantName?.toLowerCase() === merchant &&
        c.correctedCategory !== correctedCategory
    );

    // If contradictions exist, only create rule if our category wins 90%+
    if (contradictions.length > 0) {
      const confidence =
        relatedCorrections.length /
        (relatedCorrections.length + contradictions.length);
      return confidence > 0.9;
    }

    return true;
  }

  // ============================================================================
  // PROMPT ENGINEERING
  // ============================================================================

  /**
   * Builds the categorization prompt for Claude API
   *
   * Structure:
   * 1. System instructions
   * 2. Available categories with descriptions
   * 3. Few-shot examples from user's history
   * 4. Transactions to categorize
   * 5. Output format specification
   */
  private buildCategorizePrompt(
    transactions: StagedTransaction[],
    categories: BudgetCategory[],
    examples: CorrectionRecord[]
  ): string {
    const categoryList = categories
      .map(c => `- ${c.name}: ${c.description || ''}`)
      .join('\n');

    const examplesList = examples
      .slice(0, 10) // Use top 10 examples
      .map(
        e =>
          `"${e.transactionDescription}" (${e.transactionAmount || 'unknown amount'}) [${e.merchantName || 'unknown merchant'}] → ${e.correctedCategory}`
      )
      .join('\n');

    const transactionsList = transactions
      .map(
        (t, i) =>
          `${i + 1}. "${t.description}" - Amount: ${t.amount.value} ${t.amount.unit} - Merchant: ${t.merchantName || 'unknown'}`
      )
      .join('\n');

    return `You are a financial transaction categorization assistant for the Elohim Protocol, a decentralized learning network.

Your task is to categorize bank transactions into budget categories with high confidence.

AVAILABLE BUDGET CATEGORIES:
${categoryList}

HISTORICAL EXAMPLES (learn from this user's past corrections):
${examplesList || '(No examples yet - use best judgment)'}

TRANSACTIONS TO CATEGORIZE:
${transactionsList}

RESPONSE FORMAT:
Return a JSON array with exactly ${transactions.length} objects. Each object must have:
- transactionId: the transaction index (1-${transactions.length})
- category: the category name (must be from the list above)
- confidence: 0-100 (higher = more certain)
- reasoning: brief explanation (1-2 sentences)
- alternatives: array of up to 2 alternative suggestions (each with category, confidence, and reasoning)

IMPORTANT RULES:
1. Category must be from the provided list - NEVER invent new categories
2. Confidence reflects your certainty (>80% = high, 50-80% = medium, <50% = uncertain)
3. For ambiguous transactions, provide lower confidence scores
4. Consider merchant name heavily - it's often the strongest signal
5. Similar amounts/descriptions suggest repeat merchants
6. Always explain your reasoning

Return ONLY the JSON array, no other text. Example format:
[
  {
    "transactionId": 1,
    "category": "Groceries",
    "confidence": 95,
    "reasoning": "Safeway is a grocery store chain",
    "alternatives": [
      { "category": "Shopping", "confidence": 5, "reasoning": "Could be general shopping" }
    ]
  }
]`;
  }

  /**
   * Parses categorization response from Claude API
   */
  private parseCategorizationResponse(
    response: AnthropicResponse,
    transactions: StagedTransaction[]
  ): CategorizationResult[] {
    try {
      const textContent = response.content.find(c => c.type === 'text');
      if (!textContent || !textContent.text) {
        throw new Error('No text content in response');
      }

      // Extract JSON from response (handle markdown code blocks)
      let jsonText = textContent.text;
      const jsonMatch = jsonText.match(/\[[\s\S]*\]/);
      if (jsonMatch) {
        jsonText = jsonMatch[0];
      }

      const results = JSON.parse(jsonText);

      if (!Array.isArray(results)) {
        throw new Error('Response is not an array');
      }

      // Map to CategorizationResult format and validate
      return results.map((result, index) => ({
        transactionId: transactions[index]?.id || `unknown-${index}`,
        category: result.category || 'Uncategorized',
        confidence: Math.min(100, Math.max(0, result.confidence || 50)),
        reasoning: result.reasoning || '',
        alternatives: result.alternatives || [],
      }));
    } catch (error) {
      console.error('[AICategories] Failed to parse response', error);
      // Fall back to basic results
      return transactions.map((t, i) => ({
        transactionId: t.id,
        category: 'Uncategorized',
        confidence: 0,
        reasoning: 'Failed to categorize via AI',
      }));
    }
  }

  // ============================================================================
  // FALLBACK: KEYWORD MATCHING
  // ============================================================================

  /**
   * Fallback categorization using keyword patterns
   * Used when API is unavailable or rate-limited
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
  // HISTORICAL EXAMPLES FOR LEARNING
  // ============================================================================

  /**
   * Gets high-quality examples from user's correction history
   * Uses these for few-shot learning in prompts
   */
  private getHistoricalExamples(stewardId: string): CorrectionRecord[] {
    // Filter to this steward's corrections
    const stewardCorrections = this.corrections.filter(
      c => c.stewardId === stewardId
    );

    // Sort by recency
    return stewardCorrections.sort(
      (a, b) =>
        new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()
    );
  }

  // ============================================================================
  // API COMMUNICATION
  // ============================================================================

  /**
   * Calls Anthropic Claude API with retry logic
   */
  private callAnthropicAPI(
    request: AnthropicRequest
  ): Observable<AnthropicResponse> {
    const headers = new HttpHeaders({
      'x-api-key': environment.ai?.apiKey || '',
      'anthropic-version': '2023-06-01',
      'content-type': 'application/json',
    });

    let attempt = 0;

    const attempt$ = (): Observable<AnthropicResponse> => {
      return this.http
        .post<AnthropicResponse>(
          `${this.ANTHROPIC_API_BASE}/messages`,
          request,
          { headers }
        )
        .pipe(
          timeout(30000), // 30 second timeout
          catchError(error => {
            attempt++;
            if (attempt < this.MAX_RETRIES) {
              console.log(`[AICategories] Retry ${attempt}/${this.MAX_RETRIES}`);
              return attempt$();
            }
            console.error('[AICategories] API call failed after retries', error);
            return throwError(() => error);
          })
        );
    };

    return attempt$();
  }
}
