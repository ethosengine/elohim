import { TestBed } from '@angular/core/testing';
import {
  HttpTestingController,
  provideHttpClientTesting,
} from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';

import { FlowPlanningApiService } from './flow-planning-api.service';

describe('FlowPlanningApiService', () => {
  let service: FlowPlanningApiService;
  let httpMock: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [FlowPlanningApiService, provideHttpClient(), provideHttpClientTesting()],
    });
    service = TestBed.inject(FlowPlanningApiService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => httpMock.verify());

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // =========================================================================
  // Plan Management
  // =========================================================================

  describe('plan management', () => {
    it('createPlan POSTs to /api/v1/flow-planning/plans', async () => {
      const stub = { id: 'plan-1' };
      const p = service.createPlan('s1', 'Plan', 'monthly', '2024-01-01', '2024-12-31', [
        'financial-asset',
      ]);
      const req = httpMock.expectOne('/api/v1/flow-planning/plans');
      expect(req.request.method).toBe('POST');
      req.flush(stub);
      expect(await p).toEqual(stub);
    });

    it('updatePlan PATCHes to /api/v1/flow-planning/plans/:id', async () => {
      const stub = { id: 'plan-1' };
      const p = service.updatePlan('plan-1', { name: 'Updated' } as any);
      const req = httpMock.expectOne('/api/v1/flow-planning/plans/plan-1');
      expect(req.request.method).toBe('PATCH');
      req.flush(stub);
      expect(await p).toEqual(stub);
    });

    it('getPlan GETs /api/v1/flow-planning/plans/:id', async () => {
      const stub = { id: 'plan-1' };
      const p = service.getPlan('plan-1');
      const req = httpMock.expectOne('/api/v1/flow-planning/plans/plan-1');
      expect(req.request.method).toBe('GET');
      req.flush(stub);
      expect(await p).toEqual(stub);
    });

    it('getPlansForSteward GETs with stewardId param', async () => {
      const p = service.getPlansForSteward('s1');
      const req = httpMock.expectOne(
        (r) => r.url === '/api/v1/flow-planning/plans' && r.params.get('stewardId') === 's1'
      );
      expect(req.request.method).toBe('GET');
      req.flush([]);
      expect(await p).toEqual([]);
    });

    it('archivePlan POSTs to /api/v1/flow-planning/plans/:id/archive', async () => {
      const stub = { id: 'plan-1' };
      const p = service.archivePlan('plan-1', 'done');
      const req = httpMock.expectOne('/api/v1/flow-planning/plans/plan-1/archive');
      expect(req.request.method).toBe('POST');
      expect(req.request.body).toEqual({ reason: 'done' });
      req.flush(stub);
      expect(await p).toEqual(stub);
    });

    it('reviewPlan GETs /api/v1/flow-planning/plans/:id/review', async () => {
      const stub = { status: 'healthy' };
      const p = service.reviewPlan('plan-1');
      const req = httpMock.expectOne('/api/v1/flow-planning/plans/plan-1/review');
      expect(req.request.method).toBe('GET');
      req.flush(stub);
      expect(await p).toEqual(stub);
    });
  });

  // =========================================================================
  // Budget Management
  // =========================================================================

  describe('budget management', () => {
    it('createBudget POSTs to /api/v1/flow-planning/budgets', async () => {
      const stub = { id: 'b-1' };
      const p = service.createBudget('plan-1', 'Budget', 'monthly', [], '2024-01-01', '2024-01-31');
      const req = httpMock.expectOne('/api/v1/flow-planning/budgets');
      expect(req.request.method).toBe('POST');
      req.flush(stub);
      expect(await p).toEqual(stub);
    });

    it('getBudget GETs /api/v1/flow-planning/budgets/:id', async () => {
      const p = service.getBudget('b-1');
      const req = httpMock.expectOne('/api/v1/flow-planning/budgets/b-1');
      expect(req.request.method).toBe('GET');
      req.flush(null);
      expect(await p).toBeNull();
    });

    it('updateBudgetCategory PATCHes category', async () => {
      const stub = { id: 'b-1' };
      const p = service.updateBudgetCategory('b-1', 'cat-1', { value: 100, unit: 'USD' });
      const req = httpMock.expectOne('/api/v1/flow-planning/budgets/b-1/categories/cat-1');
      expect(req.request.method).toBe('PATCH');
      req.flush(stub);
      expect(await p).toEqual(stub);
    });

    it('compareBudgetToActual GETs variance', async () => {
      const stub = { totalVariance: 0 };
      const p = service.compareBudgetToActual('b-1');
      const req = httpMock.expectOne('/api/v1/flow-planning/budgets/b-1/variance');
      expect(req.request.method).toBe('GET');
      req.flush(stub);
      expect(await p).toEqual(stub);
    });

    it('rebalanceBudget POSTs to rebalance', async () => {
      const stub = { id: 'b-1' };
      const p = service.rebalanceBudget('b-1', []);
      const req = httpMock.expectOne('/api/v1/flow-planning/budgets/b-1/rebalance');
      expect(req.request.method).toBe('POST');
      req.flush(stub);
      expect(await p).toEqual(stub);
    });

    it('reconcileBudget POSTs to reconcile', async () => {
      const stub = { id: 'b-1' };
      const p = service.reconcileBudget('b-1');
      const req = httpMock.expectOne('/api/v1/flow-planning/budgets/b-1/reconcile');
      expect(req.request.method).toBe('POST');
      req.flush(stub);
      expect(await p).toEqual(stub);
    });
  });

  // =========================================================================
  // Goal Tracking
  // =========================================================================

  describe('goal tracking', () => {
    it('createGoal POSTs to /api/v1/flow-planning/goals', async () => {
      const stub = { id: 'g-1' };
      const p = service.createGoal('plan-1', { name: 'Save' } as any);
      const req = httpMock.expectOne('/api/v1/flow-planning/goals');
      expect(req.request.method).toBe('POST');
      req.flush(stub);
      expect(await p).toEqual(stub);
    });

    it('updateGoalProgress PATCHes progress', async () => {
      const stub = { id: 'g-1' };
      const p = service.updateGoalProgress('g-1', 50);
      const req = httpMock.expectOne('/api/v1/flow-planning/goals/g-1/progress');
      expect(req.request.method).toBe('PATCH');
      req.flush(stub);
      expect(await p).toEqual(stub);
    });

    it('evaluateGoal GETs evaluation', async () => {
      const stub = { status: 'on-track' };
      const p = service.evaluateGoal('g-1');
      const req = httpMock.expectOne('/api/v1/flow-planning/goals/g-1/evaluate');
      expect(req.request.method).toBe('GET');
      req.flush(stub);
      expect(await p).toEqual(stub);
    });

    it('getGoalsForPlan GETs with planId param', async () => {
      const p = service.getGoalsForPlan('plan-1');
      const req = httpMock.expectOne(
        (r) => r.url === '/api/v1/flow-planning/goals' && r.params.get('planId') === 'plan-1'
      );
      req.flush([]);
      expect(await p).toEqual([]);
    });

    it('linkGoalToResources POSTs resource links', async () => {
      const stub = { id: 'g-1' };
      const p = service.linkGoalToResources('g-1', ['r-1', 'r-2']);
      const req = httpMock.expectOne('/api/v1/flow-planning/goals/g-1/resources');
      expect(req.request.method).toBe('POST');
      expect(req.request.body).toEqual({ resourceIds: ['r-1', 'r-2'] });
      req.flush(stub);
      expect(await p).toEqual(stub);
    });
  });

  // =========================================================================
  // Projection & Forecasting
  // =========================================================================

  describe('projection and forecasting', () => {
    it('projectFinancialHealth GETs with params', async () => {
      const stub = { id: 'proj-1' };
      const p = service.projectFinancialHealth('s1', 12);
      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/flow-planning/projections/financial-health' &&
          r.params.get('stewardId') === 's1'
      );
      req.flush(stub);
      expect(await p).toEqual(stub);
    });

    it('projectResourceUtilization GETs with params', async () => {
      const stub = { id: 'proj-1' };
      const p = service.projectResourceUtilization('r-1', 6);
      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/flow-planning/projections/resource-utilization' &&
          r.params.get('resourceId') === 'r-1'
      );
      req.flush(stub);
      expect(await p).toEqual(stub);
    });

    it('projectGoalCompletion GETs with goalId param', async () => {
      const stub = { estimatedDate: '2025-06-01' };
      const p = service.projectGoalCompletion('g-1');
      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/flow-planning/projections/goal-completion' &&
          r.params.get('goalId') === 'g-1'
      );
      req.flush(stub);
      expect(await p).toEqual(stub);
    });

    it('identifyBreakpoints GETs breakpoints for projection', async () => {
      const p = service.identifyBreakpoints('proj-1', 'balance', 0);
      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/flow-planning/projections/proj-1/breakpoints' &&
          r.params.get('metric') === 'balance'
      );
      req.flush([]);
      expect(await p).toEqual([]);
    });

    it('extendTrendForward GETs trend projection', async () => {
      const stub = { id: 'proj-1' };
      const p = service.extendTrendForward('r-1', 12, 6);
      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/flow-planning/projections/trend' &&
          r.params.get('resourceId') === 'r-1'
      );
      req.flush(stub);
      expect(await p).toEqual(stub);
    });
  });

  // =========================================================================
  // Scenario Simulation
  // =========================================================================

  describe('scenario simulation', () => {
    it('createScenario POSTs to scenarios', async () => {
      const stub = { id: 'sc-1' };
      const p = service.createScenario('plan-1', 'What if', [], 'what-if');
      const req = httpMock.expectOne('/api/v1/flow-planning/scenarios');
      expect(req.request.method).toBe('POST');
      req.flush(stub);
      expect(await p).toEqual(stub);
    });

    it('runScenario POSTs to run', async () => {
      const stub = { scenarioId: 'sc-1' };
      const p = service.runScenario('sc-1');
      const req = httpMock.expectOne('/api/v1/flow-planning/scenarios/sc-1/run');
      expect(req.request.method).toBe('POST');
      req.flush(stub);
      expect(await p).toEqual(stub);
    });

    it('compareScenarios POSTs scenario IDs', async () => {
      const stub = { best: 'sc-1' };
      const p = service.compareScenarios(['sc-1', 'sc-2']);
      const req = httpMock.expectOne('/api/v1/flow-planning/scenarios/compare');
      expect(req.request.method).toBe('POST');
      expect(req.request.body).toEqual({ scenarioIds: ['sc-1', 'sc-2'] });
      req.flush(stub);
      expect(await p).toEqual(stub);
    });

    it('optimizeAllocation POSTs to optimize', async () => {
      const stub = { optimal: true };
      const p = service.optimizeAllocation('plan-1', [], 'maximize-surplus');
      const req = httpMock.expectOne('/api/v1/flow-planning/plans/plan-1/optimize');
      expect(req.request.method).toBe('POST');
      req.flush(stub);
      expect(await p).toEqual(stub);
    });
  });

  // =========================================================================
  // Cadence Management
  // =========================================================================

  describe('cadence management', () => {
    it('createRecurringPattern POSTs to patterns', async () => {
      const stub = { id: 'pat-1' };
      const p = service.createRecurringPattern('s1', { label: 'Rent' } as any);
      const req = httpMock.expectOne('/api/v1/flow-planning/patterns');
      expect(req.request.method).toBe('POST');
      req.flush(stub);
      expect(await p).toEqual(stub);
    });

    it('generateRecurringEvents GETs pattern events', async () => {
      const p = service.generateRecurringEvents('pat-1', 12);
      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/flow-planning/patterns/pat-1/events' &&
          r.params.get('durationMonths') === '12'
      );
      req.flush([]);
      expect(await p).toEqual([]);
    });

    it('calculateNextDue POSTs pattern', async () => {
      const p = service.calculateNextDue({ id: 'pat-1' } as any);
      const req = httpMock.expectOne('/api/v1/flow-planning/patterns/next-due');
      expect(req.request.method).toBe('POST');
      req.flush('2024-02-01');
      expect(await p).toBe('2024-02-01');
    });

    it('identifyPatternsFromHistory GETs detected patterns', async () => {
      const p = service.identifyPatternsFromHistory('s1', 'financial-asset', 12);
      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/flow-planning/patterns/detect' &&
          r.params.get('stewardId') === 's1'
      );
      req.flush([]);
      expect(await p).toEqual([]);
    });

    it('updatePatternFromActual POSTs actual event', async () => {
      const stub = { id: 'pat-1' };
      const p = service.updatePatternFromActual('pat-1', 'evt-1');
      const req = httpMock.expectOne('/api/v1/flow-planning/patterns/pat-1/actual');
      expect(req.request.method).toBe('POST');
      expect(req.request.body).toEqual({ actualEventId: 'evt-1' });
      req.flush(stub);
      expect(await p).toEqual(stub);
    });
  });

  // =========================================================================
  // Dashboard & Insights
  // =========================================================================

  describe('dashboard and insights', () => {
    it('buildFlowDashboard GETs dashboard', async () => {
      const stub = { activePlans: [] };
      const p = service.buildFlowDashboard('s1');
      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/flow-planning/dashboard' && r.params.get('stewardId') === 's1'
      );
      req.flush(stub);
      expect(await p).toEqual(stub);
    });

    it('analyzeFlowHealth GETs plan health', async () => {
      const stub = { score: 85 };
      const p = service.analyzeFlowHealth('plan-1');
      const req = httpMock.expectOne('/api/v1/flow-planning/plans/plan-1/health');
      req.flush(stub);
      expect(await p).toEqual(stub);
    });

    it('generatePlanningInsights GETs insights', async () => {
      const p = service.generatePlanningInsights('s1', 6);
      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/flow-planning/insights' &&
          r.params.get('stewardId') === 's1' &&
          r.params.get('lookbackMonths') === '6'
      );
      req.flush([]);
      expect(await p).toEqual([]);
    });

    it('detectAnomalies GETs anomalies', async () => {
      const p = service.detectAnomalies('r-1', 'high');
      const req = httpMock.expectOne(
        (r) =>
          r.url === '/api/v1/flow-planning/anomalies' &&
          r.params.get('resourceId') === 'r-1' &&
          r.params.get('sensitivity') === 'high'
      );
      req.flush([]);
      expect(await p).toEqual([]);
    });
  });

  // =========================================================================
  // Constitutional Compliance
  // =========================================================================

  describe('constitutional compliance', () => {
    it('checkPlanCompliance GETs compliance', async () => {
      const stub = { compliant: true };
      const p = service.checkPlanCompliance('plan-1');
      const req = httpMock.expectOne('/api/v1/flow-planning/plans/plan-1/compliance');
      expect(req.request.method).toBe('GET');
      req.flush(stub);
      expect(await p).toEqual(stub);
    });
  });
});
