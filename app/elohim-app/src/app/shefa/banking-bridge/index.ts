/**
 * Banking Bridge Module - Isolated Translation Layer
 *
 * This module handles all Plaid/banking integration LOCALLY (IndexedDB).
 *
 * Exports:
 * - Store: Local IndexedDB for staging data
 * - Types: Local-only type definitions
 */

// Store
export {
  BankingStore,
  bankingStore,
  type PlaidConnectionLocal,
  type PlaidAccountLinkLocal,
  type ImportBatchLocal,
  type StagedTransactionLocal,
  type TransactionRuleLocal,
  type CorrectionRecordLocal,
} from './stores/banking-store';
