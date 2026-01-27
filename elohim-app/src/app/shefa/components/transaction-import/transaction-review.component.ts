/**
 * Transaction Review Component
 *
 * Primary UI for reviewing and approving staged transactions before they become
 * immutable EconomicEvents.
 *
 * Features:
 * - Keyboard shortcuts for efficient operation (Ctrl+Enter approve, Ctrl+Delete reject)
 * - AI confidence badges (high/medium/low)
 * - Category dropdown with alternative suggestions
 * - Bulk approve/reject operations
 * - Duplicate warnings with manual resolution
 * - Real-time variance impact preview
 *
 * UX Philosophy:
 * - One transaction at a time (focus + clarity)
 * - Keyboard-first (power user friendly)
 * - Clear visual feedback (amounts, confidence, impact)
 * - Reversible actions (approve/reject before EconomicEvent creation)
 */

import { Component, OnInit, OnDestroy } from '@angular/core';
import { ActivatedRoute } from '@angular/router';

import { Subject } from 'rxjs';

import {
  StagedTransaction,
  ImportBatch,
  CategorySuggestion,
} from '../../models/transaction-import.model';
import { AICategorizationService } from '../../services/ai-categorization.service';
import { BudgetReconciliationService } from '../../services/budget-reconciliation.service';
import { TransactionImportService } from '../../services/transaction-import.service';

/**
 * Budget category for selection dropdown
 */
interface BudgetCategoryOption {
  id: string;
  name: string;
  description?: string;
}

/**
 * Transaction with UI state
 */
interface UITransaction extends StagedTransaction {
  _isSelected?: boolean;
  _varianceImpact?: number;
  _budgetName?: string;
}

@Component({
  selector: 'app-transaction-review',
  templateUrl: './transaction-review.component.html',
  styleUrls: ['./transaction-review.component.scss'],
})
export class TransactionReviewComponent implements OnInit, OnDestroy {
  // Route parameters
  batchId = '';

  // Data
  batch: ImportBatch | null = null;
  stagedTransactions: UITransaction[] = [];
  budgetCategories: BudgetCategoryOption[] = [];

  // Current transaction state
  currentIndex = 0;
  currentTransaction: UITransaction | null = null;

  // Selection & filters
  selectedIds = new Set<string>();
  filterStatus: 'all' | 'pending' | 'needs-attention' = 'pending';
  filterConfidenceMin = 0; // 0-100
  searchText = '';

  // UI state
  isLoading = false;
  isSaving = false;
  showDuplicateWarning = false;
  showCategoryDropdown = false;

  // Suggestions
  allSuggestions: CategorySuggestion[] = [];

  // Keyboard handling
  private readonly destroy$ = new Subject<void>();

  constructor(
    private readonly route: ActivatedRoute,
    private readonly importService: TransactionImportService,
    private readonly aiCategorization: AICategorizationService,
    private readonly budgetReconciliation: BudgetReconciliationService
  ) {}

