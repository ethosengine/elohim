/**
 * Flow Planning Models - Life Cadences, Budgeting, and Scenario Simulation
 *
 * Enables humans to plan, simulate, and budget across all stewarded resources.
 * Supports all time horizons (daily → lifecycle) and multi-variable what-if analysis.
 *
 * Key Entities:
 * - FlowPlan: Top-level planning entity with goals and milestones
 * - FlowBudget: Prescriptive allocation with category-based tracking
 * - FlowGoal: Specific targets with progress tracking
 * - FlowScenario: What-if simulation with multi-variable changes
 * - FlowProjection: Time series forecasts with breakpoint detection
 * - RecurringPattern: Life rhythm modeling with auto-generation
 */

import { ResourceMeasure, ResourceCategory } from './stewarded-resources.model';

// =============================================================================
// Time Horizons and Frequencies
// =============================================================================

export type TimeHorizon =
  | 'daily' // Day-to-day planning
  | 'weekly' // Week-by-week
  | 'monthly' // Monthly planning
  | 'quarterly' // Quarterly goals
  | 'annual' // Yearly planning
  | 'multi-year' // 3-5 year horizon
  | 'lifecycle'; // Major life milestones

export type Frequency =
  | 'daily'
  | 'weekly'
  | 'biweekly'
  | 'monthly'
  | 'quarterly'
  | 'semi-annual'
  | 'annual'
  | 'irregular'
  | 'one-time';

export type PlanStatus = 'draft' | 'active' | 'on-track' | 'at-risk' | 'completed' | 'archived';
export type BudgetStatus = 'draft' | 'active' | 'under-budget' | 'over-budget' | 'closed';
export type GoalStatus =
  | 'draft'
  | 'active'
  | 'on-track'
  | 'behind'
  | 'at-risk'
  | 'completed'
  | 'abandoned';
export type ScenarioStatus = 'draft' | 'simulated' | 'analyzing' | 'selected' | 'archived';
export type ScenarioType = 'optimistic' | 'pessimistic' | 'baseline' | 'target' | 'what-if';
export type MilestoneStatus = 'pending' | 'in-progress' | 'achieved' | 'missed' | 'deferred';
export type ProjectionMethod =
  | 'trend-extrapolation'
  | 'pattern-based'
  | 'constraint-optimized'
  | 'scenario-driven';
export type BreakpointType =
  | 'zero-crossing'
  | 'threshold-breach'
  | 'trend-reversal'
  | 'milestone-achievement';
export type BreakpointSeverity = 'info' | 'warning' | 'critical';
export type ConfidenceLevel = 'low' | 'medium' | 'high';

// =============================================================================
// Recurring Pattern Engine - Life Cadence Modeling
// =============================================================================

/**
 * RecurringPattern - Explicit scheduling model for life's rhythms
 *
 * Generates expected future events automatically.
 * Supports "every N periods" patterns (e.g., every 2 weeks, every 3 months).
 */
export interface RecurringPattern {
  id: string;
  patternNumber: string; // RP-XXXXXXXXXX
  stewardId: string;

  // Pattern identity
  label: string; // "Monthly rent payment"
  description?: string;

  // Frequency specification
  frequency: Frequency;
  frequencyValue?: number; // For "every N" patterns (e.g., every 2 weeks)

  // Amount specification
  expectedAmount: ResourceMeasure;
  varianceExpected: number; // Percentage variance (0-100)

  // Schedule
  startDate: string; // ISO 8601, when pattern begins
  endDate?: string; // ISO 8601, when pattern ends (if known)
  nextDueDate: string; // Auto-calculated

  // Pattern metadata
  resourceCategory: ResourceCategory;
  patternType: 'income' | 'expense' | 'allocation' | 'event';
  autoGenerate: boolean; // Auto-create expected events?

  // Historical tracking
  historicalOccurrences: string[]; // Event IDs of past occurrences
  missedOccurrences: number; // How many were missed/skipped
  averageActualAmount?: ResourceMeasure; // Actual average from history
  reliability: number; // 0-100, how consistently it occurs

  // Status
  status: 'active' | 'paused' | 'ended';

  // Metadata
  createdAt: string;
  updatedAt: string;
  metadata?: Record<string, unknown>;
}

// =============================================================================
// Flow Plan - Top-level Planning Entity
// =============================================================================

