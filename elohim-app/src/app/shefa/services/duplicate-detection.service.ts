/**
 * Duplicate Detection Service
 *
 * Multi-level duplicate detection for imported transactions:
 * 1. Exact match: plaidTransactionId already imported
 * 2. Hash match: SHA256(accountId|amount|date|description)
 * 3. Fuzzy match: Similar description + amount ±$0.01 + date ±2 days
 *
 * Prevents the same transaction from being imported multiple times,
 * which would create duplicate EconomicEvents.
 */

import { Injectable } from '@angular/core';

// @coverage: 58.3% (2026-02-05)

import {
  DuplicateResult,
  FuzzyMatch,
  PlaidTransaction,
  StagedTransaction,
} from '../models/transaction-import.model';

/**
 * Transaction candidate for fuzzy matching
 */
interface TransactionCandidate {
  id: string;
  plaidAccountId: string;
  amount: number;
  timestamp: string; // ISO date
  description: string;
  merchantName?: string;
}

/**
 * Hash-based transaction lookup index
 */
type HashIndex = Record<string, string[]>;

@Injectable({
  providedIn: 'root',
})
export class DuplicateDetectionService {
  // In-memory index of transaction hashes (would be backed by DHT in production)
  private hashIndex: HashIndex = {};

  // Set of known plaidTransactionIds
  private readonly plaidIdSet = new Set<string>();

  /**
   * Checks if a single transaction is a duplicate.
   *
   * Returns DuplicateResult with confidence and reasoning.
   */
  detect(transaction: PlaidTransaction): DuplicateResult {
    // Level 1: Exact plaidTransactionId match
    const exactMatch = this.checkExactMatch(transaction.transaction_id);
    if (exactMatch) {
      return {
        isDuplicate: true,
        confidence: 100,
        matchId: exactMatch,
        reason: 'Exact plaidTransactionId match - transaction already imported',
      };
    }

    // Level 2: Hash match (same account + amount + date + description)
    const hash = this.generateHash(transaction);
    const hashMatch = this.checkHashMatch(hash);
    if (hashMatch) {
      return {
        isDuplicate: true,
        confidence: 95,
        matchId: hashMatch,
        reason: 'Hash collision: same account, amount, date, and description - likely duplicate',
      };
    }

    // Level 3: Fuzzy match (similar transactions)
    const fuzzyMatch = this.fuzzySearch(transaction);
    if (fuzzyMatch && fuzzyMatch.confidence > 75) {
      // Note: threshold can be adjusted
      return {
        isDuplicate: true,
        confidence: fuzzyMatch.confidence,
        matchId: fuzzyMatch.id,
        reason: fuzzyMatch.reason,
      };
    }

    // Not a duplicate
    return {
      isDuplicate: false,
      confidence: 0,
    };
  }

  /**
   * Filters a list of transactions to remove duplicates.
   *
   * Returns only new, non-duplicate transactions.
   */
  filterDuplicates(transactions: PlaidTransaction[]): PlaidTransaction[] {
    const uniqueTransactions: PlaidTransaction[] = [];
    const seenHashes = new Set<string>();

    for (const txn of transactions) {
      const dupResult = this.detect(txn);

      if (!dupResult.isDuplicate && !seenHashes.has(this.generateHash(txn))) {
        uniqueTransactions.push(txn);
        seenHashes.add(this.generateHash(txn));
      }
    }

    return uniqueTransactions;
  }

  /**
   * Registers a transaction as processed (adds to indexes)
   * Call this after a transaction is successfully imported.
   */
  registerTransaction(staged: StagedTransaction): void {
    this.plaidIdSet.add(staged.plaidTransactionId);

    const hash = this.generateHashFromStaged(staged);
    if (!this.hashIndex[hash]) {
      this.hashIndex[hash] = [];
    }
    this.hashIndex[hash].push(staged.id);
  }

  /**
   * Bulk register multiple transactions
   */
  registerTransactions(stagedList: StagedTransaction[]): void {
    stagedList.forEach(staged => this.registerTransaction(staged));
  }

  /**
   * Clears all indexes (for testing or refresh)
   */
  clearIndexes(): void {
    this.hashIndex = {};
    this.plaidIdSet.clear();
  }

  // ============================================================================
  // LEVEL 1: EXACT MATCH
  // ============================================================================

  /**
   * Checks if this exact plaidTransactionId has been imported before.
   *
   * Most reliable check - Plaid IDs are unique within an account.
   */
  private checkExactMatch(plaidTransactionId: string): string | null {
    if (this.plaidIdSet.has(plaidTransactionId)) {
      // In production, query DHT to get the transaction ID
      return plaidTransactionId; // Placeholder
    }
    return null;
  }

  // ============================================================================
  // LEVEL 2: HASH MATCH
  // ============================================================================

  /**
   * Generates deterministic SHA256 hash from transaction components.
   *
   * Components (in order):
   * - account_id
   * - amount (rounded to 2 decimals)
   * - date
   * - name (transaction description)
   *
   * Same transaction imported twice will have identical hash.
   */
  private generateHash(transaction: PlaidTransaction): string {
    const data = `${transaction.account_id}|${this.roundAmount(transaction.amount)}|${transaction.date}|${transaction.name}`;
    return this.sha256Hash(data);
  }

  /**
   * Generates hash from StagedTransaction
   */
  private generateHashFromStaged(staged: StagedTransaction): string {
    const data = `${staged.plaidAccountId}|${this.roundAmount(staged.amount.value)}|${staged.timestamp.split('T')[0]}|${staged.description}`;
    return this.sha256Hash(data);
  }

