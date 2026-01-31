/**
 * Stewardship Service Tests
 *
 * Tests for graduated capability management and policy enforcement.
 */

import { TestBed } from '@angular/core/testing';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';

import { StewardshipService } from './stewardship.service';

describe('StewardshipService', () => {
  let service: StewardshipService;
  let mockHolochain: jasmine.SpyObj<HolochainClientService>;

  beforeEach(() => {
    // Create mock for HolochainClientService
    mockHolochain = jasmine.createSpyObj('HolochainClientService', ['callZome']);

    TestBed.configureTestingModule({
      providers: [StewardshipService, { provide: HolochainClientService, useValue: mockHolochain }],
    });

    service = TestBed.inject(StewardshipService);
  });

  // ==========================================================================
  // Service Creation
  // ==========================================================================

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // ==========================================================================
  // State Signals
  // ==========================================================================

  it('should have policy signal', () => {
    expect(service.policy).toBeDefined();
  });

  it('should have mySubjects signal', () => {
    expect(service.mySubjects).toBeDefined();
  });

  it('should have myStewards signal', () => {
    expect(service.myStewards).toBeDefined();
  });

  it('should have isLoading signal', () => {
    expect(service.isLoading).toBeDefined();
  });

  it('should have error signal', () => {
    expect(service.error).toBeDefined();
  });

  // ==========================================================================
  // Public Methods
  // ==========================================================================

  it('should have getMyPolicy method', () => {
    expect(service.getMyPolicy).toBeDefined();
    expect(typeof service.getMyPolicy).toBe('function');
  });

  it('should have checkContentAccess method', () => {
    expect(service.checkContentAccess).toBeDefined();
    expect(typeof service.checkContentAccess).toBe('function');
  });

  it('should have checkFeatureAccess method', () => {
    expect(service.checkFeatureAccess).toBeDefined();
    expect(typeof service.checkFeatureAccess).toBe('function');
  });

  it('should have checkRouteAccess method', () => {
    expect(service.checkRouteAccess).toBeDefined();
    expect(typeof service.checkRouteAccess).toBe('function');
  });

  it('should have checkTimeAccess method', () => {
    expect(service.checkTimeAccess).toBeDefined();
    expect(typeof service.checkTimeAccess).toBe('function');
  });

  it('should have createGrant method', () => {
    expect(service.createGrant).toBeDefined();
    expect(typeof service.createGrant).toBe('function');
  });

  it('should have delegateGrant method', () => {
    expect(service.delegateGrant).toBeDefined();
    expect(typeof service.delegateGrant).toBe('function');
  });

  it('should have revokeGrant method', () => {
    expect(service.revokeGrant).toBeDefined();
    expect(typeof service.revokeGrant).toBe('function');
  });

  it('should have getMySubjects method', () => {
    expect(service.getMySubjects).toBeDefined();
    expect(typeof service.getMySubjects).toBe('function');
  });

  it('should have getMyStewards method', () => {
    expect(service.getMyStewards).toBeDefined();
    expect(typeof service.getMyStewards).toBe('function');
  });

  it('should have upsertPolicy method', () => {
    expect(service.upsertPolicy).toBeDefined();
    expect(typeof service.upsertPolicy).toBe('function');
  });

  it('should have getPoliciesForSubject method', () => {
    expect(service.getPoliciesForSubject).toBeDefined();
    expect(typeof service.getPoliciesForSubject).toBe('function');
  });

  it('should have getGrantForSubject method', () => {
    expect(service.getGrantForSubject).toBeDefined();
    expect(typeof service.getGrantForSubject).toBe('function');
  });

  it('should have getSubjectPolicy method', () => {
    expect(service.getSubjectPolicy).toBeDefined();
    expect(typeof service.getSubjectPolicy).toBe('function');
  });

  it('should have getParentPolicy method', () => {
    expect(service.getParentPolicy).toBeDefined();
    expect(typeof service.getParentPolicy).toBe('function');
  });

  it('should have getPolicyChain method', () => {
    expect(service.getPolicyChain).toBeDefined();
    expect(typeof service.getPolicyChain).toBe('function');
  });

  it('should have getMyPolicyChain method', () => {
    expect(service.getMyPolicyChain).toBeDefined();
    expect(typeof service.getMyPolicyChain).toBe('function');
  });

  it('should have fileAppeal method', () => {
    expect(service.fileAppeal).toBeDefined();
    expect(typeof service.fileAppeal).toBe('function');
  });

  it('should have getMyAppeals method', () => {
    expect(service.getMyAppeals).toBeDefined();
    expect(typeof service.getMyAppeals).toBe('function');
  });

  it('should have logActivity method', () => {
    expect(service.logActivity).toBeDefined();
    expect(typeof service.logActivity).toBe('function');
  });

  it('should have getMyActivityLogs method', () => {
    expect(service.getMyActivityLogs).toBeDefined();
    expect(typeof service.getMyActivityLogs).toBe('function');
  });

  it('should have initialize method', () => {
    expect(service.initialize).toBeDefined();
    expect(typeof service.initialize).toBe('function');
  });

  it('should have clearCache method', () => {
    expect(service.clearCache).toBeDefined();
    expect(typeof service.clearCache).toBe('function');
  });

  // ==========================================================================
  // Computed Signals
  // ==========================================================================

  it('should have hasStewards computed signal', () => {
    expect(service.hasStewards).toBeDefined();
  });

  it('should have hasSubjects computed signal', () => {
    expect(service.hasSubjects).toBeDefined();
  });

  // ==========================================================================
  // Initial State
  // ==========================================================================

  it('should initialize with null policy', () => {
    expect(service.policy()).toBeNull();
  });

  it('should initialize with empty subjects', () => {
    expect(service.mySubjects()).toEqual([]);
  });

  it('should initialize with empty stewards', () => {
    expect(service.myStewards()).toEqual([]);
  });

  it('should initialize with loading false', () => {
    expect(service.isLoading()).toBe(false);
  });

  it('should initialize with null error', () => {
    expect(service.error()).toBeNull();
  });

  // ==========================================================================
  // Clear Cache
  // ==========================================================================

  it('should clear cache when clearCache is called', () => {
    service.clearCache();
    expect(service.policy()).toBeNull();
    expect(service.mySubjects()).toEqual([]);
    expect(service.myStewards()).toEqual([]);
    expect(service.error()).toBeNull();
  });

  // ==========================================================================
  // getMyPolicy - Policy Loading
  // ==========================================================================

  it('should load policy successfully', async () => {
    const mockPolicy = {
      subjectId: 'user-123',
      computedAt: '2026-02-04T00:00:00Z',
      blockedCategories: [],
      blockedHashes: [],
      ageRatingMax: null,
      reachLevelMax: null,
      sessionMaxMinutes: null,
      dailyMaxMinutes: null,
      timeWindowsJson: '[]',
      cooldownMinutes: null,
      disabledFeatures: [],
      disabledRoutes: [],
      requireApproval: [],
      logSessions: false,
      logCategories: false,
      logPolicyEvents: true,
      retentionDays: 30,
      subjectCanView: true,
    };

    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: mockPolicy }));

    const result = await service.getMyPolicy();

    expect(result).toBeDefined();
    expect(result?.subjectId).toBe('user-123');
    expect(service.policy()?.subjectId).toBe('user-123');
  });

  it('should set loading state during policy fetch', async () => {
    let resolvePromise: (value: unknown) => void;
    const delayedPromise: Promise<any> = new Promise(resolve => {
      resolvePromise = resolve;
    });

    mockHolochain.callZome.and.returnValue(delayedPromise);

    // Start the fetch (don't await yet)
    const policyPromise = service.getMyPolicy();

    // Wait for next tick to let loading state be set
    await Promise.resolve();
    expect(service.isLoading()).toBe(true);

    // Now resolve the mock
    resolvePromise!({ success: true, data: null });
    await policyPromise;

    expect(service.isLoading()).toBe(false);
  });

  it('should handle policy fetch error gracefully', async () => {
    mockHolochain.callZome.and.returnValue(Promise.reject(new Error('Network error')));

    const result = await service.getMyPolicy();

    expect(result).toBeNull();
    expect(service.error()).toBe('Network error');
  });

  it('should return null when policy fetch fails', async () => {
    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: false, data: null }));

    const result = await service.getMyPolicy();

    expect(result).toBeNull();
  });

  // ==========================================================================
  // checkContentAccess - Policy Enforcement
  // ==========================================================================

  it('should allow content access when no restrictions', async () => {
    mockHolochain.callZome.and.returnValue(
      Promise.resolve({ success: true, data: { Allow: null } })
    );

    const result = await service.checkContentAccess('hash-123', ['educational']);

    expect(result).toBeDefined();
    expect(result.type).toBe('allow');
  });

  it('should block content access when restricted', async () => {
    const blockData = { Block: { reason: 'Age restricted' } };
    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: blockData }));

    const result = await service.checkContentAccess('hash-456', ['adult']);

    expect(result.type).toBe('block');
    if (result.type === 'block') {
      expect(result.reason).toBe('Age restricted');
    }
  });

  it('should fail open for content access on error', async () => {
    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: false, data: null }));

    const result = await service.checkContentAccess('hash-789', []);

    expect(result.type).toBe('allow');
  });

  // ==========================================================================
  // checkFeatureAccess - Feature Restrictions
  // ==========================================================================

  it('should allow feature access when not disabled', async () => {
    const policy = {
      subjectId: 'user-1',
      computedAt: '2026-02-04T00:00:00Z',
      blockedCategories: [],
      blockedHashes: [],
      ageRatingMax: null,
      reachLevelMax: null,
      sessionMaxMinutes: null,
      dailyMaxMinutes: null,
      timeWindowsJson: '[]',
      cooldownMinutes: null,
      disabledFeatures: [],
      disabledRoutes: [],
      requireApproval: [],
      logSessions: false,
      logCategories: false,
      logPolicyEvents: true,
      retentionDays: 30,
      subjectCanView: true,
    };

    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: policy }));

    const result = await service.checkFeatureAccess('analytics');

    expect(result).toBe(true);
  });

  it('should restrict disabled features', async () => {
    const policy = {
      subjectId: 'user-1',
      computedAt: '2026-02-04T00:00:00Z',
      blockedCategories: [],
      blockedHashes: [],
      ageRatingMax: null,
      reachLevelMax: null,
      sessionMaxMinutes: null,
      dailyMaxMinutes: null,
      timeWindowsJson: '[]',
      cooldownMinutes: null,
      disabledFeatures: ['social-sharing', 'export'],
      disabledRoutes: [],
      requireApproval: [],
      logSessions: false,
      logCategories: false,
      logPolicyEvents: true,
      retentionDays: 30,
      subjectCanView: true,
    };

    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: policy }));

    const result = await service.checkFeatureAccess('social-sharing');

    expect(result).toBe(false);
  });

  it('should fail open for feature access when no policy', async () => {
    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: false, data: null }));

    const result = await service.checkFeatureAccess('any-feature');

    expect(result).toBe(true);
  });

  // ==========================================================================
  // checkRouteAccess - Route Pattern Matching
  // ==========================================================================

  it('should allow route access when not restricted', async () => {
    const policy = {
      subjectId: 'user-1',
      computedAt: '2026-02-04T00:00:00Z',
      blockedCategories: [],
      blockedHashes: [],
      ageRatingMax: null,
      reachLevelMax: null,
      sessionMaxMinutes: null,
      dailyMaxMinutes: null,
      timeWindowsJson: '[]',
      cooldownMinutes: null,
      disabledFeatures: [],
      disabledRoutes: [],
      requireApproval: [],
      logSessions: false,
      logCategories: false,
      logPolicyEvents: true,
      retentionDays: 30,
      subjectCanView: true,
    };

    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: policy }));

    const result = await service.checkRouteAccess('/dashboard');

    expect(result).toBe(true);
  });

  it('should restrict disabled routes', async () => {
    const policy = {
      subjectId: 'user-1',
      computedAt: '2026-02-04T00:00:00Z',
      blockedCategories: [],
      blockedHashes: [],
      ageRatingMax: null,
      reachLevelMax: null,
      sessionMaxMinutes: null,
      dailyMaxMinutes: null,
      timeWindowsJson: '[]',
      cooldownMinutes: null,
      disabledFeatures: [],
      disabledRoutes: ['/admin/**', '/settings'],
      requireApproval: [],
      logSessions: false,
      logCategories: false,
      logPolicyEvents: true,
      retentionDays: 30,
      subjectCanView: true,
    };

    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: policy }));

    const result = await service.checkRouteAccess('/admin/users');

    expect(result).toBe(false);
  });

  // ==========================================================================
  // checkTimeAccess - Time Limits
  // ==========================================================================

  it('should return time access status', async () => {
    const policy = {
      subjectId: 'user-1',
      computedAt: '2026-02-04T00:00:00Z',
      blockedCategories: [],
      blockedHashes: [],
      ageRatingMax: null,
      reachLevelMax: null,
      sessionMaxMinutes: 120,
      dailyMaxMinutes: 480,
      timeWindowsJson: '[]',
      cooldownMinutes: null,
      disabledFeatures: [],
      disabledRoutes: [],
      requireApproval: [],
      logSessions: false,
      logCategories: false,
      logPolicyEvents: true,
      retentionDays: 30,
      subjectCanView: true,
    };

    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: policy }));

    const result = await service.checkTimeAccess();

    expect(result.status).toBe('allowed');
    if ('remainingSession' in result) {
      expect(result.remainingSession).toBe(120);
    }
    if ('remainingDaily' in result) {
      expect(result.remainingDaily).toBe(480);
    }
  });

  // ==========================================================================
  // Grant Operations - createGrant
  // ==========================================================================

  it('should create grant successfully', async () => {
    const mockGrant = {
      actionHash: 'hash-123',
      grant: {
        id: 'grant-001',
        stewardId: 'steward-1',
        subjectId: 'subject-1',
        tier: 'guardian',
        authorityBasis: 'legal',
        evidenceHash: null,
        verifiedBy: 'admin',
        contentFiltering: true,
        timeLimits: true,
        featureRestrictions: false,
        activityMonitoring: true,
        policyDelegation: false,
        delegatable: false,
        delegatedFrom: null,
        delegationDepth: 0,
        grantedAt: '2026-02-04T00:00:00Z',
        expiresAt: '2026-08-04T00:00:00Z',
        reviewAt: '2026-05-04T00:00:00Z',
        status: 'active',
        appealId: null,
        createdAt: '2026-02-04T00:00:00Z',
        updatedAt: '2026-02-04T00:00:00Z',
      },
    };

    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: mockGrant }));

    const input = {
      subjectId: 'subject-1',
      authorityBasis: 'mutual_consent' as const,
      evidenceHash: undefined,
      verifiedBy: 'admin',
      contentFiltering: true,
      timeLimits: true,
      featureRestrictions: false,
      activityMonitoring: true,
      policyDelegation: false,
      delegatable: false,
      expiresInDays: 180,
      reviewInDays: 90,
    };

    const result = await service.createGrant(input);

    expect(result).toBeDefined();
    expect(result?.id).toBe('grant-001');
    expect(result?.status).toBe('active');
  });

  // ==========================================================================
  // Grant Operations - revokeGrant
  // ==========================================================================

  it('should revoke grant successfully', async () => {
    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: null }));

    const result = await service.revokeGrant('grant-001');

    expect(result).toBe(true);
  });

  it('should return false when revoke grant fails', async () => {
    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: false, data: null }));

    const result = await service.revokeGrant('grant-001');

    expect(result).toBe(false);
  });

  // ==========================================================================
  // Grant Operations - getMySubjects
  // ==========================================================================

  it('should get my subjects successfully', async () => {
    const mockGrants = [
      {
        actionHash: 'hash-1',
        grant: {
          id: 'grant-001',
          stewardId: 'steward-1',
          subjectId: 'subject-1',
          tier: 'guardian',
          authorityBasis: 'legal',
          evidenceHash: null,
          verifiedBy: 'admin',
          contentFiltering: true,
          timeLimits: true,
          featureRestrictions: false,
          activityMonitoring: true,
          policyDelegation: false,
          delegatable: false,
          delegatedFrom: null,
          delegationDepth: 0,
          grantedAt: '2026-02-04T00:00:00Z',
          expiresAt: '2026-08-04T00:00:00Z',
          reviewAt: '2026-05-04T00:00:00Z',
          status: 'active',
          appealId: null,
          createdAt: '2026-02-04T00:00:00Z',
          updatedAt: '2026-02-04T00:00:00Z',
        },
      },
    ];

    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: mockGrants }));

    const result = await service.getMySubjects();

    expect(result).toBeDefined();
    expect(result.length).toBe(1);
    expect(result[0].id).toBe('grant-001');
  });

  it('should return empty array when getMySubjects fails', async () => {
    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: false, data: null }));

    const result = await service.getMySubjects();

    expect(result).toEqual([]);
  });

  // ==========================================================================
  // Grant Operations - getMyStewards
  // ==========================================================================

  it('should get my stewards successfully', async () => {
    const mockStewards = [
      {
        actionHash: 'hash-1',
        grant: {
          id: 'grant-002',
          stewardId: 'steward-2',
          subjectId: 'me',
          tier: 'parent',
          authorityBasis: 'familial',
          evidenceHash: null,
          verifiedBy: 'system',
          contentFiltering: true,
          timeLimits: true,
          featureRestrictions: true,
          activityMonitoring: true,
          policyDelegation: false,
          delegatable: false,
          delegatedFrom: null,
          delegationDepth: 0,
          grantedAt: '2026-02-04T00:00:00Z',
          expiresAt: '2026-08-04T00:00:00Z',
          reviewAt: '2026-05-04T00:00:00Z',
          status: 'active',
          appealId: null,
          createdAt: '2026-02-04T00:00:00Z',
          updatedAt: '2026-02-04T00:00:00Z',
        },
      },
    ];

    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: mockStewards }));

    const result = await service.getMyStewards();

    expect(result).toBeDefined();
    expect(result.length).toBe(1);
  });

  // ==========================================================================
  // Computed Signals - hasStewards and hasSubjects
  // ==========================================================================

  it('should compute hasStewards as false when empty', () => {
    expect(service.hasStewards()).toBe(false);
  });

  it('should compute hasSubjects as false when empty', () => {
    expect(service.hasSubjects()).toBe(false);
  });

  // ==========================================================================
  // Policy Query Methods
  // ==========================================================================

  it('should get policies for subject', async () => {
    const mockPolicies = [
      {
        actionHash: 'hash-p1',
        policy: {
          id: 'policy-1',
          subjectId: 'subject-1',
          deviceId: 'device-1',
          authorId: 'author-1',
          authorTier: 'guardian',
          inheritsFrom: null,
          blockedCategoriesJson: '[]',
          blockedHashesJson: '[]',
          ageRatingMax: null,
          reachLevelMax: null,
          sessionMaxMinutes: null,
          dailyMaxMinutes: null,
          timeWindowsJson: '[]',
          cooldownMinutes: null,
          disabledFeaturesJson: '[]',
          disabledRoutesJson: '[]',
          requireApprovalJson: '[]',
          logSessions: false,
          logCategories: false,
          logPolicyEvents: true,
          retentionDays: 30,
          subjectCanView: true,
          effectiveFrom: '2026-02-04T00:00:00Z',
          effectiveUntil: null,
          version: 1,
          createdAt: '2026-02-04T00:00:00Z',
          updatedAt: '2026-02-04T00:00:00Z',
        },
      },
    ];

    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: mockPolicies }));

    const result = await service.getPoliciesForSubject('subject-1');

    expect(result).toBeDefined();
    expect(result.length).toBe(1);
  });

  it('should return empty array when getPoliciesForSubject fails', async () => {
    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: false, data: null }));

    const result = await service.getPoliciesForSubject('subject-1');

    expect(result).toEqual([]);
  });

  // ==========================================================================
  // Appeal Operations
  // ==========================================================================

  it('should file appeal successfully', async () => {
    const mockAppeal = {
      actionHash: 'hash-a1',
      appeal: {
        id: 'appeal-1',
        appellantId: 'user-1',
        grantId: 'grant-1',
        policyId: null,
        appealType: 'hardship',
        groundsJson: '["reason1"]',
        evidenceJson: '[]',
        advocateId: null,
        advocateNotes: null,
        arbitrationLayer: 'family',
        assignedTo: null,
        status: 'filed',
        statusChangedAt: null,
        decisionJson: null,
        decisionMadeBy: null,
        decisionMadeAt: null,
        filedAt: '2026-02-04T00:00:00Z',
        expiresAt: '2026-05-04T00:00:00Z',
        createdAt: '2026-02-04T00:00:00Z',
        updatedAt: '2026-02-04T00:00:00Z',
      },
    };

    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: mockAppeal }));

    const input = {
      grantId: 'grant-1',
      policyId: undefined,
      appealType: 'excessive' as const,
      grounds: ['reason1'],
      evidenceJson: '[]',
      advocateId: undefined,
    };

    const result = await service.fileAppeal(input);

    expect(result).toBeDefined();
    expect(result?.id).toBe('appeal-1');
  });

  it('should get my appeals successfully', async () => {
    const mockAppeals = [
      {
        actionHash: 'hash-a1',
        appeal: {
          id: 'appeal-1',
          appellantId: 'user-1',
          grantId: 'grant-1',
          policyId: null,
          appealType: 'hardship',
          groundsJson: '[]',
          evidenceJson: '[]',
          advocateId: null,
          advocateNotes: null,
          arbitrationLayer: 'family',
          assignedTo: null,
          status: 'filed',
          statusChangedAt: null,
          decisionJson: null,
          decisionMadeBy: null,
          decisionMadeAt: null,
          filedAt: '2026-02-04T00:00:00Z',
          expiresAt: '2026-05-04T00:00:00Z',
          createdAt: '2026-02-04T00:00:00Z',
          updatedAt: '2026-02-04T00:00:00Z',
        },
      },
    ];

    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: mockAppeals }));

    const result = await service.getMyAppeals();

    expect(result).toBeDefined();
    expect(result.length).toBe(1);
  });

  // ==========================================================================
  // Activity Logging
  // ==========================================================================

  it('should log activity successfully', async () => {
    const mockLog = {
      actionHash: 'hash-log1',
      log: {
        id: 'log-1',
        subjectId: 'subject-1',
        deviceId: 'device-1',
        sessionId: 'session-1',
        sessionStartedAt: '2026-02-04T00:00:00Z',
        sessionDurationMinutes: 60,
        categoriesAccessedJson: '["educational"]',
        policyEventsJson: '[]',
        loggedAt: '2026-02-04T01:00:00Z',
        retentionExpiresAt: '2026-03-04T01:00:00Z',
      },
    };

    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: mockLog }));

    const result = await service.logActivity('session-1', 60, ['educational'], []);

    expect(result).toBeDefined();
    expect(result?.sessionId).toBe('session-1');
  });

  it('should get activity logs successfully', async () => {
    const mockLogs = [
      {
        actionHash: 'hash-log1',
        log: {
          id: 'log-1',
          subjectId: 'subject-1',
          deviceId: 'device-1',
          sessionId: 'session-1',
          sessionStartedAt: '2026-02-04T00:00:00Z',
          sessionDurationMinutes: 60,
          categoriesAccessedJson: '[]',
          policyEventsJson: '[]',
          loggedAt: '2026-02-04T01:00:00Z',
          retentionExpiresAt: '2026-03-04T01:00:00Z',
        },
      },
    ];

    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: mockLogs }));

    const result = await service.getMyActivityLogs();

    expect(result).toBeDefined();
    expect(result.length).toBe(1);
  });

  // ==========================================================================
  // Initialize Method
  // ==========================================================================

  it('should initialize successfully', async () => {
    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: null }));

    await service.initialize();

    expect(service.isLoading()).toBe(false);
  });
});
