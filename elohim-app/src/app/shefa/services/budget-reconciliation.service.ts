/**
 * Budget Reconciliation Service
 *
 * Updates FlowBudget with actual transaction amounts when transactions are approved.
 *
 * Responsibilities:
 * - Link StagedTransaction to specific FlowBudget categories
 * - Update actualAmount when transaction is approved
 * - Recalculate variance (actual - planned)
 * - Calculate health status (healthy/warning/critical)
 * - Track variance trends for insights
 * - Maintain audit trail of all reconciliations
 *
 * The FlowBudget model is prescriptive (what we planned),
 * while EconomicEvents are the actual transactions that occurred.
 * This service bridges them.
 */

import { Injectable } from '@angular/core';

// @coverage: 95.5% (2026-01-31)

import { StagedTransaction, ReconciliationResult } from '../models/transaction-import.model';

type HealthStatusType = 'healthy' | 'warning' | 'critical';

/**
 * ResourceMeasure
 */
interface ResourceMeasure {
  value: number;
  unit: string; // "USD", etc.
}

/**
 * BudgetCategory within a FlowBudget
 */
interface BudgetCategory {
  id: string;
  name: string;
  description?: string;

  // Planned amounts
  plannedAmount: ResourceMeasure;

  // Actual amounts (from reconciliation)
  actualAmount: ResourceMeasure;

  // Calculated variance
  variance: number;
  variancePercent: number;

  // Linked transactions
  transactionIds: string[]; // EconomicEvent IDs

  // Metadata
  createdAt: string;
  updatedAt?: string;
}

/**
 * FlowBudget - Prescriptive budget for a flow/project
 */
interface FlowBudget {
  id: string;

  // Identity
  budgetNumber: string; // FB-XXXXXXXXXX
  stewardId: string;
  flowId?: string; // Which flow this budget is for
  name: string;
  description?: string;

  // Period
  periodStart: string; // ISO date
  periodEnd: string; // ISO date

  // Budget structure
  categories: BudgetCategory[];

  // Totals
  totalPlanned: ResourceMeasure;
  totalActual: ResourceMeasure;
  totalVariance: number;
  totalVariancePercent: number;

  // Health indicators
  healthStatus: HealthStatusType;
  lastReconciled?: string;

  // Variance tracking
  varianceTrend?: {
    timestamp: string;
    variancePercent: number;
  }[];

  // Audit
  createdAt: string;
  updatedAt: string;
}

/**
 * Variance notification
 */
interface VarianceAlert {
  budgetId: string;
  categoryId?: string;
  severity: 'info' | 'warning' | 'critical';
  message: string;
  variancePercent: number;
  timestamp: string;
}

@Injectable({
  providedIn: 'root',
})
export class BudgetReconciliationService {
  // Variance thresholds
  private readonly CRITICAL_THRESHOLD = 0.2; // 20% over budget
  private readonly WARNING_THRESHOLD = 0.1; // 10% over budget

  constructor() {
    // private flowPlanning: FlowPlanningService,  // TODO: Inject actual service
    // private budgetService: BudgetService,
    // private notificationService: NotificationService,
  }

