import { ComponentFixture, TestBed } from '@angular/core/testing';
import { signal } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';

import { CapabilitiesDashboardComponent } from './capabilities-dashboard.component';
import { StewardshipService } from '../../services/stewardship.service';
import type { ComputedPolicy, StewardshipGrant, TimeAccessDecision } from '../../models/stewardship.model';

describe('CapabilitiesDashboardComponent', () => {
  let component: CapabilitiesDashboardComponent;
  let fixture: ComponentFixture<CapabilitiesDashboardComponent>;
  let mockStewardshipService: jasmine.SpyObj<StewardshipService>;

  const mockPolicy: ComputedPolicy = {
    subjectId: 'subject-456',
    computedAt: '2024-01-01T00:00:00Z',
    blockedCategories: ['violence', 'adult'],
    blockedHashes: [],
    disabledFeatures: ['messaging'],
    disabledRoutes: ['/admin'],
    requireApproval: [],
    sessionMaxMinutes: 60,
    dailyMaxMinutes: 120,
    timeWindows: [],
    ageRatingMax: 'PG-13',
    reachLevelMax: 5,
    cooldownMinutes: 10,
    logSessions: true,
    logCategories: true,
    logPolicyEvents: true,
    retentionDays: 30,
    subjectCanView: true,
  };

  const mockGrant: StewardshipGrant = {
    id: 'grant-1',
    stewardId: 'steward-123',
    subjectId: 'subject-456',
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

  const mockTimeAccess: TimeAccessDecision = {
    status: 'allowed',
    remainingSession: 45,
    remainingDaily: 90,
  };

  beforeEach(async () => {
    mockStewardshipService = jasmine.createSpyObj('StewardshipService', [
      'getMyPolicy',
      'getMyStewards',
      'checkTimeAccess',
    ]);

    mockStewardshipService.getMyPolicy.and.returnValue(Promise.resolve(mockPolicy));
    mockStewardshipService.getMyStewards.and.returnValue(Promise.resolve([mockGrant]));
    mockStewardshipService.checkTimeAccess.and.returnValue(Promise.resolve(mockTimeAccess));

    await TestBed.configureTestingModule({
      imports: [CapabilitiesDashboardComponent],
      providers: [
        { provide: StewardshipService, useValue: mockStewardshipService },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(CapabilitiesDashboardComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('Data Loading', () => {
    it('should load policy, stewards, and time access on init', async () => {
      await component.loadData();

      expect(mockStewardshipService.getMyPolicy).toHaveBeenCalled();
      expect(mockStewardshipService.getMyStewards).toHaveBeenCalled();
      expect(mockStewardshipService.checkTimeAccess).toHaveBeenCalled();
      expect(component.policy()).toEqual(mockPolicy);
      expect(component.stewards()).toEqual([mockGrant]);
      expect(component.timeAccess()).toEqual(mockTimeAccess);
    });

    it('should handle load error gracefully', async () => {
      mockStewardshipService.getMyPolicy.and.returnValue(Promise.reject(new Error('Network error')));

      await component.loadData();

      expect(component.error()).toContain('Failed to load capabilities');
    });

    it('should refresh time access without affecting other data', async () => {
      const newTimeAccess: TimeAccessDecision = {
        status: 'allowed',
        remainingSession: 30,
        remainingDaily: 60,
      };
      mockStewardshipService.checkTimeAccess.and.returnValue(Promise.resolve(newTimeAccess));

      await component.refreshTimeAccess();

      expect(component.timeAccess()).toEqual(newTimeAccess);
    });

    it('should silently handle time access refresh failure', async () => {
      mockStewardshipService.checkTimeAccess.and.returnValue(Promise.reject(new Error('Timeout')));

      await component.refreshTimeAccess();

      // Should not throw or set error - just log warning
      expect(component.error()).toBeNull();
    });
  });

  describe('Formatters', () => {
    it('should format minutes to hours and minutes', () => {
      expect(component.formatMinutes(90)).toBe('1h 30m');
      expect(component.formatMinutes(60)).toBe('1h');
      expect(component.formatMinutes(45)).toBe('45m');
    });

    it('should format category names', () => {
      expect(component.formatCategory('adult_content')).toBe('Adult Content');
      expect(component.formatCategory('violence')).toBe('Violence');
    });

    it('should format feature names', () => {
      expect(component.formatFeature('direct_messaging')).toBe('Direct Messaging');
      expect(component.formatFeature('file_sharing')).toBe('File Sharing');
    });
  });

  describe('Lifecycle Hooks', () => {
    it('should load data on init', async () => {
      spyOn(component, 'loadData');

      component.ngOnInit();

      expect(component.loadData).toHaveBeenCalled();
    });

    it('should setup timer on init', () => {
      component.ngOnInit();

      expect(component['timerSubscription']).toBeDefined();
    });

    it('should cleanup timer on destroy', () => {
      component.ngOnInit();
      const subscription = component['timerSubscription'];
      spyOn(subscription!, 'unsubscribe');

      component.ngOnDestroy();

      expect(subscription!.unsubscribe).toHaveBeenCalled();
    });
  });

  describe('Computed State - Has Restrictions', () => {
    it('should detect blocked categories', () => {
      component.policy.set(mockPolicy);

      expect(component.hasRestrictions()).toBe(true);
    });

    it('should detect disabled features', () => {
      component.policy.set({
        ...mockPolicy,
        blockedCategories: [],
        disabledFeatures: ['messaging'],
      });

      expect(component.hasRestrictions()).toBe(true);
    });

    it('should detect disabled routes', () => {
      component.policy.set({
        ...mockPolicy,
        blockedCategories: [],
        disabledFeatures: [],
        disabledRoutes: ['/admin'],
      });

      expect(component.hasRestrictions()).toBe(true);
    });

    it('should detect session time limits', () => {
      component.policy.set({
        ...mockPolicy,
        blockedCategories: [],
        disabledFeatures: [],
        disabledRoutes: [],
        sessionMaxMinutes: 60,
      });

      expect(component.hasRestrictions()).toBe(true);
    });

    it('should detect daily time limits', () => {
      component.policy.set({
        ...mockPolicy,
        blockedCategories: [],
        disabledFeatures: [],
        disabledRoutes: [],
        sessionMaxMinutes: undefined,
        dailyMaxMinutes: 120,
      });

      expect(component.hasRestrictions()).toBe(true);
    });

    it('should detect age rating limits', () => {
      component.policy.set({
        ...mockPolicy,
        blockedCategories: [],
        disabledFeatures: [],
        disabledRoutes: [],
        sessionMaxMinutes: undefined,
        dailyMaxMinutes: undefined,
        ageRatingMax: 'PG-13',
      });

      expect(component.hasRestrictions()).toBe(true);
    });

    it('should detect reach level limits', () => {
      component.policy.set({
        ...mockPolicy,
        blockedCategories: [],
        disabledFeatures: [],
        disabledRoutes: [],
        sessionMaxMinutes: undefined,
        dailyMaxMinutes: undefined,
        ageRatingMax: undefined,
        reachLevelMax: 5,
      });

      expect(component.hasRestrictions()).toBe(true);
    });

    it('should return false when no restrictions', () => {
      component.policy.set({
        ...mockPolicy,
        blockedCategories: [],
        disabledFeatures: [],
        disabledRoutes: [],
        sessionMaxMinutes: undefined,
        dailyMaxMinutes: undefined,
        ageRatingMax: undefined,
        reachLevelMax: undefined,
      });

      expect(component.hasRestrictions()).toBe(false);
    });

    it('should return false when no policy', () => {
      component.policy.set(null);

      expect(component.hasRestrictions()).toBe(false);
    });
  });

  describe('Time Display', () => {
    it('should display session remaining time', () => {
      component.timeAccess.set(mockTimeAccess);

      expect(component.sessionRemaining()).toBe('45m');
    });

    it('should display daily remaining time', () => {
      component.timeAccess.set(mockTimeAccess);

      expect(component.dailyRemaining()).toBe('1h 30m');
    });

    it('should return null when time access not allowed', () => {
      component.timeAccess.set({
        status: 'session_limit',
      });

      expect(component.sessionRemaining()).toBeNull();
      expect(component.dailyRemaining()).toBeNull();
    });

    it('should return null when remaining time undefined', () => {
      component.timeAccess.set({
        status: 'allowed',
      });

      expect(component.sessionRemaining()).toBeNull();
      expect(component.dailyRemaining()).toBeNull();
    });
  });

  describe('Time Status', () => {
    it('should show ok status when allowed', () => {
      component.timeAccess.set({ status: 'allowed' });

      const status = component.timeStatus();

      expect(status?.status).toBe('ok');
      expect(status?.message).toContain('allowed');
    });

    it('should show blocked status when outside window', () => {
      component.timeAccess.set({ status: 'outside_window' });

      const status = component.timeStatus();

      expect(status?.status).toBe('blocked');
      expect(status?.message).toContain('Outside allowed time window');
    });

    it('should show blocked status when session limit reached', () => {
      component.timeAccess.set({ status: 'session_limit' });

      const status = component.timeStatus();

      expect(status?.status).toBe('blocked');
      expect(status?.message).toContain('Session time limit');
    });

    it('should show blocked status when daily limit reached', () => {
      component.timeAccess.set({ status: 'daily_limit' });

      const status = component.timeStatus();

      expect(status?.status).toBe('blocked');
      expect(status?.message).toContain('Daily time limit');
    });

    it('should return null when no time access', () => {
      component.timeAccess.set(null);

      expect(component.timeStatus()).toBeNull();
    });
  });

  describe('Restrictions List', () => {
    it('should list blocked categories', () => {
      component.policy.set(mockPolicy);

      const restrictions = component.restrictions();
      const categoryRestrictions = restrictions.filter(r => r.type === 'category');

      expect(categoryRestrictions.length).toBe(2);
      expect(categoryRestrictions[0].label).toContain('Violence');
      expect(categoryRestrictions[1].label).toContain('Adult');
    });

    it('should list disabled features', () => {
      component.policy.set(mockPolicy);

      const restrictions = component.restrictions();
      const featureRestrictions = restrictions.filter(r => r.type === 'feature');

      expect(featureRestrictions.length).toBe(1);
      expect(featureRestrictions[0].label).toContain('Messaging');
    });

    it('should list disabled routes', () => {
      component.policy.set(mockPolicy);

      const restrictions = component.restrictions();
      const routeRestrictions = restrictions.filter(r => r.type === 'route');

      expect(routeRestrictions.length).toBe(1);
      expect(routeRestrictions[0].label).toContain('/admin');
    });

    it('should list session time limit', () => {
      component.policy.set(mockPolicy);

      const restrictions = component.restrictions();
      const timeRestrictions = restrictions.filter(r => r.type === 'time' && r.label.includes('Session'));

      expect(timeRestrictions.length).toBe(1);
      expect(timeRestrictions[0].label).toContain('60 minutes');
    });

    it('should list daily time limit', () => {
      component.policy.set(mockPolicy);

      const restrictions = component.restrictions();
      const timeRestrictions = restrictions.filter(r => r.type === 'time' && r.label.includes('Daily'));

      expect(timeRestrictions.length).toBe(1);
      expect(timeRestrictions[0].label).toContain('120 minutes');
    });

    it('should list age rating limit', () => {
      component.policy.set(mockPolicy);

      const restrictions = component.restrictions();
      const ageRestrictions = restrictions.filter(r => r.type === 'age_rating');

      expect(ageRestrictions.length).toBe(1);
      expect(ageRestrictions[0].label).toContain('PG-13');
    });

    it('should list reach level limit', () => {
      component.policy.set(mockPolicy);

      const restrictions = component.restrictions();
      const reachRestrictions = restrictions.filter(r => r.type === 'reach');

      expect(reachRestrictions.length).toBe(1);
      expect(reachRestrictions[0].label).toContain('5');
    });

    it('should mark all restrictions as appealable', () => {
      component.policy.set(mockPolicy);

      const restrictions = component.restrictions();

      expect(restrictions.every(r => r.canAppeal)).toBe(true);
    });

    it('should return empty array when no policy', () => {
      component.policy.set(null);

      expect(component.restrictions()).toEqual([]);
    });
  });

  describe('Primary Steward', () => {
    it('should identify first active steward', () => {
      component.stewards.set([
        { ...mockGrant, status: 'active' },
        { ...mockGrant, id: 'grant-2', status: 'active' },
        { ...mockGrant, id: 'grant-3', status: 'active' },
      ]);

      const primary = component.primarySteward();

      expect(primary?.id).toBe('grant-1'); // First active grant
    });

    it('should return null when no active stewards', () => {
      component.stewards.set([
        { ...mockGrant, status: 'expired' },
        { ...mockGrant, id: 'grant-2', status: 'expired' },
      ]);

      expect(component.primarySteward()).toBeNull();
    });

    it('should return null when no stewards', () => {
      component.stewards.set([]);

      expect(component.primarySteward()).toBeNull();
    });
  });

  describe('Actions', () => {
    it('should handle file appeal action', () => {
      const restriction = {
        type: 'category' as const,
        label: 'Violence content blocked',
        description: 'Test',
        canAppeal: true,
      };

      component.fileAppeal(restriction);

      // Should not throw - implementation pending
    });

    it('should handle contact steward action', () => {
      component.stewards.set([mockGrant]);

      component.contactSteward();

      // Should not throw - implementation pending
    });

    it('should not attempt contact when no steward', () => {
      component.stewards.set([]);

      component.contactSteward();

      // Should not throw
    });
  });

  describe('Refresh Functionality', () => {
    it('should reload data on refresh', () => {
      spyOn(component, 'loadData');

      component.refresh();

      expect(component.loadData).toHaveBeenCalled();
    });
  });

  describe('Error Handling', () => {
    it('should clear error message', () => {
      component.error.set('Test error');

      component.clearError();

      expect(component.error()).toBeNull();
    });
  });

  describe('Helper Methods - Steward Labels', () => {
    it('should get steward tier label', () => {
      const label = component.getStewardTierLabel('guardian');

      expect(label).toBeDefined();
      expect(typeof label).toBe('string');
    });

    it('should get authority basis label', () => {
      const label = component.getAuthorityBasisLabel('minor_guardianship');

      expect(label).toBeDefined();
      expect(typeof label).toBe('string');
    });
  });

  describe('Edge Cases', () => {
    it('should handle zero minutes formatting', () => {
      expect(component.formatMinutes(0)).toBe('0m');
    });

    it('should handle large minute values', () => {
      expect(component.formatMinutes(150)).toBe('2h 30m');
      expect(component.formatMinutes(300)).toBe('5h');
    });

    it('should handle single character categories', () => {
      expect(component.formatCategory('x')).toBe('X');
    });

    it('should handle empty string category', () => {
      expect(component.formatCategory('')).toBe('');
    });

    it('should handle mixed case in categories', () => {
      expect(component.formatCategory('ADULT_CONTENT')).toBe('ADULT CONTENT');
    });
  });

  describe('Integration - Multiple Restrictions', () => {
    it('should handle policy with all restriction types', () => {
      const complexPolicy: ComputedPolicy = {
        subjectId: 'subject-456',
        computedAt: '2024-01-01T00:00:00Z',
        blockedCategories: ['violence', 'adult', 'gambling'],
        blockedHashes: ['hash1', 'hash2'],
        disabledFeatures: ['messaging', 'file_sharing', 'video_calls'],
        disabledRoutes: ['/admin', '/settings', '/marketplace'],
        requireApproval: ['external_links'],
        sessionMaxMinutes: 45,
        dailyMaxMinutes: 180,
        timeWindows: [],
        ageRatingMax: 'PG',
        reachLevelMax: 3,
        cooldownMinutes: 15,
        logSessions: true,
        logCategories: true,
        logPolicyEvents: true,
        retentionDays: 90,
        subjectCanView: true,
      };
      component.policy.set(complexPolicy);

      const restrictions = component.restrictions();

      expect(restrictions.length).toBeGreaterThan(10);
      expect(restrictions.filter(r => r.type === 'category').length).toBe(3);
      expect(restrictions.filter(r => r.type === 'feature').length).toBe(3);
      expect(restrictions.filter(r => r.type === 'route').length).toBe(3);
      expect(restrictions.filter(r => r.type === 'time').length).toBe(2);
      expect(restrictions.filter(r => r.type === 'age_rating').length).toBe(1);
      expect(restrictions.filter(r => r.type === 'reach').length).toBe(1);
    });
  });

  describe('Time Access Refresh', () => {
    it('should update time access silently', async () => {
      const newTimeAccess: TimeAccessDecision = {
        status: 'allowed',
        remainingSession: 20,
        remainingDaily: 40,
      };
      mockStewardshipService.checkTimeAccess.and.returnValue(Promise.resolve(newTimeAccess));

      await component.refreshTimeAccess();

      expect(component.timeAccess()).toEqual(newTimeAccess);
      expect(component.error()).toBeNull();
    });
  });
});