  ngOnInit(): void {
    // Get batch ID from route
    this.batchId = this.route.snapshot.paramMap.get('batchId') ?? '';
    if (!this.batchId) {
      console.error('[TransactionReview] No batchId in route');
      return;
    }

    // Load batch and transactions
    this.loadBatch();

    // Setup keyboard listeners
    this.setupKeyboardListeners();

    // Load budget categories
    this.loadBudgetCategories();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  /**
   * Loads the import batch and all staged transactions
   */
  private loadBatch(): void {
    this.isLoading = true;

    try {
      this.batch = this.importService.getBatch(this.batchId);
      if (!this.batch) {
        console.error(`[TransactionReview] Batch ${this.batchId} not found`);
        return;
      }

      // Load staged transactions
      const staged = this.importService.getStagedTransactionsForBatch(this.batchId);
      this.stagedTransactions = staged.map(txn => ({
        ...txn,
        _isSelected: false,
      }));

      // Filter and sort
      this.applyFilters();

      // Load first transaction
      if (this.stagedTransactions.length > 0) {
        this.loadTransaction(0);
      }

      console.warn(
        `[TransactionReview] Loaded ${this.stagedTransactions.length} staged transactions`
      );
    } catch (error) {
      console.error('[TransactionReview] Failed to load batch', error);
    } finally {
      this.isLoading = false;
    }
  }

  /**
   * Loads a specific transaction for review
   */
  private loadTransaction(index: number): void {
    if (index < 0 || index >= this.stagedTransactions.length) {
      console.warn(`[TransactionReview] Index ${index} out of bounds`);
      return;
    }

    this.currentIndex = index;
    this.currentTransaction = this.stagedTransactions[index];

    // Load suggestions
    this.allSuggestions = this.currentTransaction.suggestedCategories ?? [];

    // Check for duplicates
    this.showDuplicateWarning = this.currentTransaction.isDuplicate;

    // Calculate variance impact
    if (this.currentTransaction.budgetId) {
      // Variance impact equals transaction amount for now (should be calculated from budget)
      this.currentTransaction._varianceImpact = this.currentTransaction.amount.value;
    }

    console.warn(
      `[TransactionReview] Loaded transaction ${index}: ${this.currentTransaction.description}`
    );
  }

  /**
   * Loads available budget categories for dropdown
   */
  private loadBudgetCategories(): void {
    // Budget categories will be loaded from BudgetService when available
    this.budgetCategories = [
      { id: 'groceries', name: 'Groceries', description: 'Food & household items' },
      { id: 'dining', name: 'Dining', description: 'Restaurants & cafes' },
      { id: 'shopping', name: 'Shopping', description: 'Retail & online' },
      { id: 'transportation', name: 'Transportation', description: 'Gas, rideshare, transit' },
      { id: 'entertainment', name: 'Entertainment', description: 'Movies, games, etc' },
      { id: 'health', name: 'Health', description: 'Pharmacy & medical' },
      { id: 'utilities', name: 'Utilities', description: 'Electric, water, internet' },
      { id: 'housing', name: 'Housing', description: 'Rent, mortgage, property' },
      { id: 'uncategorized', name: 'Uncategorized', description: 'Needs manual review' },
    ];
  }

  /**
   * Applies filters and sorts transactions
   */
  private applyFilters(): void {
    let filtered = [...this.stagedTransactions];

    // Filter by status
    const pendingStatus = 'pending' as const;
    const needsAttentionStatus = 'needs-attention' as const;
    if (this.filterStatus === pendingStatus) {
      filtered = filtered.filter(t => t.reviewStatus === pendingStatus);
    } else if (this.filterStatus === needsAttentionStatus) {
      filtered = filtered.filter(t => t.reviewStatus === needsAttentionStatus);
    }

    // Filter by confidence
    filtered = filtered.filter(t => t.categoryConfidence >= this.filterConfidenceMin);

    // Filter by search
    if (this.searchText.trim()) {
      const search = this.searchText.toLowerCase();
      filtered = filtered.filter(
        t =>
          t.description.toLowerCase().includes(search) ||
          (t.merchantName?.toLowerCase().includes(search) ?? false) ||
          t.category.toLowerCase().includes(search)
      );
    }

    // Sort by amount (largest first for impact)
    filtered.sort((a, b) => Math.abs(b.amount.value) - Math.abs(a.amount.value));

    this.stagedTransactions = filtered;
  }

  // ============================================================================
  // NAVIGATION
  // ============================================================================

  /**
   * Move to next transaction
   */
  nextTransaction(): void {
    if (this.currentIndex < this.stagedTransactions.length - 1) {
      this.loadTransaction(this.currentIndex + 1);
    } else {
      console.warn('[TransactionReview] End of transactions');
    }
  }

  /**
   * Move to previous transaction
   */
  prevTransaction(): void {
    if (this.currentIndex > 0) {
      this.loadTransaction(this.currentIndex - 1);
    } else {
      console.warn('[TransactionReview] Beginning of transactions');
    }
  }

  /**
   * Jump to specific transaction
   */
  jumpToTransaction(index: number): void {
    this.loadTransaction(index);
  }

  // ============================================================================
  // REVIEW ACTIONS
  // ============================================================================

  /**
   * Approve current transaction
   */
  async approveTransaction(): Promise<void> {
    if (!this.currentTransaction) {
      console.error('[TransactionReview] No transaction selected');
      return;
    }

    this.isSaving = true;
    try {
      await this.importService.approveTransaction(this.currentTransaction.id);

      // Update UI
      this.currentTransaction.reviewStatus = 'approved';
      console.warn(`[TransactionReview] Approved: ${this.currentTransaction.description}`);

      // Move to next
      setTimeout(() => this.nextTransaction(), 300);
    } catch (error) {
      console.error('[TransactionReview] Approval failed', error);
      alert(`Failed to approve transaction: ${String(error)}`);
    } finally {
      this.isSaving = false;
    }
  }

  /**
   * Reject current transaction
   */
  async rejectTransaction(): Promise<void> {
    if (!this.currentTransaction) {
      console.error('[TransactionReview] No transaction selected');
      return;
    }

    this.isSaving = true;
    try {
      await this.importService.rejectTransaction(this.currentTransaction.id);

      // Update UI
      this.currentTransaction.reviewStatus = 'rejected';
      console.warn(`[TransactionReview] Rejected: ${this.currentTransaction.description}`);

      // Move to next
      setTimeout(() => this.nextTransaction(), 300);
    } catch (error) {
      console.error('[TransactionReview] Rejection failed', error);
      alert(`Failed to reject transaction: ${String(error)}`);
    } finally {
      this.isSaving = false;
    }
  }

  /**
   * Mark current transaction as needing attention
   */
  markAsNeedsAttention(): void {
    if (!this.currentTransaction) return;

    this.currentTransaction.reviewStatus = 'needs-attention';
    console.warn(
      `[TransactionReview] Marked for attention: ${this.currentTransaction.description}`
    );
  }

  // ============================================================================
  // CATEGORIZATION
  // ============================================================================

  /**
   * Updates category for current transaction
   */
  updateCategory(categoryName: string): void {
    if (!this.currentTransaction) return;

    this.currentTransaction.category = categoryName;
    this.currentTransaction.categorySource = 'manual';
    this.currentTransaction.categoryConfidence = 100; // User explicitly chose

    this.showCategoryDropdown = false;

    console.warn(`[TransactionReview] Category changed to ${categoryName}`);

    // Notify AI service for learning - to be implemented
  }

  /**
   * Accepts an AI suggestion
   */
  acceptSuggestion(suggestion: CategorySuggestion): void {
    if (!this.currentTransaction) return;

    this.currentTransaction.category = suggestion.category;
    this.currentTransaction.categoryConfidence = suggestion.confidence;
    this.currentTransaction.categorySource = 'ai';

    console.warn(`[TransactionReview] Accepted suggestion: ${suggestion.category}`);
  }

  /**
   * Gets confidence badge color
   */
  getConfidenceBadgeClass(): string {
    if (!this.currentTransaction) return '';
    const conf = this.currentTransaction.categoryConfidence;
    if (conf >= 80) return 'badge-success'; // High
    if (conf >= 50) return 'badge-warning'; // Medium
    return 'badge-danger'; // Low
  }

  /**
   * Gets confidence badge text
   */
  getConfidenceBadgeText(): string {
    if (!this.currentTransaction) return '';
    const conf = this.currentTransaction.categoryConfidence;
    if (conf >= 80) return 'High Confidence';
    if (conf >= 50) return 'Medium Confidence';
    return 'Low Confidence';
  }

  // ============================================================================
  // DUPLICATE HANDLING
  // ============================================================================

  /**
   * Mark as not a duplicate (override fuzzy detection)
   */
  markAsNotDuplicate(): void {
    if (!this.currentTransaction) return;

    this.currentTransaction.isDuplicate = false;
    this.showDuplicateWarning = false;

    console.warn('[TransactionReview] Override: marked as not duplicate');
  }

  // ============================================================================
  // BULK OPERATIONS
  // ============================================================================

  /**
   * Toggle selection of current transaction
   */
  toggleSelection(): void {
    if (!this.currentTransaction) return;

    this.currentTransaction._isSelected = !this.currentTransaction._isSelected;
    if (this.currentTransaction._isSelected) {
      this.selectedIds.add(this.currentTransaction.id);
    } else {
      this.selectedIds.delete(this.currentTransaction.id);
    }
  }

  /**
   * Select all visible transactions
   */
  selectAll(): void {
    this.stagedTransactions.forEach(txn => {
      txn._isSelected = true;
      this.selectedIds.add(txn.id);
    });
  }

  /**
   * Clear selection
   */
  clearSelection(): void {
    this.stagedTransactions.forEach(txn => {
      txn._isSelected = false;
    });
    this.selectedIds.clear();
  }

  /**
   * Approve all selected transactions
   */
  async approveBulk(): Promise<void> {
    if (this.selectedIds.size === 0) {
      alert('No transactions selected');
      return;
    }

    const confirmed = confirm(`Approve ${this.selectedIds.size} transactions?`);
    if (!confirmed) return;

    this.isSaving = true;
    try {
      const ids = Array.from(this.selectedIds);
      await this.importService.approveBatch(ids);

      // Update UI
      ids.forEach(id => {
        const txn = this.stagedTransactions.find(t => t.id === id);
        if (txn) txn.reviewStatus = 'approved';
      });

      this.clearSelection();
      alert(`Approved ${ids.length} transactions`);
    } catch (error) {
      console.error('[TransactionReview] Bulk approval failed', error);
      alert(`Bulk approval failed: ${String(error)}`);
    } finally {
      this.isSaving = false;
    }
  }

  // ============================================================================
  // KEYBOARD SHORTCUTS
  // ============================================================================

  /**
   * Sets up keyboard event listeners
   */
  private setupKeyboardListeners(): void {
    document.addEventListener('keydown', (e: KeyboardEvent) => {
      // Cmd/Ctrl + Enter = Approve
      if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
        e.preventDefault();
        this.approveTransaction();
        return;
      }

      // Cmd/Ctrl + Delete = Reject
      if ((e.metaKey || e.ctrlKey) && e.key === 'Delete') {
        e.preventDefault();
        this.rejectTransaction();
        return;
      }

      // Arrow Right = Next
      if (e.key === 'ArrowRight') {
        e.preventDefault();
        this.nextTransaction();
        return;
      }

      // Arrow Left = Previous
      if (e.key === 'ArrowLeft') {
        e.preventDefault();
        this.prevTransaction();
        return;
      }

      // E = Edit category
      if (e.key === 'e' || e.key === 'E') {
        e.preventDefault();
        this.showCategoryDropdown = !this.showCategoryDropdown;
        return;
      }

      // F = Flag for attention
      if (e.key === 'f' || e.key === 'F') {
        e.preventDefault();
        this.markAsNeedsAttention();
        return;
      }

      // Escape = Close dropdown
      if (e.key === 'Escape') {
        this.showCategoryDropdown = false;
      }
    });
  }