/**
 * FlowPlan - Top-level planning entity
 *
 * Defines scope of planning effort:
 * - Time horizon (daily to lifecycle)
 * - Resource coverage
 * - Goals and milestones
 * - Budget allocations
 */
export interface FlowPlan {
  id: string;
  planNumber: string; // FP-XXXXXXXXXX
  stewardId: string;

  // Plan scope
  name: string; // "2025 Financial Stability Plan"
  description?: string;
  timeHorizon: TimeHorizon;
  planPeriodStart: string; // ISO 8601
  planPeriodEnd: string; // ISO 8601

  // Resource coverage
  resourceScopes: ResourceCategory[]; // Which resources are included
  includedResourceIds: string[]; // Specific resources tracked

  // Goals & milestones
  goals: string[]; // FlowGoal IDs
  milestones: string[]; // FlowMilestone IDs

  // Budget allocations
  budgets: string[]; // FlowBudget IDs

  // Status
  status: PlanStatus;
  confidenceScore: number; // 0-100, confidence in achieving goals
  completionPercent: number; // How much of plan is executed

  // Tracking
  createdAt: string;
  activatedAt?: string;
  completedAt?: string;
  lastReviewedAt?: string;
  nextReviewDue: string;

  // Events
  planEventIds: string[]; // EconomicEvent IDs for plan lifecycle

  metadata?: Record<string, unknown>;
}

// =============================================================================
// Flow Budget - Prescriptive Allocation
// =============================================================================

/**
 * FlowBudget - Prescriptive allocation across resource categories
 *
 * Enables:
 * - Budget vs actual tracking
 * - Category-based expense management
 * - Allocation rebalancing
 * - Variance analysis
 *
 * Note: Different from AllocationBlock (in StewardedResource) which is descriptive.
 * FlowBudget is prescriptive (what we plan to spend).
 */
export interface FlowBudget {
  id: string;
  budgetNumber: string; // FB-XXXXXXXXXX
  planId: string; // Parent FlowPlan
  stewardId: string;

  // Budget identity
  name: string; // "Monthly Household Budget"
  description?: string;
  budgetPeriod: 'weekly' | 'monthly' | 'quarterly' | 'annual';

  // Period tracking
  periodStart: string; // ISO 8601
  periodEnd: string; // ISO 8601

  // Categories and allocations
  categories: BudgetCategory[];
  totalPlanned: ResourceMeasure;
  totalActual: ResourceMeasure;
  variance: number; // Difference: actual - planned
  variancePercent: number; // (variance / planned) * 100

  // Status
  status: BudgetStatus;
  healthStatus: 'healthy' | 'warning' | 'critical';

  // Tracking
  createdAt: string;
  updatedAt: string;
  lastReconciled: string; // When was actual vs planned last calculated

  budgetEventIds: string[]; // Economic events linked to budget
}

export interface BudgetCategory {
  id: string;
  categoryName: string; // "Housing", "Food", "Learning", "Community Service"
  categoryType: 'fixed' | 'variable' | 'discretionary';

  // Allocations
  plannedAmount: ResourceMeasure;
  actualAmount: ResourceMeasure;
  variance: number;
  variancePercent: number;

  // Sub-categories
  subcategories?: BudgetCategory[];

  // Limits
  hardLimit?: ResourceMeasure; // Cannot exceed
  targetAmount?: ResourceMeasure; // Ideal spending
  minimumAmount?: ResourceMeasure; // Cannot go below

  // Tracking
  transactionIds: string[]; // Actual transactions in this category
  recurringPatternIds: string[]; // Recurring patterns contributing

  notes?: string;
}

// =============================================================================
// Flow Goal - Specific Targets
// =============================================================================

/**
 * FlowGoal - Specific target to achieve
 *
 * Examples:
 * - "Build 6-month emergency fund"
 * - "Pay off credit card debt"
 * - "Increase sustainable income"
 * - "Allocate 10% of time to community service"
 */
export interface FlowGoal {
  id: string;
  goalNumber: string; // FG-XXXXXXXXXX
  planId: string;
  stewardId: string;

  // Goal definition
  name: string; // "Build 6-month emergency fund"
  description?: string;
  goalType:
    | 'savings'
    | 'debt-reduction'
    | 'income-increase'
    | 'allocation-shift'
    | 'milestone'
    | 'custom';

