/**
 * FlowPlanningApiService — Thin HTTP client for flow planning operations.
 *
 * Calls doorway `/api/v1/flow-planning/*` endpoints, implementing
 * IFlowPlanning. Replaces the fat FlowPlanningService when the business
 * logic lives behind the Rust API boundary.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import type { IFlowPlanning } from '../interfaces/flow-planning.interface';
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

const BASE = '/api/v1/flow-planning';

@Injectable({ providedIn: 'root' })
export class FlowPlanningApiService implements IFlowPlanning {
  private readonly http = inject(HttpClient);

  // =========================================================================
  // Plan Management
  // =========================================================================

  async createPlan(
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
  ): Promise<FlowPlan> {
    return firstValueFrom(
      this.http.post<FlowPlan>(`${BASE}/plans`, {
        stewardId,
        name,
        timeHorizon,
        periodStart,
        periodEnd,
        resourceScopes,
        ...options,
      })
    );
  }

  async updatePlan(planId: string, updates: Partial<FlowPlan>): Promise<FlowPlan> {
    return firstValueFrom(this.http.patch<FlowPlan>(`${BASE}/plans/${planId}`, updates));
  }

  async getPlan(planId: string): Promise<FlowPlan | null> {
    return firstValueFrom(this.http.get<FlowPlan | null>(`${BASE}/plans/${planId}`));
  }

  async getPlansForSteward(stewardId: string, status?: PlanStatus): Promise<FlowPlan[]> {
    const params: Record<string, string> = { stewardId };
    if (status) params['status'] = status;
    return firstValueFrom(this.http.get<FlowPlan[]>(`${BASE}/plans`, { params }));
  }

  async archivePlan(planId: string, reason: string): Promise<FlowPlan> {
    return firstValueFrom(
      this.http.post<FlowPlan>(`${BASE}/plans/${planId}/archive`, { reason })
    );
  }

  async reviewPlan(planId: string): Promise<PlanReviewResult> {
    return firstValueFrom(
      this.http.get<PlanReviewResult>(`${BASE}/plans/${planId}/review`)
    );
  }

  // =========================================================================
  // Budget Management
  // =========================================================================

  async createBudget(
    planId: string,
    name: string,
    budgetPeriod: 'weekly' | 'monthly' | 'quarterly' | 'annual',
    categories: BudgetCategory[],
    periodStart: string,
    periodEnd: string
  ): Promise<FlowBudget> {
    return firstValueFrom(
      this.http.post<FlowBudget>(`${BASE}/budgets`, {
        planId,
        name,
        budgetPeriod,
        categories,
        periodStart,
        periodEnd,
      })
    );
  }

  async getBudget(budgetId: string): Promise<FlowBudget | null> {
    return firstValueFrom(this.http.get<FlowBudget | null>(`${BASE}/budgets/${budgetId}`));
  }

  async updateBudgetCategory(
    budgetId: string,
    categoryId: string,
    plannedAmount: { value: number; unit: string }
  ): Promise<FlowBudget> {
    return firstValueFrom(
      this.http.patch<FlowBudget>(`${BASE}/budgets/${budgetId}/categories/${categoryId}`, {
        plannedAmount,
      })
    );
  }

  async compareBudgetToActual(budgetId: string): Promise<BudgetVarianceReport> {
    return firstValueFrom(
      this.http.get<BudgetVarianceReport>(`${BASE}/budgets/${budgetId}/variance`)
    );
  }

  async rebalanceBudget(
    budgetId: string,
    constraints: AllocationConstraint[]
  ): Promise<FlowBudget> {
    return firstValueFrom(
      this.http.post<FlowBudget>(`${BASE}/budgets/${budgetId}/rebalance`, { constraints })
    );
  }

  async reconcileBudget(budgetId: string): Promise<FlowBudget> {
    return firstValueFrom(
      this.http.post<FlowBudget>(`${BASE}/budgets/${budgetId}/reconcile`, {})
    );
  }

  // =========================================================================
  // Goal Tracking
  // =========================================================================

  async createGoal(planId: string, goalDefinition: Partial<FlowGoal>): Promise<FlowGoal> {
    return firstValueFrom(
      this.http.post<FlowGoal>(`${BASE}/goals`, { planId, ...goalDefinition })
    );
  }

  async updateGoalProgress(goalId: string, currentValue: number): Promise<FlowGoal> {
    return firstValueFrom(
      this.http.patch<FlowGoal>(`${BASE}/goals/${goalId}/progress`, { currentValue })
    );
  }

  async evaluateGoal(goalId: string): Promise<GoalEvaluationResult> {
    return firstValueFrom(
      this.http.get<GoalEvaluationResult>(`${BASE}/goals/${goalId}/evaluate`)
    );
  }

  async getGoalsForPlan(planId: string, status?: GoalStatus): Promise<FlowGoal[]> {
    const params: Record<string, string> = { planId };
    if (status) params['status'] = status;
    return firstValueFrom(this.http.get<FlowGoal[]>(`${BASE}/goals`, { params }));
  }

  async linkGoalToResources(goalId: string, resourceIds: string[]): Promise<FlowGoal> {
    return firstValueFrom(
      this.http.post<FlowGoal>(`${BASE}/goals/${goalId}/resources`, { resourceIds })
    );
  }

  // =========================================================================
  // Projection & Forecasting
  // =========================================================================

  async projectFinancialHealth(
    stewardId: string,
    months: number,
    confidenceLevel?: ConfidenceLevel
  ): Promise<FlowProjection> {
    const params: Record<string, string> = { stewardId, months: String(months) };
    if (confidenceLevel) params['confidenceLevel'] = confidenceLevel;
    return firstValueFrom(
      this.http.get<FlowProjection>(`${BASE}/projections/financial-health`, { params })
    );
  }

  async projectResourceUtilization(
    resourceId: string,
    months: number
  ): Promise<FlowProjection> {
    return firstValueFrom(
      this.http.get<FlowProjection>(`${BASE}/projections/resource-utilization`, {
        params: { resourceId, months: String(months) },
      })
    );
  }

  async projectGoalCompletion(goalId: string): Promise<GoalProjection> {
    return firstValueFrom(
      this.http.get<GoalProjection>(`${BASE}/projections/goal-completion`, {
        params: { goalId },
      })
    );
  }

  async identifyBreakpoints(
    projectionId: string,
    metric: string,
    threshold: number
  ): Promise<ProjectionBreakpoint[]> {
    return firstValueFrom(
      this.http.get<ProjectionBreakpoint[]>(
        `${BASE}/projections/${projectionId}/breakpoints`,
        { params: { metric, threshold: String(threshold) } }
      )
    );
  }

  async extendTrendForward(
    resourceId: string,
    historicalMonths: number,
    projectionMonths: number
  ): Promise<FlowProjection> {
    return firstValueFrom(
      this.http.get<FlowProjection>(`${BASE}/projections/trend`, {
        params: {
          resourceId,
          historicalMonths: String(historicalMonths),
          projectionMonths: String(projectionMonths),
        },
      })
    );
  }

  // =========================================================================
  // Scenario Simulation
  // =========================================================================

  async createScenario(
    planId: string,
    name: string,
    changes: ScenarioChange[],
    scenarioType?: 'optimistic' | 'pessimistic' | 'baseline' | 'target' | 'what-if'
  ): Promise<FlowScenario> {
    return firstValueFrom(
      this.http.post<FlowScenario>(`${BASE}/scenarios`, {
        planId,
        name,
        changes,
        scenarioType,
      })
    );
  }

  async runScenario(scenarioId: string): Promise<ScenarioSimulationResult> {
    return firstValueFrom(
      this.http.post<ScenarioSimulationResult>(
        `${BASE}/scenarios/${scenarioId}/run`,
        {}
      )
    );
  }

  async compareScenarios(scenarioIds: string[]): Promise<ScenarioComparison> {
    return firstValueFrom(
      this.http.post<ScenarioComparison>(`${BASE}/scenarios/compare`, { scenarioIds })
    );
  }

  async optimizeAllocation(
    planId: string,
    constraints: AllocationConstraint[],
    objectiveFunction:
      | 'maximize-surplus'
      | 'minimize-variance'
      | 'achieve-targets'
      | 'balance-categories'
  ): Promise<OptimizationResult> {
    return firstValueFrom(
      this.http.post<OptimizationResult>(`${BASE}/plans/${planId}/optimize`, {
        constraints,
        objectiveFunction,
      })
    );
  }

  // =========================================================================
  // Cadence Management
  // =========================================================================

  async createRecurringPattern(
    stewardId: string,
    patternDefinition: Partial<RecurringPattern>
  ): Promise<RecurringPattern> {
    return firstValueFrom(
      this.http.post<RecurringPattern>(`${BASE}/patterns`, {
        stewardId,
        ...patternDefinition,
      })
    );
  }

  async generateRecurringEvents(
    patternId: string,
    durationMonths: number
  ): Promise<{ timestamp: string; expectedAmount: number; label: string }[]> {
    return firstValueFrom(
      this.http.get<{ timestamp: string; expectedAmount: number; label: string }[]>(
        `${BASE}/patterns/${patternId}/events`,
        { params: { durationMonths: String(durationMonths) } }
      )
    );
  }

  async calculateNextDue(pattern: RecurringPattern): Promise<string> {
    return firstValueFrom(
      this.http.post<string>(`${BASE}/patterns/next-due`, pattern)
    );
  }

  async identifyPatternsFromHistory(
    stewardId: string,
    resourceCategory: ResourceCategory,
    lookbackMonths: number
  ): Promise<RecurringPattern[]> {
    return firstValueFrom(
      this.http.get<RecurringPattern[]>(`${BASE}/patterns/detect`, {
        params: {
          stewardId,
          resourceCategory,
          lookbackMonths: String(lookbackMonths),
        },
      })
    );
  }

  async updatePatternFromActual(
    patternId: string,
    actualEventId: string
  ): Promise<RecurringPattern> {
    return firstValueFrom(
      this.http.post<RecurringPattern>(`${BASE}/patterns/${patternId}/actual`, {
        actualEventId,
      })
    );
  }

  // =========================================================================
  // Dashboard & Insights
  // =========================================================================

  async buildFlowDashboard(stewardId: string): Promise<FlowPlanningDashboard> {
    return firstValueFrom(
      this.http.get<FlowPlanningDashboard>(`${BASE}/dashboard`, {
        params: { stewardId },
      })
    );
  }

  async analyzeFlowHealth(planId: string): Promise<FlowHealthAnalysis> {
    return firstValueFrom(
      this.http.get<FlowHealthAnalysis>(`${BASE}/plans/${planId}/health`)
    );
  }

  async generatePlanningInsights(
    stewardId: string,
    lookbackMonths?: number
  ): Promise<FlowPlanningInsight[]> {
    const params: Record<string, string> = { stewardId };
    if (lookbackMonths != null) params['lookbackMonths'] = String(lookbackMonths);
    return firstValueFrom(
      this.http.get<FlowPlanningInsight[]>(`${BASE}/insights`, { params })
    );
  }

  async detectAnomalies(
    resourceId: string,
    sensitivity?: 'low' | 'medium' | 'high'
  ): Promise<AnomalyDetection[]> {
    const params: Record<string, string> = { resourceId };
    if (sensitivity) params['sensitivity'] = sensitivity;
    return firstValueFrom(
      this.http.get<AnomalyDetection[]>(`${BASE}/anomalies`, { params })
    );
  }

  // =========================================================================
  // Constitutional Compliance
  // =========================================================================

  async checkPlanCompliance(planId: string): Promise<ComplianceCheck> {
    return firstValueFrom(
      this.http.get<ComplianceCheck>(`${BASE}/plans/${planId}/compliance`)
    );
  }
}
