import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute, Router } from '@angular/router';

import { PolicyConsoleComponent } from './policy-console.component';
import { StewardshipService } from '../../services/stewardship.service';
import type { DevicePolicy, ComputedPolicy, StewardshipGrant } from '../../models/stewardship.model';

describe('PolicyConsoleComponent', () => {
  let component: PolicyConsoleComponent;
  let fixture: ComponentFixture<PolicyConsoleComponent>;
  let mockStewardshipService: jasmine.SpyObj<StewardshipService>;
  let mockRouter: jasmine.SpyObj<Router>;
  let mockActivatedRoute: Partial<ActivatedRoute>;

  const mockPolicy: DevicePolicy = {
    id: 'policy-123',
    subjectId: 'subject-123',
    authorId: 'author-456',
    authorTier: 'guardian',
    contentRules: {
      blockedCategories: ['violence'],
      blockedHashes: [],
    },
    timeRules: {
      sessionMaxMinutes: 60,
      dailyMaxMinutes: 120,
      timeWindows: [],
    },
    featureRules: {
      disabledFeatures: [],
      disabledRoutes: [],
      requireApproval: [],
    },
    blockedCategories: [],
    blockedHashes: [],
    timeWindows: [],
    disabledFeatures: [],
    disabledRoutes: [],
    requireApproval: [],
    logSessions: true,
    logCategories: true,
    logPolicyEvents: true,
    retentionDays: 30,
    subjectCanView: true,
    effectiveFrom: '2024-01-01T00:00:00Z',
    version: 1,
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-01T00:00:00Z',
  };

  beforeEach(async () => {
    mockStewardshipService = jasmine.createSpyObj('StewardshipService', [
      'getMyPolicy',
      'getSubjectPolicy',
      'getGrantForSubject',
      'getPolicyChain',
      'getMyPolicyChain',
      'getParentPolicy',
      'upsertPolicy',
    ]);

    mockRouter = jasmine.createSpy('Router') as any;

    mockActivatedRoute = {
      snapshot: {
        paramMap: {
          get: jasmine.createSpy('get').and.returnValue(null),
        } as any,
      } as any,
    };

    mockStewardshipService.getMyPolicy.and.returnValue(Promise.resolve({} as ComputedPolicy));
    mockStewardshipService.getMyPolicyChain.and.returnValue(Promise.resolve([]));

    await TestBed.configureTestingModule({
      imports: [PolicyConsoleComponent],
      providers: [
        { provide: StewardshipService, useValue: mockStewardshipService },
        { provide: Router, useValue: mockRouter },
        { provide: ActivatedRoute, useValue: mockActivatedRoute },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(PolicyConsoleComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('Tab Navigation', () => {
    it('should change active tab', () => {
      component.setActiveTab('time');

      expect(component.activeTab()).toBe('time');
    });
  });

  describe('Content Rules Editing', () => {
    it('should toggle category blocking', () => {
      component.toggleCategory('violence');

      expect(component.isCategoryBlocked('violence')).toBe(true);

      component.toggleCategory('violence');

      expect(component.isCategoryBlocked('violence')).toBe(false);
    });

    it('should set age rating', () => {
      component.setAgeRating('R');

      expect(component.editingContentRules().ageRatingMax).toBe('R');
    });

    it('should set reach level', () => {
      component.setReachLevel(3);

      expect(component.editingContentRules().reachLevelMax).toBe(3);
    });
  });

  describe('Time Rules Editing', () => {
    it('should set session limit', () => {
      component.setSessionLimit(90);

      expect(component.editingTimeRules().sessionMaxMinutes).toBe(90);
    });

    it('should set daily limit', () => {
      component.setDailyLimit(180);

      expect(component.editingTimeRules().dailyMaxMinutes).toBe(180);
    });

    it('should set cooldown', () => {
      component.setCooldown(15);

      expect(component.editingTimeRules().cooldownMinutes).toBe(15);
    });

    it('should add time window', () => {
      const initialLength = component.editingTimeRules().timeWindows.length;

      component.addTimeWindow();

      expect(component.editingTimeRules().timeWindows.length).toBe(initialLength + 1);
    });

    it('should remove time window', () => {
      component.addTimeWindow();
      component.addTimeWindow();

      component.removeTimeWindow(0);

      expect(component.editingTimeRules().timeWindows.length).toBe(1);
    });
  });

  describe('Feature Rules Editing', () => {
    it('should toggle feature disabling', () => {
      component.toggleFeature('direct_messaging');

      expect(component.isFeatureDisabled('direct_messaging')).toBe(true);

      component.toggleFeature('direct_messaging');

      expect(component.isFeatureDisabled('direct_messaging')).toBe(false);
    });

    it('should not disable inalienable features', () => {
      const inalienable = component.inalienableFeatures[0];

      component.toggleFeature(inalienable);

      expect(component.isFeatureDisabled(inalienable)).toBe(false);
    });
  });

  describe('Formatters', () => {
    it('should format category names', () => {
      expect(component.formatCategory('adult_content')).toBe('Adult Content');
    });

    it('should format feature names', () => {
      expect(component.formatFeature('file_sharing')).toBe('File Sharing');
    });

    it('should format time', () => {
      expect(component.formatTime(9, 30)).toBe('09:30');
      expect(component.formatTime(14, 5)).toBe('14:05');
    });
  });

  describe('Message Clearing', () => {
    it('should clear error and success messages', () => {
      component.error.set('Error');
      component.successMessage.set('Success');

      component.clearMessages();

      expect(component.error()).toBeNull();
      expect(component.successMessage()).toBeNull();
    });
  });

  describe('Component Initialization', () => {
    it('should initialize with null subject for self-editing', async () => {
      fixture.detectChanges();
      await fixture.whenStable();

      expect(component.subjectId()).toBeNull();
      expect(component.isEditingSelf()).toBe(true);
    });

    it('should initialize with subject ID from route', async () => {
      (mockActivatedRoute.snapshot!.paramMap.get as jasmine.Spy).and.returnValue('subject-123');

      component.ngOnInit();
      await fixture.whenStable();

      expect(component.subjectId()).toBe('subject-123');
      expect(component.isEditingSelf()).toBe(false);
    });

    it('should load data on init', async () => {
      spyOn(component, 'loadData').and.returnValue(Promise.resolve());

      component.ngOnInit();

      expect(component.loadData).toHaveBeenCalled();
    });
  });

  describe('Data Loading - Self', () => {
    beforeEach(() => {
      component.subjectId.set(null);
    });

    it('should load own policy when editing self', async () => {
      const mockComputedPolicy: ComputedPolicy = {
        subjectId: 'self',
        computedAt: '2024-01-01T00:00:00Z',
        blockedCategories: ['violence'],
        blockedHashes: [],
        disabledFeatures: ['messaging'],
        disabledRoutes: [],
        requireApproval: [],
        sessionMaxMinutes: 60,
        dailyMaxMinutes: 120,
        timeWindows: [],
        cooldownMinutes: 10,
        logSessions: true,
        logCategories: true,
        logPolicyEvents: true,
        retentionDays: 30,
        subjectCanView: true,
      };
      mockStewardshipService.getMyPolicy.and.returnValue(Promise.resolve(mockComputedPolicy));

      await component.loadData();

      expect(mockStewardshipService.getMyPolicy).toHaveBeenCalled();
      expect(mockStewardshipService.getMyPolicyChain).toHaveBeenCalled();
      expect(component.myTier()).toBe('self');
      expect(component.isLoading()).toBe(false);
    });

    it('should populate editing state from computed policy', async () => {
      const mockComputedPolicy: ComputedPolicy = {
        subjectId: 'self',
        computedAt: '2024-01-01T00:00:00Z',
        blockedCategories: ['violence', 'adult'],
        blockedHashes: [],
        disabledFeatures: ['messaging'],
        disabledRoutes: ['/admin'],
        requireApproval: [],
        sessionMaxMinutes: 90,
        dailyMaxMinutes: 180,
        timeWindows: [{ startHour: 9, startMinute: 0, endHour: 17, endMinute: 0, daysOfWeek: [1, 2, 3, 4, 5] }],
        cooldownMinutes: 15,
        ageRatingMax: 'PG-13',
        reachLevelMax: 5,
        logSessions: true,
        logCategories: true,
        logPolicyEvents: true,
        retentionDays: 30,
        subjectCanView: true,
      };
      mockStewardshipService.getMyPolicy.and.returnValue(Promise.resolve(mockComputedPolicy));

      await component.loadData();

      expect(component.editingContentRules().blockedCategories).toEqual(['violence', 'adult']);
      expect(component.editingContentRules().ageRatingMax).toBe('PG-13');
      expect(component.editingContentRules().reachLevelMax).toBe(5);
      expect(component.editingTimeRules().sessionMaxMinutes).toBe(90);
      expect(component.editingTimeRules().dailyMaxMinutes).toBe(180);
      expect(component.editingTimeRules().cooldownMinutes).toBe(15);
      expect(component.editingFeatureRules().disabledFeatures).toEqual(['messaging']);
      expect(component.editingFeatureRules().disabledRoutes).toEqual(['/admin']);
    });

    it('should handle load error gracefully', async () => {
      mockStewardshipService.getMyPolicy.and.returnValue(Promise.reject(new Error('Network error')));

      await component.loadData();

      expect(component.error()).toContain('Failed to load policy');
      expect(component.isLoading()).toBe(false);
    });
  });

  describe('Data Loading - Other Subject', () => {
    beforeEach(() => {
      component.subjectId.set('subject-123');
    });

    it('should load subject policy with grant', async () => {
      const mockGrant: StewardshipGrant = {
        id: 'grant-1',
        stewardId: 'steward-123',
        subjectId: 'subject-123',
        tier: 'guardian',
        authorityBasis: 'minor_guardianship',
        verifiedBy: 'verifier-123',
        contentFiltering: true,
        timeLimits: true,
        featureRestrictions: true,
        activityMonitoring: true,
        policyDelegation: false,
        delegatable: false,
        delegationDepth: 0,
        grantedAt: '2024-01-01T00:00:00Z',
        expiresAt: '2025-01-01T00:00:00Z',
        reviewAt: '2024-07-01T00:00:00Z',
        status: 'active',
        createdAt: '2024-01-01T00:00:00Z',
        updatedAt: '2024-01-01T00:00:00Z',
      };

      mockStewardshipService.getGrantForSubject.and.returnValue(Promise.resolve(mockGrant));
      mockStewardshipService.getSubjectPolicy.and.returnValue(Promise.resolve(mockPolicy));
      mockStewardshipService.getPolicyChain.and.returnValue(Promise.resolve([]));
      mockStewardshipService.getParentPolicy.and.returnValue(Promise.resolve(null));

      await component.loadData();

      expect(mockStewardshipService.getGrantForSubject).toHaveBeenCalledWith('subject-123');
      expect(mockStewardshipService.getSubjectPolicy).toHaveBeenCalledWith('subject-123');
      expect(mockStewardshipService.getPolicyChain).toHaveBeenCalledWith('subject-123');
      expect(mockStewardshipService.getParentPolicy).toHaveBeenCalledWith('subject-123');
      expect(component.grant()).toEqual(mockGrant);
      expect(component.myTier()).toBe('guardian');
      expect(component.isLoading()).toBe(false);
    });

    it('should populate editing state from subject policy', async () => {
      mockStewardshipService.getGrantForSubject.and.returnValue(Promise.resolve({} as StewardshipGrant));
      mockStewardshipService.getSubjectPolicy.and.returnValue(Promise.resolve(mockPolicy));
      mockStewardshipService.getPolicyChain.and.returnValue(Promise.resolve([]));
      mockStewardshipService.getParentPolicy.and.returnValue(Promise.resolve(null));

      await component.loadData();

      expect(component.editingContentRules()).toEqual(mockPolicy.contentRules);
      expect(component.editingTimeRules()).toEqual(mockPolicy.timeRules);
      expect(component.editingFeatureRules()).toEqual(mockPolicy.featureRules);
    });
  });

  describe('Permission Checks', () => {
    it('should allow guardian to edit content', () => {
      component.myTier.set('guardian');

      expect(component.canEditContent()).toBe(true);
    });

    it('should allow coordinator to edit content', () => {
      component.myTier.set('coordinator');

      expect(component.canEditContent()).toBe(true);
    });

    it('should not allow guide to edit content', () => {
      component.myTier.set('guide');

      expect(component.canEditContent()).toBe(false);
    });

    it('should not allow self to edit content', () => {
      component.myTier.set('self');

      expect(component.canEditContent()).toBe(false);
    });

    it('should allow guide to view monitoring', () => {
      component.myTier.set('guide');

      expect(component.canViewMonitoring()).toBe(true);
    });

    it('should not allow guide to edit monitoring', () => {
      component.myTier.set('guide');

      expect(component.canEditMonitoring()).toBe(false);
    });

    it('should allow guardian to edit monitoring', () => {
      component.myTier.set('guardian');

      expect(component.canEditMonitoring()).toBe(true);
    });
  });

  describe('Time Window Management', () => {
    it('should update time window field', () => {
      component.addTimeWindow();

      component.updateTimeWindow(0, 'startHour', 10);

      expect(component.editingTimeRules().timeWindows[0].startHour).toBe(10);
    });

    it('should update multiple fields independently', () => {
      component.addTimeWindow();

      component.updateTimeWindow(0, 'startHour', 10);
      component.updateTimeWindow(0, 'endHour', 18);
      component.updateTimeWindow(0, 'daysOfWeek', [1, 3, 5]);

      const window = component.editingTimeRules().timeWindows[0];
      expect(window.startHour).toBe(10);
      expect(window.endHour).toBe(18);
      expect(window.daysOfWeek).toEqual([1, 3, 5]);
    });

    it('should handle multiple time windows', () => {
      component.addTimeWindow();
      component.addTimeWindow();
      component.addTimeWindow();

      expect(component.editingTimeRules().timeWindows.length).toBe(3);

      component.removeTimeWindow(1);

      expect(component.editingTimeRules().timeWindows.length).toBe(2);
    });
  });

  describe('Category Inheritance', () => {
    it('should detect inherited category from parent policy', () => {
      const parentPolicy: ComputedPolicy = {
        subjectId: 'parent',
        computedAt: '2024-01-01T00:00:00Z',
        blockedCategories: ['violence', 'adult'],
        blockedHashes: [],
        disabledFeatures: [],
        disabledRoutes: [],
        requireApproval: [],
        timeWindows: [],
        logSessions: true,
        logCategories: true,
        logPolicyEvents: true,
        retentionDays: 30,
        subjectCanView: true,
      };
      component.parentPolicy.set(parentPolicy);

      expect(component.isCategoryInherited('violence')).toBe(true);
      expect(component.isCategoryInherited('spam')).toBe(false);
    });

    it('should detect inherited feature from parent policy', () => {
      const parentPolicy: ComputedPolicy = {
        subjectId: 'parent',
        computedAt: '2024-01-01T00:00:00Z',
        blockedCategories: [],
        blockedHashes: [],
        disabledFeatures: ['messaging', 'file_sharing'],
        disabledRoutes: [],
        requireApproval: [],
        timeWindows: [],
        logSessions: true,
        logCategories: true,
        logPolicyEvents: true,
        retentionDays: 30,
        subjectCanView: true,
      };
      component.parentPolicy.set(parentPolicy);

      expect(component.isFeatureInherited('messaging')).toBe(true);
      expect(component.isFeatureInherited('direct_calls')).toBe(false);
    });

    it('should return false for inheritance when no parent policy', () => {
      component.parentPolicy.set(null);

      expect(component.isCategoryInherited('violence')).toBe(false);
      expect(component.isFeatureInherited('messaging')).toBe(false);
    });
  });

  describe('Inalienable Features', () => {
    it('should identify inalienable features', () => {
      const inalienable = component.inalienableFeatures[0];

      expect(component.isFeatureInalienable(inalienable)).toBe(true);
    });

    it('should not disable inalienable features', () => {
      const inalienable = component.inalienableFeatures[0];
      component.editingFeatureRules.set({ disabledFeatures: [], disabledRoutes: [], requireApproval: [] });

      component.toggleFeature(inalienable);

      expect(component.isFeatureDisabled(inalienable)).toBe(false);
    });

    it('should allow disabling non-inalienable features', () => {
      component.toggleFeature('direct_messaging');

      expect(component.isFeatureDisabled('direct_messaging')).toBe(true);
    });
  });

  describe('Save Policy', () => {
    beforeEach(() => {
      mockStewardshipService.upsertPolicy.and.returnValue(Promise.resolve(null));
      mockStewardshipService.getMyPolicy.and.returnValue(Promise.resolve({} as ComputedPolicy));
      mockStewardshipService.getMyPolicyChain.and.returnValue(Promise.resolve([]));
    });

    it('should not save if already saving', async () => {
      component.isSaving.set(true);

      await component.savePolicy();

      expect(mockStewardshipService.upsertPolicy).not.toHaveBeenCalled();
    });

    it('should clear messages before saving', async () => {
      component.error.set('Previous error');
      component.successMessage.set('Previous success');

      await component.savePolicy();

      expect(component.error()).toBeNull();
      expect(component.successMessage()).not.toBeNull(); // Set after save
    });

    it('should call upsertPolicy with editing state', async () => {
      component.subjectId.set('subject-123');
      component.editingContentRules.set({
        blockedCategories: ['violence'],
        blockedHashes: [],
        ageRatingMax: 'PG',
        reachLevelMax: 3,
      });

      await component.savePolicy();

      expect(mockStewardshipService.upsertPolicy).toHaveBeenCalledWith({
        subjectId: 'subject-123',
        contentRules: jasmine.objectContaining({
          blockedCategories: ['violence'],
          ageRatingMax: 'PG',
        }),
        timeRules: jasmine.any(Object),
        featureRules: jasmine.any(Object),
      });
    });

    it('should set success message on successful save', async () => {
      await component.savePolicy();

      expect(component.successMessage()).toContain('Policy saved successfully');
    });

    it('should reload data after successful save', async () => {
      spyOn(component, 'loadData').and.returnValue(Promise.resolve());

      await component.savePolicy();

      expect(component.loadData).toHaveBeenCalled();
    });

    it('should handle save error gracefully', async () => {
      mockStewardshipService.upsertPolicy.and.returnValue(Promise.reject(new Error('Save failed')));

      await component.savePolicy();

      expect(component.error()).toContain('Failed to save policy');
      expect(component.isSaving()).toBe(false);
    });

    it('should reset saving state after completion', async () => {
      await component.savePolicy();

      expect(component.isSaving()).toBe(false);
    });
  });

  describe('Display Helpers', () => {
    it('should format day names correctly', () => {
      expect(component.getDayName(0)).toBe('Sun');
      expect(component.getDayName(1)).toBe('Mon');
      expect(component.getDayName(6)).toBe('Sat');
    });

    it('should handle invalid day numbers', () => {
      expect(component.getDayName(7)).toBe('');
      expect(component.getDayName(-1)).toBe('');
    });

    it('should get steward tier labels', () => {
      const label = component.getStewardTierLabel('guardian');

      expect(label).toBeDefined();
      expect(typeof label).toBe('string');
    });

    it('should get authority basis labels', () => {
      const label = component.getAuthorityBasisLabel('minor_guardianship');

      expect(label).toBeDefined();
      expect(typeof label).toBe('string');
    });
  });

  describe('Subject Display Name', () => {
    it('should show "Your Settings" when editing self', () => {
      component.subjectId.set(null);

      expect(component.subjectDisplayName()).toBe('Your Settings');
    });

    it('should show subject ID snippet when editing other', () => {
      const mockGrant: StewardshipGrant = {
        id: 'grant-1',
        stewardId: 'steward-123',
        subjectId: 'subject-123456789',
        tier: 'guardian',
        authorityBasis: 'minor_guardianship',
        verifiedBy: 'verifier-123',
        contentFiltering: true,
        timeLimits: true,
        featureRestrictions: false,
        activityMonitoring: true,
        policyDelegation: false,
        delegatable: false,
        delegationDepth: 0,
        grantedAt: '2024-01-01T00:00:00Z',
        expiresAt: '2025-01-01T00:00:00Z',
        reviewAt: '2024-07-01T00:00:00Z',
        status: 'active',
        createdAt: '2024-01-01T00:00:00Z',
        updatedAt: '2024-01-01T00:00:00Z',
      };
      component.grant.set(mockGrant);
      component.subjectId.set('subject-123456789');

      const displayName = component.subjectDisplayName();

      expect(displayName).toContain('Settings for');
      expect(displayName).toContain('subject-');
      expect(displayName).toContain('...');
    });
  });

  describe('Edge Cases', () => {
    it('should handle undefined values in time rules', () => {
      component.setSessionLimit(undefined);
      component.setDailyLimit(undefined);
      component.setCooldown(undefined);

      expect(component.editingTimeRules().sessionMaxMinutes).toBeUndefined();
      expect(component.editingTimeRules().dailyMaxMinutes).toBeUndefined();
      expect(component.editingTimeRules().cooldownMinutes).toBeUndefined();
    });

    it('should handle undefined values in content rules', () => {
      component.setAgeRating(undefined);
      component.setReachLevel(undefined);

      expect(component.editingContentRules().ageRatingMax).toBeUndefined();
      expect(component.editingContentRules().reachLevelMax).toBeUndefined();
    });

    it('should handle removing last time window', () => {
      component.addTimeWindow();

      component.removeTimeWindow(0);

      expect(component.editingTimeRules().timeWindows.length).toBe(0);
    });

    it('should handle time window with zero-based day indexing', () => {
      component.addTimeWindow();
      component.updateTimeWindow(0, 'daysOfWeek', [0, 6]); // Sunday and Saturday

      expect(component.editingTimeRules().timeWindows[0].daysOfWeek).toEqual([0, 6]);
    });
  });
});