  // Target
  targetMetric: string; // "emergency_fund_months", "debt_paid_down", etc.
  targetValue: number;
  targetUnit: string;
  currentValue: number;
  startingValue: number;

  // Timeline
  deadline: string; // ISO 8601
  createdAt: string;
  startedAt?: string;
  completedAt?: string;

  // Progress
  progressPercent: number; // (current - starting) / (target - starting) * 100
  onTrack: boolean;
  estimatedCompletionDate?: string; // ISO 8601

  // Dependencies
  linkedResourceIds: string[]; // Resources this goal depends on
  linkedBudgetIds: string[]; // Budgets supporting this goal
  blockedBy?: string[]; // Other goals blocking this

  // Status
  status: GoalStatus;

  goalEventIds: string[];
  metadata?: Record<string, unknown>;
}

// =============================================================================
// Flow Milestone - Key Checkpoints
// =============================================================================

/**
 * FlowMilestone - Key checkpoint in plan execution
 *
 * Example:
 * - "Reach dignity floor income"
 * - "Establish emergency fund"
 * - "Complete debt consolidation"
 */
export interface FlowMilestone {
  id: string;
  milestoneNumber: string; // FM-XXXXXXXXXX
  planId: string;

  // Milestone definition
  name: string; // "Reach dignity floor income"
  description?: string;

  // Checkpoint
  targetDate: string; // ISO 8601
  actualDate?: string; // ISO 8601

  // Success criteria
  successCriteria: MilestoneSuccessCriterion[];
  allCriteriaMet: boolean;

  // Dependencies
  dependsOnGoals: string[]; // FlowGoal IDs
  dependsOnMilestones: string[]; // Other milestone IDs
  blocksGoals: string[]; // Goals waiting for this

  // Status
  status: MilestoneStatus;

  achievedAt?: string; // ISO 8601
  milestoneEventIds: string[];
  metadata?: Record<string, unknown>;
}

export interface MilestoneSuccessCriterion {
  id: string;
  criterionName: string; // "Monthly income >= $4,000"
  metric: string;
  operator: 'eq' | 'ne' | 'gt' | 'gte' | 'lt' | 'lte';
  targetValue: number;
  currentValue: number;
  met: boolean;
}

// =============================================================================
// Flow Scenario - What-If Simulation
// =============================================================================

/**
 * FlowScenario - Multi-variable simulation
 *
 * Enables what-if analysis:
 * - "What if income increases 20%?"
 * - "What if rent drops $500?"
 * - "What if I allocate 10 more hours/week to learning?"
 */
export interface FlowScenario {
  id: string;
  scenarioNumber: string; // FS-XXXXXXXXXX
  planId: string;
  stewardId: string;

  // Scenario definition
  name: string; // "20% Income Increase Scenario"
  description?: string;
  scenarioType: ScenarioType;

  // Variable changes
  changes: ScenarioChange[];

  // Projection results
  projections: string[]; // FlowProjection IDs

  // Comparison
  baselineScenarioId?: string; // Compare against this
  deltaMetrics: Record<string, number>; // Key differences from baseline

  // Status
  status: ScenarioStatus;
  simulatedAt?: string; // ISO 8601

  createdAt: string;
  updatedAt: string;

  scenarioEventIds: string[];
  metadata?: Record<string, unknown>;
}

export interface ScenarioChange {
  id: string;
  changeType: 'income' | 'expense' | 'allocation' | 'resource-capacity' | 'frequency' | 'rate';

  // Target of change
  targetResourceId?: string;
  targetBudgetCategoryId?: string;
  targetRecurringPatternId?: string;

  // Change specification
  changeOperator: 'add' | 'subtract' | 'multiply' | 'set' | 'percent-increase' | 'percent-decrease';
  changeValue: number;
  changeUnit?: string;

  // Description
  label: string; // "Increase salary by 20%"
  rationale?: string;

  // Timing
  effectiveFrom: string; // ISO 8601
  effectiveUntil?: string; // ISO 8601
}

// =============================================================================
// Flow Projection - Forward-Looking Forecast
// =============================================================================

/**
 * FlowProjection - Time series forecast
 *
 * Projects resource states forward based on:
 * - Current trends
 * - Recurring patterns
 * - Scenario changes
 * - Constraint boundaries
 */
