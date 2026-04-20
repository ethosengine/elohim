/**
 * IFlowPlanning -- Abstract interface for comprehensive flow management:
 * plan CRUD, budgets, goals, projections, scenarios, cadence, and dashboard.
 *
 * Decouples planning lifecycle from persistence. Consumers inject the
 * FLOW_PLANNING token; the default factory resolves to FlowPlanningApiService.
 *
 * @example
 * ```typescript
 * private readonly planning = inject(FLOW_PLANNING);
 * await this.planning.createPlan(stewardId, name, 'monthly', start, end, scopes);
 * ```
 */

import { InjectionToken, inject } from '@angular/core';

import { FlowPlanningApiService } from '../services/flow-planning-api.service';

import type {
  FlowPlan,
  FlowBudget,
  BudgetCategory,
  FlowGoal,
  FlowMilestone,
  FlowScenario,
  ScenarioChange,
  FlowProjection,
  ProjectionBreakpoint,
  RecurringPattern,
  AllocationConstraint,
  OptimizationResult,
  FlowPlanningDashboard,
  FlowPlanningInsight,
  BudgetVarianceReport,
  GoalEvaluationResult,
  GoalProjection,
  FlowHealthAnalysis,
  AnomalyDetection,
  ScenarioComparison,
  ScenarioSimulationResult,
  PlanReviewResult,
  ComplianceCheck,
  TimeHorizon,
  PlanStatus,
  GoalStatus,
  ConfidenceLevel,
} from '../models/flow-planning.model';
import type { ResourceCategory } from '../models/stewarded-resources.model';

// =============================================================================
// INTERFACE
// =============================================================================

export interface IFlowPlanning {
  // -- Plan Management --

  createPlan(
    stewardId: string,
    name: string,
    timeHorizon: TimeHorizon,
    periodStart: string,
    periodEnd: string,
    resourceScopes: ResourceCategory[],
    options?: {
      description?: string;
      goals?: Partial<FlowGoal>[];
      milestones?: Partial<FlowMilestone>[];
    }
  ): Promise<FlowPlan>;

  updatePlan(planId: string, updates: Partial<FlowPlan>): Promise<FlowPlan>;
  getPlan(planId: string): Promise<FlowPlan | null>;
  getPlansForSteward(stewardId: string, status?: PlanStatus): Promise<FlowPlan[]>;
  archivePlan(planId: string, reason: string): Promise<FlowPlan>;
  reviewPlan(planId: string): Promise<PlanReviewResult>;

  // -- Budget Management --

  createBudget(
    planId: string,
    name: string,
    budgetPeriod: 'weekly' | 'monthly' | 'quarterly' | 'annual',
    categories: BudgetCategory[],
    periodStart: string,
    periodEnd: string
  ): Promise<FlowBudget>;

  getBudget(budgetId: string): Promise<FlowBudget | null>;

  updateBudgetCategory(
    budgetId: string,
    categoryId: string,
    plannedAmount: { value: number; unit: string }
  ): Promise<FlowBudget>;

  compareBudgetToActual(budgetId: string): Promise<BudgetVarianceReport>;
  rebalanceBudget(budgetId: string, constraints: AllocationConstraint[]): Promise<FlowBudget>;
  reconcileBudget(budgetId: string): Promise<FlowBudget>;

  // -- Goal Tracking --

  createGoal(planId: string, goalDefinition: Partial<FlowGoal>): Promise<FlowGoal>;
  updateGoalProgress(goalId: string, currentValue: number): Promise<FlowGoal>;
  evaluateGoal(goalId: string): Promise<GoalEvaluationResult>;
  getGoalsForPlan(planId: string, status?: GoalStatus): Promise<FlowGoal[]>;
  linkGoalToResources(goalId: string, resourceIds: string[]): Promise<FlowGoal>;

  // -- Projection & Forecasting --

  projectFinancialHealth(
    stewardId: string,
    months: number,
    confidenceLevel?: ConfidenceLevel
  ): Promise<FlowProjection>;

  projectResourceUtilization(resourceId: string, months: number): Promise<FlowProjection>;
  projectGoalCompletion(goalId: string): Promise<GoalProjection>;

  identifyBreakpoints(
    projectionId: string,
    metric: string,
    threshold: number
  ): Promise<ProjectionBreakpoint[]>;

  extendTrendForward(
    resourceId: string,
    historicalMonths: number,
    projectionMonths: number
  ): Promise<FlowProjection>;

  // -- Scenario Simulation --

  createScenario(
    planId: string,
    name: string,
    changes: ScenarioChange[],
    scenarioType?: 'optimistic' | 'pessimistic' | 'baseline' | 'target' | 'what-if'
  ): Promise<FlowScenario>;

  runScenario(scenarioId: string): Promise<ScenarioSimulationResult>;
  compareScenarios(scenarioIds: string[]): Promise<ScenarioComparison>;

  optimizeAllocation(
    planId: string,
    constraints: AllocationConstraint[],
    objectiveFunction:
      | 'maximize-surplus'
      | 'minimize-variance'
      | 'achieve-targets'
      | 'balance-categories'
  ): Promise<OptimizationResult>;

  // -- Cadence Management --

  createRecurringPattern(
    stewardId: string,
    patternDefinition: Partial<RecurringPattern>
  ): Promise<RecurringPattern>;

  generateRecurringEvents(
    patternId: string,
    durationMonths: number
  ): Promise<{ timestamp: string; expectedAmount: number; label: string }[]>;

  calculateNextDue(pattern: RecurringPattern): Promise<string>;

  identifyPatternsFromHistory(
    stewardId: string,
    resourceCategory: ResourceCategory,
    lookbackMonths: number
  ): Promise<RecurringPattern[]>;

  updatePatternFromActual(patternId: string, actualEventId: string): Promise<RecurringPattern>;

  // -- Dashboard & Insights --

  buildFlowDashboard(stewardId: string): Promise<FlowPlanningDashboard>;
  analyzeFlowHealth(planId: string): Promise<FlowHealthAnalysis>;
  generatePlanningInsights(
    stewardId: string,
    lookbackMonths?: number
  ): Promise<FlowPlanningInsight[]>;
  detectAnomalies(
    resourceId: string,
    sensitivity?: 'low' | 'medium' | 'high'
  ): Promise<AnomalyDetection[]>;

  // -- Constitutional Compliance --

  checkPlanCompliance(planId: string): Promise<ComplianceCheck>;
}

// =============================================================================
// INJECTION TOKEN
// =============================================================================

/**
 * Injection token for flow planning.
 *
 * Override in tests:
 * ```typescript
 * { provide: FLOW_PLANNING, useValue: mockFlowPlanning }
 * ```
 */
export const FLOW_PLANNING = new InjectionToken<IFlowPlanning>('FlowPlanning', {
  providedIn: 'root',
  factory: () => inject(FlowPlanningApiService),
});
