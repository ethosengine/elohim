/**
 * Transaction Import Domain Models
 *
 * Defines entities for external bank account integration via Plaid API
 * with staged reconciliation workflow and AI-based categorization.
 *
 * Data Flow:
 * Plaid Bank API → PlaidTransaction[] → StagedTransaction[] → EconomicEvent[]
 */

import { ResourceMeasure } from './stewarded-resources.model';
import { EventState } from '@app/elohim/models/economic-event.model';

// ============================================================================
// PLAID CONNECTION MANAGEMENT
// ============================================================================

/**
 * PlaidConnection represents an OAuth-authenticated connection to a user's
 * financial institution via Plaid API.
 *
 * Stores encrypted access tokens and account mappings for transaction sync.
 */
export interface PlaidConnection {
  id: string;
  connectionNumber: string; // PC-XXXXXXXXXX formatted ID

  stewardId: string; // Who owns this connection

  // Plaid OAuth credentials (encrypted client-side)
  plaidItemId: string; // Unique identifier for user account on Plaid
  plaidAccessToken: string; // Encrypted with Web Crypto API - never logged
  plaidInstitutionId: string; // e.g., "ins_3"
  institutionName: string; // e.g., "Chase Bank"

  // Account mapping: Plaid accounts → Elohim FinancialAssets
  linkedAccounts: PlaidAccountLink[];

  // Connection health
  status: 'active' | 'requires-reauth' | 'disconnected' | 'error';
  lastSyncedAt?: string;
  errorMessage?: string;
  webhookUrl?: string;

  // Audit trail
  createdAt: string;
  updatedAt: string;
}

/**
 * Maps a Plaid account to an Elohim FinancialAsset for transaction linking.
 */
export interface PlaidAccountLink {
  plaidAccountId: string;
  plaidAccountName: string;
  plaidAccountSubtype: string; // e.g., "checking", "savings"
  financialAssetId: string; // Elohim FinancialAsset this account maps to
  balanceAmount: number;
  currency: string;
  lastLinkedAt: string;
}

// ============================================================================
// IMPORT BATCH MANAGEMENT
// ============================================================================

/**
 * ImportBatch tracks a cohesive set of transaction imports from a Plaid
 * connection for a specific date range and account set.
 *
 * State machine:
 * created → fetching → categorizing → staged → reviewing → approved → completed
 *                                                             ↓
 *                                                         rejected
 */
export interface ImportBatch {
  id: string;
  batchNumber: string; // IB-XXXXXXXXXX formatted ID

  stewardId: string; // Who initiated this import
  connectionId: string; // Which connection was used

  // Scope
  accountIds: string[]; // Which Plaid accounts to import from
  dateRange: {
    start: string; // ISO 8601 date
    end: string; // ISO 8601 date
  };

  // Statistics
  totalTransactions: number;
  newTransactions: number; // Not duplicates
  duplicateTransactions: number;
  errorTransactions: number;

  // Processing status
  status:
    | 'fetching' // Pulling from Plaid API
    | 'categorizing' // AI classification in progress
    | 'staged' // Ready for review
    | 'reviewing' // User reviewing transactions
    | 'approved' // Ready to create EconomicEvents
    | 'completed' // EconomicEvents created
    | 'rejected'; // User rejected batch

  stagedTransactionIds: string[]; // FK to StagedTransaction

  // AI categorization
  aiCategorizationEnabled: boolean;
  aiCategorizationCompletedAt?: string;

  // Review workflow
  reviewedBy?: string; // AgentPubKey of reviewer
  reviewedAt?: string;
  reviewNotes?: string;

  // Audit trail
  createdAt: string;
  completedAt?: string;
}

// ============================================================================
// STAGED TRANSACTION (Pre-EconomicEvent)
// ============================================================================

/**
 * StagedTransaction represents a transaction fetched from Plaid that is
 * pending review and categorization before being converted to an immutable
 * EconomicEvent.
 *
 * This staged approach allows:
 * 1. Manual review before commitment
 * 2. AI categorization suggestions
 * 3. Duplicate detection and resolution
 * 4. Budget category linkage
 *
 * State machine:
 * pending → categorized → [approved | rejected | needs-attention]
 *                              ↓
 *                         completed (EconomicEvent created)
 */
export interface StagedTransaction {
  id: string;
  batchId: string; // FK to ImportBatch
  stewardId: string; // Who owns this transaction

  // Source tracking (critical for reconciliation)
  plaidTransactionId: string; // Plaid's unique ID (prevents re-import)
  plaidAccountId: string; // Which Plaid account this came from
  financialAssetId: string; // Elohim FinancialAsset