export interface FlowProjection {
  id: string;
  projectionNumber: string; // FP-XXXXXXXXXX
  planId?: string;
  scenarioId?: string; // If from a scenario simulation
  stewardId: string;

  // Projection scope
  resourceCategory: ResourceCategory;
  resourceId?: string; // Specific resource (optional)

  // Time series
  projectionStart: string; // ISO 8601
  projectionEnd: string; // ISO 8601
  projectionHorizon: TimeHorizon;
  dataPoints: ProjectionDataPoint[];

  // Confidence
  confidenceLevel: ConfidenceLevel;
  confidencePercent: number; // 0-100

  // Methodology
  projectionMethod: ProjectionMethod;
  assumptionsMade: string[]; // List of assumptions

  // Breakpoints
  breakpoints: ProjectionBreakpoint[]; // Key inflection points

  createdAt: string;
  metadata?: Record<string, unknown>;
}

export interface ProjectionDataPoint {
  timestamp: string; // ISO 8601 point in time
  projectedValue: number;
  unit: string;

  // Confidence bands
  lowEstimate?: number; // Pessimistic bound
  highEstimate?: number; // Optimistic bound

  // Contributing factors
  trendContribution?: number;
  patternContribution?: number;
  scenarioContribution?: number;
}

export interface ProjectionBreakpoint {
  timestamp: string; // ISO 8601
  breakpointType: BreakpointType;
  description: string; // "Emergency fund depleted"
  severity: BreakpointSeverity;
  relatedGoalId?: string;
  relatedMilestoneId?: string;
}

// =============================================================================
// Constraint Solving & Optimization
// =============================================================================

/**
 * AllocationConstraint - Boundaries for optimization
 *
 * Used in constraint-solving algorithms to find optimal allocations.
 * Examples:
 * - Housing: min $500, max $1500
 * - Community service: at least 10% of time
 * - Financial: cannot exceed constitutional ceiling
 */
export interface AllocationConstraint {
  id: string;
  constraintType: 'minimum' | 'maximum' | 'range' | 'ratio' | 'constitutional';

  // Target
  targetResourceCategory: ResourceCategory;
  targetBudgetCategory?: string;

  // Bounds
  minimumValue?: number;
  maximumValue?: number;
  targetValue?: number;

  // Relationships
  relativeToCategory?: ResourceCategory;
  ratioOperator?: 'eq' | 'gt' | 'gte' | 'lt' | 'lte';
  ratioValue?: number; // e.g., "time:community >= 0.15" (15% of time)

  // Priority
  priority: 'required' | 'preferred' | 'optional';
  weight: number; // For optimization (1-10)

  // Constitutional basis
  constitutionalLimitId?: string; // Link to ConstitutionalLimit

  description: string;
  rationale?: string;
}

export interface OptimizationResult {
  id: string;
  optimizationId: string;

  // Input
  constraints: AllocationConstraint[];
  objectiveFunction:
    | 'maximize-surplus'
    | 'minimize-variance'
    | 'achieve-targets'
    | 'balance-categories';

  // Output
  optimalAllocations: Record<string, number>; // category -> amount
  objectiveValue: number; // Achieved objective score
  constraintsSatisfied: number;
  constraintsViolated: number;

  // Solution quality
  feasible: boolean;
  optimal: boolean;
  solutionQuality: 'excellent' | 'good' | 'acceptable' | 'poor' | 'infeasible';

  // Recommendations
  suggestedChanges: ScenarioChange[];

  computedAt: string;
  metadata?: Record<string, unknown>;
}

// =============================================================================
// Dashboard & Insights
// =============================================================================

export interface FlowPlanningDashboard {
  stewardId: string;

  // Active plans
  activePlans: FlowPlan[];
  upcomingMilestones: FlowMilestone[];
  atRiskGoals: FlowGoal[];

  // Budget health
  budgetSummary: {
    totalMonthlyBudget: number;
    totalMonthlyActual: number;
    overBudgetCategories: number;
    healthStatus: 'healthy' | 'warning' | 'critical';
  };

  // Projections
  keyProjections: FlowProjection[];
  breakpointsInNext90Days: ProjectionBreakpoint[];

  // Insights
  insights: FlowPlanningInsight[];
  recommendations: FlowPlanningRecommendation[];

  lastUpdatedAt: string;
}