  /**
   * Main reconciliation method
   *
   * Called after an EconomicEvent is created from an approved StagedTransaction.
   *
   * Updates:
   * 1. Actual amount in budget category
   * 2. Variance calculation
   * 3. Health status
   * 4. Transaction linkage
   * 5. Emits alerts if thresholds crossed
   */
  async reconcileBudget(
    staged: StagedTransaction,
    economicEventId: string
  ): Promise<ReconciliationResult> {
    // Skip if no budget linkage
    if (!staged.budgetId || !staged.budgetCategoryId) {
      return {
        budgetId: '',
        budgetCategoryId: '',
        previousActualAmount: 0,
        newActualAmount: 0,
        amountAdded: 0,
        varianceBeforeReconciliation: 0,
        varianceAfterReconciliation: 0,
        newHealthStatus: 'healthy',
        reconciled: false,
        timestamp: new Date().toISOString(),
      };
    }

    try {
      // TODO: Replace with actual BudgetService call
      // const budget = await this.budgetService.getBudget(staged.budgetId);

      // Mock budget retrieval
      const budget = this.createMockBudget(staged.budgetId, staged.stewardId);

      const category = budget.categories.find(c => c.id === staged.budgetCategoryId);

      if (!category) {
        throw new Error(
          `Budget category ${staged.budgetCategoryId} not found in budget ${staged.budgetId}`
        );
      }

      // Calculate changes
      const previousActualAmount = category.actualAmount.value;
      const amountToAdd = staged.amount.value;
      const newActualAmount = previousActualAmount + amountToAdd;

      // Update actual amount
      category.actualAmount.value = newActualAmount;

      // Recalculate variance for this category
      category.variance = category.actualAmount.value - category.plannedAmount.value;
      category.variancePercent = (category.variance / category.plannedAmount.value) * 100;

      // Link transaction
      if (!category.transactionIds) {
        category.transactionIds = [];
      }
      category.transactionIds.push(economicEventId);
      category.updatedAt = new Date().toISOString();

      // Update budget totals
      this.recalculateBudgetTotals(budget);

      // Determine health status
      const previousHealthStatus = budget.healthStatus;
      budget.healthStatus = this.calculateHealthStatus(budget);
      budget.lastReconciled = new Date().toISOString();

      // Add to variance trend
      this.addVarianceTrend(budget);

      // Persist to storage
      await this.updateBudget(budget);

      // Check for alerts
      this.checkVarianceAlerts(budget, category, previousHealthStatus);

      const result: ReconciliationResult = {
        budgetId: staged.budgetId,
        budgetCategoryId: staged.budgetCategoryId,
        previousActualAmount,
        newActualAmount,
        amountAdded: amountToAdd,
        varianceBeforeReconciliation: previousActualAmount - category.plannedAmount.value,
        varianceAfterReconciliation: category.variance,
        newHealthStatus: budget.healthStatus,
        reconciled: true,
        timestamp: new Date().toISOString(),
      };
      return result;
    } catch (error) {
      throw new Error('Budget reconciliation failed: ' + String(error));
    }
  }

  /**
   * Bulk reconcile multiple transactions
   */
  async reconcileMultiple(
    transactions: { staged: StagedTransaction; eventId: string }[]
  ): Promise<ReconciliationResult[]> {
    const results: ReconciliationResult[] = [];

    for (const { staged, eventId } of transactions) {
      try {
        const result = await this.reconcileBudget(staged, eventId);
        results.push(result);
      } catch {
        // Budget reconciliation failed for this transaction - continue with others
      }
    }

    return results;
  }

  // ============================================================================
  // VARIANCE CALCULATION
  // ============================================================================

  /**
   * Recalculates all totals in a budget
   */
  private recalculateBudgetTotals(budget: FlowBudget): void {
    let totalPlannedValue = 0;
    let totalActualValue = 0;

    for (const category of budget.categories) {
      totalPlannedValue += category.plannedAmount.value;
      totalActualValue += category.actualAmount.value;
    }

    budget.totalPlanned.value = totalPlannedValue;
    budget.totalActual.value = totalActualValue;
    budget.totalVariance = budget.totalActual.value - budget.totalPlanned.value;
    budget.totalVariancePercent = (budget.totalVariance / budget.totalPlanned.value) * 100;
  }

  /**
   * Calculates budget health status based on variance
   *
   * Factors:
   * - Percentage of categories over budget
   * - Total variance as percentage of planned
   *
   * Returns: healthy | warning | critical
   */
  private calculateHealthStatus(budget: FlowBudget): 'healthy' | 'warning' | 'critical' {
    // Critical: 20% over budget total
    if (budget.totalVariance > budget.totalPlanned.value * this.CRITICAL_THRESHOLD) {
      return 'critical';
    }

    // Count categories over budget
    const overBudgetCategories = budget.categories.filter(c => c.variance > 0);
    const overBudgetPercent = (overBudgetCategories.length / budget.categories.length) * 100;

    // Warning: >50% categories over budget OR 10% over total
    if (
      overBudgetPercent > 50 ||
      budget.totalVariance > budget.totalPlanned.value * this.WARNING_THRESHOLD
    ) {
      return 'warning';
    }

    return 'healthy';
  }