  // Transaction data from Plaid
  timestamp: string; // ISO 8601 datetime when transaction occurred
  type: 'debit' | 'credit' | 'transfer' | 'fee';
  amount: ResourceMeasure; // { value: number, unit: string (e.g., "USD") }
  description: string; // Raw description from bank
  merchantName?: string; // Parsed merchant name

  // AI categorization
  category: string; // BudgetCategory.name that this maps to
  categoryConfidence: number; // 0-100, higher = more confident
  categorySource: 'ai' | 'plaid' | 'manual' | 'rule'; // Where category came from
  suggestedCategories?: CategorySuggestion[]; // Alternative AI suggestions

  // Budget linkage (for variance tracking)
  budgetId?: string; // Which FlowBudget this maps to
  budgetCategoryId?: string; // Which BudgetCategory within that budget

  // Duplicate detection
  isDuplicate: boolean;
  duplicateOfTransactionId?: string; // If duplicate, which transaction it duplicates
  duplicateConfidence?: number; // 0-100 confidence in duplicate determination

  // Review workflow
  reviewStatus: 'pending' | 'approved' | 'rejected' | 'needs-attention';
  reviewedBy?: string; // AgentPubKey who reviewed
  reviewedAt?: string;

  // Economic event (created after approval)
  economicEventId?: string; // FK to EconomicEvent created from this
  eventState?: EventState; // Snapshot of event state

  // Preserve everything for debugging and reconciliation
  plaidRawData: Record<string, unknown>; // Original Plaid transaction JSON

  // Audit trail
  createdAt: string;
}

// ============================================================================
// AI CATEGORIZATION
// ============================================================================

/**
 * CategorySuggestion represents one possible categorization suggested by
 * the AI categorization service with confidence and reasoning.
 */
export interface CategorySuggestion {
  category: string; // Budget category name
  confidence: number; // 0-100
  reasoning?: string; // Why this category was suggested
  source: 'ai' | 'pattern' | 'keyword' | 'historical'; // What led to this suggestion
}

/**
 * CategorizationResponse contains results from AI categorization batch.
 */
export interface CategorizationResponse {
  results: CategorizationResult[];
  duration?: number; // Milliseconds to categorize
  model?: string; // Which AI model was used (e.g., "claude-3-5-sonnet")
}

export interface CategorizationResult {
  transactionId: string; // FK to StagedTransaction
  category: string; // Recommended category
  confidence: number; // 0-100
  reasoning?: string;
  alternatives?: CategorySuggestion[];
}

// ============================================================================
// TRANSACTION RULES (Auto-Categorization)
// ============================================================================

/**
 * TransactionRule enables automatic categorization of transactions based
 * on merchant name, description patterns, or amount ranges.
 *
 * Rules are learned from user corrections to AI categorizations:
 * - When a user corrects an AI categorization 5+ times for the same merchant
 * - With confidence > 90%, a rule is auto-created
 */
export interface TransactionRule {
  id: string;
  ruleNumber: string; // TR-XXXXXXXXXX formatted ID

  stewardId: string; // Who owns this rule

  // Rule definition
  name: string; // e.g., "Amazon purchases → Shopping"
  description?: string;

  // Matching criteria
  matchType:
    | 'exact' // Exact string match
    | 'contains' // Substring match
    | 'starts-with'
    | 'regex' // Regular expression
    | 'merchant' // Match merchant name
    | 'amount-range'; // Match amount between min/max

  matchField: 'description' | 'merchant' | 'amount' | 'account';
  matchValue: string | number; // String for text fields, number for amounts
  matchValueMax?: number; // For amount-range type

  // Action when rule matches
  action:
    | 'categorize' // Apply targetCategory
    | 'flag' // Mark as needs-attention
    | 'auto-approve' // Automatically approve and create event
    | 'auto-reject'; // Automatically reject

  targetCategory?: string; // Category to apply
  targetBudgetId?: string; // Budget to link to

  // Learning
  learnFromCorrections: boolean; // Should corrections update this rule?
  accuracyRate?: number; // Percentage of transactions where rule was correct

  // Control
  priority: number; // 0-100, higher = evaluated first
  enabled: boolean;

  // Audit trail
  createdAt: string;
  updatedAt?: string;
}

// ============================================================================
// IMPORT WORKFLOW
// ============================================================================

/**
 * ImportRequest initiates a new import batch.
 */
export interface ImportRequest {
  connectionId: string; // Which Plaid connection to use
  accountIds?: string[]; // Specific accounts to import (all if omitted)
  dateRange: {
    start: string; // ISO 8601
    end: string; // ISO 8601
  };
  aiCategorizationEnabled?: boolean; // Defaults to true
  skipDuplicateCheck?: boolean; // Defaults to false
}

