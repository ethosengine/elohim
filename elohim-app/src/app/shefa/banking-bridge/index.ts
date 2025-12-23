/**
 * Banking Bridge Module - Isolated Translation Layer
 *
 * This module handles all Plaid/banking integration LOCALLY (IndexedDB).
 * Only the EconomicEventBridgeService crosses into Holochain when
 * transactions are approved.
 *
 * Exports:
 * - Store: Local IndexedDB for staging data
 * - Bridge: Service to commit approved transactions to Holochain
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

// Bridge service
export {
  EconomicEventBridgeService,
  type EconomicEventPayload,
  type CommitResult,
  type BatchCommitResult,
} from './services/economic-event-bridge.service';