  /**
   * Tracks variance trend over time
   */
  private addVarianceTrend(budget: FlowBudget): void {
    budget.varianceTrend ??= [];

    budget.varianceTrend.push({
      timestamp: new Date().toISOString(),
      variancePercent: budget.totalVariancePercent,
    });

    // Keep last 30 data points
    if (budget.varianceTrend.length > 30) {
      budget.varianceTrend = budget.varianceTrend.slice(-30);
    }
  }

  // ============================================================================
  // ALERTING
  // ============================================================================

  /**
   * Checks if variance has crossed alert thresholds and emits alerts
   */
  private checkVarianceAlerts(
    budget: FlowBudget,
    category: BudgetCategory,
    previousHealthStatus: HealthStatusType
  ): void {
    const alerts: VarianceAlert[] = [];

    // Category-level alerts
    if (category.variancePercent > 20) {
      alerts.push({
        budgetId: budget.id,
        categoryId: category.id,
        severity: 'critical',
        message: `Category "${category.name}" is ${Math.abs(category.variancePercent).toFixed(1)}% over budget`,
        variancePercent: category.variancePercent,
        timestamp: new Date().toISOString(),
      });
    } else if (category.variancePercent > 10) {
      alerts.push({
        budgetId: budget.id,
        categoryId: category.id,
        severity: 'warning',
        message: `Category "${category.name}" is ${Math.abs(category.variancePercent).toFixed(1)}% over budget`,
        variancePercent: category.variancePercent,
        timestamp: new Date().toISOString(),
      });
    }

    // Budget-level alerts
    if (previousHealthStatus !== budget.healthStatus && budget.healthStatus !== 'healthy') {
      alerts.push({
        budgetId: budget.id,
        severity: budget.healthStatus === 'critical' ? 'critical' : 'warning',
        message: `Budget "${budget.name}" status changed to ${budget.healthStatus}`,
        variancePercent: budget.totalVariancePercent,
        timestamp: new Date().toISOString(),
      });
    }

    // Emit alerts (notification service integration pending)
    if (alerts.length > 0) {
      // TODO: Emit alerts via notification service
    }
  }

  // ============================================================================
  // PERSISTENCE
  // ============================================================================

  /**
   * Updates budget in storage
   * Persistence will be integrated with BudgetService
   */
  private updateBudget(_budget: FlowBudget): Promise<void> {
    // Persistence integration pending
    return Promise.resolve();
  }

  /**
   * Retrieves a budget
   * Retrieval will be integrated with BudgetService
   */
  private getBudget(budgetId: string): Promise<FlowBudget> {
    // BudgetService integration pending
    return Promise.resolve(this.createMockBudget(budgetId, ''));
  }

  /**
   * Creates a mock budget for testing
   */
  private createMockBudget(budgetId: string, stewardId: string): FlowBudget {
    return {
      id: budgetId,
      budgetNumber: `FB-${(crypto.getRandomValues(new Uint32Array(1))[0] / 2 ** 32).toString(36).substring(2, 10).toUpperCase()}`,
      stewardId,
      name: 'Monthly Budget',
      description: 'Mock budget for testing',

      periodStart: new Date().toISOString().split('T')[0],
      periodEnd: new Date(Date.now() + 30 * 24 * 60 * 60 * 1000).toISOString().split('T')[0],

      categories: [
        {
          id: 'cat-1',
          name: 'Groceries',
          plannedAmount: { value: 500, unit: 'USD' },
          actualAmount: { value: 0, unit: 'USD' },
          variance: 0,
          variancePercent: 0,
          transactionIds: [],
          createdAt: new Date().toISOString(),
        },
        {
          id: 'cat-2',
          name: 'Dining',
          plannedAmount: { value: 200, unit: 'USD' },
          actualAmount: { value: 0, unit: 'USD' },
          variance: 0,
          variancePercent: 0,
          transactionIds: [],
          createdAt: new Date().toISOString(),
        },
        {
          id: 'cat-3',
          name: 'Transportation',
          plannedAmount: { value: 300, unit: 'USD' },
          actualAmount: { value: 0, unit: 'USD' },
          variance: 0,
          variancePercent: 0,
          transactionIds: [],
          createdAt: new Date().toISOString(),
        },
      ],

      totalPlanned: { value: 1000, unit: 'USD' },
      totalActual: { value: 0, unit: 'USD' },
      totalVariance: 0,
      totalVariancePercent: 0,

      healthStatus: 'healthy',

      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };
  }
}