  /**
   * Checks if hash exists in index
   */
  private checkHashMatch(hash: string): string | null {
    const matches = this.hashIndex[hash];
    if (matches && matches.length > 0) {
      return matches[0]; // Return first match
    }
    return null;
  }

  // ============================================================================
  // LEVEL 3: FUZZY MATCH
  // ============================================================================

  /**
   * Fuzzy duplicate detection using:
   * 1. Same account ✓
   * 2. Amount within ±$0.01
   * 3. Date within ±2 days
   * 4. Description similarity (Levenshtein distance < 3)
   *
   * All criteria must be met for a match.
   */
  private fuzzySearch(txn: PlaidTransaction): FuzzyMatch | null {
    // Build list of candidate transactions (same account, similar amount)
    const candidates = this.getCandidates(txn.account_id, txn.amount, txn.date);

    for (const candidate of candidates) {
      const amountMatch = Math.abs(candidate.amount - txn.amount) <= 0.01;
      const dateMatch = this.dateDiffDays(candidate.timestamp, txn.date) <= 2;
      const descMatch =
        this.levenshteinDistance(candidate.description.toLowerCase(), txn.name.toLowerCase()) <= 3;

      if (amountMatch && dateMatch && descMatch) {
        const confidence = this.calculateFuzzyConfidence(candidate, txn);

        return {
          id: candidate.id,
          confidence,
          reason: `Fuzzy match: similar description (${this.levenshteinDistance(
            candidate.description.toLowerCase(),
            txn.name.toLowerCase()
          )} chars), amount ±$0.01, date ±2 days`,
        };
      }
    }

    return null;
  }

  /**
   * Finds candidate transactions for fuzzy matching
   *
   * Candidates are:
   * - Same account
   * - Amount within ±5% (broader initial filter)
   * - Date within ±14 days (broader initial filter)
   */
  private getCandidates(
    _accountId: string,
    _amount: number,
    _dateStr: string
  ): TransactionCandidate[] {
    // In production, query DHT instead of in-memory
    // For now, return empty (no historical transactions in memory)
    return [];
  }

  /**
   * Calculates fuzzy match confidence (0-100)
   *
   * Factors:
   * - Amount difference (closer = higher)
   * - Date difference (closer = higher)
   * - Description similarity (more similar = higher)
   */
  private calculateFuzzyConfidence(candidate: TransactionCandidate, txn: PlaidTransaction): number {
    let score = 75; // Base score for passing all criteria

    // Amount similarity (±$1.00 = full points)
    const amountDiff = Math.abs(candidate.amount - txn.amount);
    const amountScore = Math.max(0, 15 * (1 - amountDiff / 1.0));
    score += amountScore;

    // Date similarity (±7 days = full points)
    const dateDiff = this.dateDiffDays(candidate.timestamp, txn.date);
    const dateScore = Math.max(0, 5 * (1 - dateDiff / 7));
    score += dateScore;

    // Description similarity (0 distance = full points)
    const descDist = this.levenshteinDistance(
      candidate.description.toLowerCase(),
      txn.name.toLowerCase()
    );
    const descScore = Math.max(0, 5 * (1 - descDist / 10));
    score += descScore;

    return Math.min(100, Math.round(score));
  }

  // ============================================================================
  // UTILITY: STRING SIMILARITY
  // ============================================================================

  /**
   * Levenshtein distance between two strings.
   *
   * Measures minimum edits (insert, delete, replace) needed to transform
   * one string into another. Lower distance = more similar.
   */
  private levenshteinDistance(a: string, b: string): number {
    const matrix: number[][] = [];

    for (let i = 0; i <= b.length; i++) {
      matrix[i] = [i];
    }

    for (let j = 0; j <= a.length; j++) {
      matrix[0][j] = j;
    }

    for (let i = 1; i <= b.length; i++) {
      for (let j = 1; j <= a.length; j++) {
        if (b.charAt(i - 1) === a.charAt(j - 1)) {
          matrix[i][j] = matrix[i - 1][j - 1];
        } else {
          matrix[i][j] = Math.min(
            matrix[i - 1][j - 1] + 1, // Substitution
            matrix[i][j - 1] + 1, // Insertion
            matrix[i - 1][j] + 1 // Deletion
          );
        }
      }
    }

    return matrix[b.length][a.length];
  }

  // ============================================================================
  // UTILITY: HASHING
  // ============================================================================

  /**
   * Simple SHA256-like hash (in browser without crypto library)
   *
   * Note: This is a placeholder. In production, use:
   * const hash = await crypto.subtle.digest('SHA-256', data);
   */
  private sha256Hash(data: string): string {
    // Placeholder implementation using simple hash
    // In production: return proper SHA256 hash
    let hash = 0;
    for (let i = 0; i < data.length; i++) {
      const char = data.codePointAt(i) ?? 0;
      hash = (hash << 5) - hash + char;
      hash = hash & hash; // Convert to 32-bit integer
    }
    return Math.abs(hash).toString(16).padStart(64, '0');
  }

  // ============================================================================
  // UTILITY: DATE OPERATIONS
  // ============================================================================

  /**
   * Calculates difference in days between two dates
   */
  private dateDiffDays(dateStr1: string, dateStr2: string): number {
    const date1 = new Date(dateStr1);
    const date2 = new Date(dateStr2);
    const diffTime = Math.abs(date2.getTime() - date1.getTime());
    return Math.ceil(diffTime / (1000 * 60 * 60 * 24));
  }

  // ============================================================================
  // UTILITY: AMOUNT OPERATIONS
  // ============================================================================

  /**
   * Rounds amount to 2 decimal places for comparison
   */
  private roundAmount(amount: number): number {
    return Math.round(amount * 100) / 100;
  }
}