/**
 * DuplicateResult indicates whether a transaction is a duplicate of an
 * existing one and with what confidence.
 */
export interface DuplicateResult {
  isDuplicate: boolean;
  confidence: number; // 0-100
  matchId?: string; // ID of transaction this duplicates
  reason?: string; // Why it was flagged as duplicate
}

/**
 * FuzzyMatch result from fuzzy duplicate detection.
 */
export interface FuzzyMatch {
  id: string; // ID of matching transaction
  confidence: number; // 0-100
  reason: string; // Description of match
}

/**
 * SyncResult from real-time webhook sync.
 */
export interface SyncResult {
  newTransactionsCount: number;
  updatedTransactionsCount: number;
  syncedAt: string;
  nextCursorValue?: string;
}

/**
 * PlaidWebhookPayload structure from Plaid webhooks.
 */
export interface PlaidWebhookPayload {
  webhook_type: string; // "TRANSACTIONS"
  webhook_code: string; // "SYNC_UPDATES" | "INITIAL_UPDATE" | etc.
  item_id: string;
  account_id?: string;
  new_transactions: number;
  removed_transaction_ids?: string[];
  cursor?: string;
}

/**
 * PlaidTransaction is the raw transaction from Plaid API.
 */
export interface PlaidTransaction {
  transaction_id: string;
  account_id: string;
  amount: number;
  iso_currency_code: string;
  authorized_date: string; // YYYY-MM-DD
  date: string; // YYYY-MM-DD
  name: string;
  merchant_name?: string;
  personal_finance_category?: {
    primary: string;
    detailed: string;
  };
  counterparties?: Array<{
    name: string;
    type: string;
  }>;
  transaction_type: string;
  pending: boolean;
  [key: string]: unknown;
}

// ============================================================================
// CORRECTION LEARNING
// ============================================================================

/**
 * CorrectionRecord captures when a user corrects an AI categorization.
 * Used to improve future AI predictions and create transaction rules.
 */
export interface CorrectionRecord {
  id: string;
  stewardId: string;

  // Original AI categorization
  transactionDescription: string;
  merchantName?: string;
  transactionAmount: number;
  originalCategory: string;
  originalConfidence: number;

  // User's correction
  correctedCategory: string;
  correctionReason?: string; // Why user disagreed

  // Learning impact
  improvedAccuracy: boolean; // Whether this correction improved future AI
  ruleCreatedFrom: boolean; // Whether this led to a TransactionRule

  // Audit trail
  timestamp: string;
}

// ============================================================================
// BUDGET RECONCILIATION
// ============================================================================

/**
 * ReconciliationResult tracks the outcome of reconciling a transaction
 * with a FlowBudget.
 */
export interface ReconciliationResult {
  budgetId: string;
  budgetCategoryId: string;
  previousActualAmount: number;
  newActualAmount: number;
  amountAdded: number;
  varianceBeforeReconciliation: number;
  varianceAfterReconciliation: number;
  newHealthStatus: 'healthy' | 'warning' | 'critical';
  reconciled: boolean;
  timestamp: string;
}

// ============================================================================
// EXPORT TYPES FOR HOLOCHAIN
// ============================================================================

/**
 * Types exported to Holochain integrity zome.
 * Simplified versions without UI-only fields.
 */

export interface PlaidConnectionEntry {
  connection_number: string;
  steward_id: string;
  plaid_item_id: string;
  plaid_access_token_encrypted: string;
  institution_name: string;
  linked_accounts_json: string; // Serialized Vec<PlaidAccountLink>
  status: string;
  created_at: number; // Timestamp
  updated_at: number; // Timestamp
}

export interface ImportBatchEntry {
  batch_number: string;
  steward_id: string;
  connection_id: string;
  date_range_json: string;
  total_transactions: number;
  status: string;
  created_at: number;
  completed_at?: number;
}

export interface StagedTransactionEntry {
  steward_id: string;
  batch_id: string;
  plaid_transaction_id: string;
  timestamp: number;
  amount: number; // f64 in Rust
  description: string;
  category: string;
  category_confidence: number; // u8 in Rust (0-100)
  review_status: string;
  economic_event_id?: string;
  plaid_raw_data_json: string;
}

export interface TransactionRuleEntry {
  rule_number: string;
  steward_id: string;
  name: string;
  match_type: string;
  match_field: string;
  match_value: string;
  target_category?: string;
  enabled: boolean;
  priority: number;
}