export interface FlowPlanningInsight {
  id: string;
  insightType: 'pattern-detected' | 'anomaly' | 'opportunity' | 'risk' | 'trend';
  title: string;
  description: string;
  confidence: number;
  basedOnPeriod: string;
  relatedPlanId?: string;
  relatedGoalId?: string;
  actionable: boolean;
}

export interface FlowPlanningRecommendation {
  id: string;
  priority: 'critical' | 'high' | 'medium' | 'low';
  recommendationType:
    | 'budget-adjustment'
    | 'goal-revision'
    | 'allocation-shift'
    | 'pattern-optimization';
  title: string;
  description: string;
  suggestedAction: string;
  estimatedImpact: string;
  relatedPlanId?: string;
  relatedBudgetId?: string;
  relatedGoalId?: string;
}

// =============================================================================
// Analysis & Reporting
// =============================================================================

export interface BudgetVarianceReport {
  budgetId: string;
  periodStart: string;
  periodEnd: string;

  // Overall variance
  totalPlanned: number;
  totalActual: number;
  totalVariance: number;
  totalVariancePercent: number;

  // By category
  categoryVariances: CategoryVariance[];

  // Status
  overBudgetCategories: string[];
  underBudgetCategories: string[];

  // Recommendations
  recommendations: string[];

  generatedAt: string;
}

export interface CategoryVariance {
  categoryName: string;
  plannedAmount: number;
  actualAmount: number;
  variance: number;
  variancePercent: number;
  status: 'under-budget' | 'on-budget' | 'over-budget';
}

export interface GoalEvaluationResult {
  goalId: string;
  onTrack: boolean;
  progressPercent: number;
  estimatedCompletionDate?: string;
  daysToDeadline: number;
  riskLevel: 'low' | 'medium' | 'high';
  recommendation: string;
}

export interface GoalProjection {
  goalId: string;
  currentValue: number;
  targetValue: number;
  projectedCompletionDate?: string;
  completionLikelihood: number; // 0-100
  variableFactors: string[]; // What affects completion
}

export interface FlowHealthAnalysis {
  planId: string;

  // Overall health
  healthStatus: 'healthy' | 'warning' | 'critical';
  completionPercent: number;
  onTrackGoals: number;
  atRiskGoals: number;

  // Budget health
  budgetUtilization: number;
  overBudgetCategories: number;

  // Timeline health
  milestonesAchieved: number;
  milestonesMissed: number;
  upcomingMilestones: number;

  // Risks
  identifiedRisks: string[];
  mitigation: string[];

  analysis: string;
}

export interface AnomalyDetection {
  resourceId: string;
  anomalyType: string;
  description: string;
  confidence: number;
  historicalNormal: number;
  currentValue: number;
  deviation: number;
  severity: 'low' | 'medium' | 'high';
}

export interface ScenarioComparison {
  scenarios: {
    scenarioId: string;
    name: string;
    type: ScenarioType;
  }[];

  // Metric comparisons
  metrics: {
    metricName: string;
    values: Record<string, number>; // scenario ID -> value
    bestOption: string;
    worstOption: string;
  }[];

  // Recommendation
  recommendedScenario: string;
  rationale: string;
}

export interface ScenarioSimulationResult {
  scenarioId: string;
  baselineScenarioId: string;

  // What changed
  appliedChanges: ScenarioChange[];

  // Results
  projections: FlowProjection[];

  // Impact
  deltaMetrics: Record<string, number>; // metric name -> change value
  affectedGoals: string[]; // Goal IDs affected
  affectedMilestones: string[]; // Milestone IDs affected

  // Feasibility
  feasible: boolean;
  concerns: string[];

  simulatedAt: string;
}

export interface PlanReviewResult {
  planId: string;
  reviewDate: string;

  // Status
  stillRelevant: boolean;
  needsAdjustment: boolean;

  // Progress
  progressMade: string[];
  blockers: string[];

  // Recommendations
  suggestedAdjustments: string[];
  nextReviewDate: string;
}

export interface ComplianceCheck {
  planId: string;
  compliant: boolean;

  // Violations
  violations: ConstitutionalViolation[];

  // Transition paths
  transitionPathsNeeded: ConstitutionalViolation[];
}

export interface ConstitutionalViolation {
  resourceId: string;
  category: ResourceCategory;
  currentValue: number;
  constitutionalCeiling: number;
  excess: number;
}