  // ============================================================================
  // UI HELPERS
  // ============================================================================

  /**
   * Formats currency for display
   */
  formatCurrency(amount: number, currency: string): string {
    const formatter = new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: currency,
    });
    return formatter.format(amount);
  }

  /**
   * Formats date for display
   */
  formatDate(dateStr: string): string {
    const date = new Date(dateStr);
    return date.toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    });
  }

  /**
   * Gets transaction type badge
   */
  getTransactionTypeBadge(): string {
    if (!this.currentTransaction) return '';
    switch (this.currentTransaction.type) {
      case 'debit':
        return 'Expense';
      case 'credit':
        return 'Income';
      case 'fee':
        return 'Fee';
      case 'transfer':
        return 'Transfer';
      default:
        return 'Other';
    }
  }

  /**
   * Gets transaction type badge color
   */
  getTransactionTypeBadgeClass(): string {
    if (!this.currentTransaction) return '';
    switch (this.currentTransaction.type) {
      case 'debit':
        return 'badge-danger';
      case 'credit':
        return 'badge-success';
      case 'fee':
        return 'badge-warning';
      case 'transfer':
        return 'badge-info';
      default:
        return 'badge-secondary';
    }
  }

  /**
   * Gets completion percentage
   */
  getCompletionPercent(): number {
    const approved = this.stagedTransactions.filter(t => t.reviewStatus === 'approved').length;
    return this.stagedTransactions.length > 0
      ? Math.round((approved / this.stagedTransactions.length) * 100)
      : 0;
  }

  /**
   * Gets remaining transaction count
   */
  getRemainingCount(): number {
    return this.stagedTransactions.filter(t => t.reviewStatus === 'pending').length;
  }
}
